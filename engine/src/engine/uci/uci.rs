//! Implementation of the Universal Chess Interface (UCI) protocol

use std::{
    io::{BufRead, IsTerminal},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    thread,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::{
    ENGINE_NAME, ENGINE_VERSION,
    chess::{notations::san, perft, prelude::*},
    engine::{
        eval::{
            WhiteEval,
            nnue::{AccumulatorCache, NNUE},
            wdl,
        },
        options::{EngineOptions, defaults},
        search::{
            NullReporter, PersistentState, Reporter, ThreadData, TimeControl, probe_tb_at_root,
            search, time_control::StopControl, types::SearchResults,
        },
        uci::{
            bench::bench,
            commands,
            commands::UciCommand,
            options::UciOption,
            parser,
            responses::{IdParam, UciReporter, UciResponse},
        },
        util,
        util::{log, speedtest::speedtest},
    },
};

pub struct Threads {
    threads: Vec<Thread>,
    thread_control: StopControl,
}

impl Threads {
    pub fn new() -> Self {
        let mut ts = Self {
            threads: vec![],
            thread_control: StopControl::new(0),
        };

        ts.scale(1);
        ts
    }

    pub fn scale(&mut self, n: usize) {
        // If we just started, we may be scaling from 0 threads in which case we don't need to quit
        // our existing threads.
        if !self.threads.is_empty() {
            self.threads.drain(..).for_each(Thread::join);
        }

        self.threads = (0..n).map(Thread::new).collect();
        self.thread_control = StopControl::new(0);

        self.send(ThreadCommand::Ping);
    }

    #[expect(clippy::needless_pass_by_value, reason = "Must be cloned for each tx")]
    pub fn send(&self, cmd: ThreadCommand) {
        for thread in &self.threads {
            thread.send(cmd.clone());
        }
    }

    pub fn wait(&self) {
        self.thread_control.wait_until_finished();
    }

    pub fn busy(&self) -> bool {
        self.thread_control.is_busy()
    }

    pub fn reset(&mut self) {
        self.send(ThreadCommand::Reset);
    }

    pub fn stop_and_wait(&self) {
        assert!(self.thread_control.is_busy());

        // Tell the threads to stop
        self.thread_control.stop();

        // Wait until all threads have stopped
        self.thread_control.wait_until_finished();

        // Reset the stop flag for the next search
        self.thread_control.reset();
    }
}

pub struct Thread {
    handle: JoinHandle<()>,
    tx: SyncSender<ThreadCommand>,
}

impl Thread {
    pub fn new(id: usize) -> Self {
        let (tx, rx) = sync_channel(0);

        let handle = thread::spawn({
            move || {
                if std::panic::catch_unwind(move || {
                    worker_thread_loop(&rx, id);
                })
                .is_err()
                {
                    std::process::exit(-1);
                }
            }
        });

        Self { handle, tx }
    }

    pub fn send(&self, cmd: ThreadCommand) {
        self.tx.send(cmd).expect("Unable to send thread command");
    }

    pub fn join(self) {
        drop(self.tx);
        self.handle.join().expect("Unable to join thread");
    }
}

#[derive(Clone)]
#[expect(clippy::large_enum_variant, reason = "No issues with these being large")]
pub enum ThreadCommand {
    Search {
        game: Game,
        time_control: TimeControl,
        stop_control: StopControl,
        options: EngineOptions,
        persistent_state: Arc<PersistentState>,
        reporter: Arc<dyn Reporter + Send + Sync>,
        results: Arc<SearchResults>,
    },
    Ping,
    Reset,
}

pub struct Uci {
    game: Game,
    threads: Threads,
    persistent_state: Arc<PersistentState>,
    reporter: Arc<UciReporter>,

    uci_options: Vec<UciOption>,
    options: EngineOptions,

    // If we're running without using stdin (i.e. passing the UCI commands as command line
    // args) then we need to block on anything taking place on other threads, otherwise we'll
    // exit immediately as the search takes place on another thread.
    block_on_threads: bool,
    debug: bool,
}

impl Uci {
    fn execute(&mut self, cmd: &UciCommand) -> Result<ExecuteResult, String> {
        match cmd {
            UciCommand::Uci => {
                self.reporter.pretty_output.store(false, Ordering::Relaxed);

                self.reporter.send(&UciResponse::Id(IdParam::Name(format!(
                    "{ENGINE_NAME} {ENGINE_VERSION}"
                ))));
                self.reporter
                    .send(&UciResponse::Id(IdParam::Author("Jonathan Gilchrist")));

                // Options
                for option in &self.uci_options {
                    self.reporter.send(&UciResponse::Option(option));
                }

                self.reporter.send(&UciResponse::UciOk);
            }
            UciCommand::Debug(on) => {
                self.debug = *on;
            }
            UciCommand::IsReady => self.reporter.send(&UciResponse::ReadyOk),
            UciCommand::SetOption { name, value } => {
                if self.threads.busy() {
                    self.reporter
                        .generic_report("cannot set options while searching");
                    return Ok(ExecuteResult::KeepGoing);
                }

                let Some(option) = self.uci_options.iter().find(|o| o.name == name) else {
                    let unknown_option = format!("unknown option: {name}");
                    log::crashlog(&unknown_option);
                    self.reporter.generic_report(&unknown_option);

                    return Ok(ExecuteResult::KeepGoing);
                };

                option.set(
                    value,
                    &mut self.game,
                    &mut self.threads,
                    &mut self.persistent_state,
                    &mut self.options,
                    &mut self.reporter,
                )?;
            }
            UciCommand::UciNewGame => {
                if self.threads.busy() {
                    self.reporter
                        .generic_report("cannot start new game while searching");
                    return Ok(ExecuteResult::KeepGoing);
                }

                self.game = Game::new();
                self.game.is_frc = self.options.frc;

                self.threads.reset();

                Arc::get_mut(&mut self.persistent_state)
                    .expect("Unable to get unique access to state")
                    .reset(&self.options);
            }
            UciCommand::Position { position, moves } => {
                if self.threads.busy() {
                    self.reporter
                        .generic_report("cannot set position while searching");
                    return Ok(ExecuteResult::KeepGoing);
                }

                let mut game = match position {
                    commands::Position::StartPos => Game::new(),
                    commands::Position::Fen(fen) => {
                        if !self.options.frc {
                            Game::from_fen(fen)
                        } else {
                            Game::from_frc_fen(fen)
                        }
                    }
                    .map_err(|e| e.to_string())?,
                };

                for uci_mv in moves {
                    let mv = uci_mv.find_in_game(&game);
                    game.make_move(mv);
                }

                self.game = game;
                self.game.is_frc = self.options.frc;
            }
            UciCommand::Go { time_control } => {
                if self.threads.busy() {
                    self.reporter.generic_report("already searching");
                    return Ok(ExecuteResult::KeepGoing);
                }

                if self.persistent_state.tablebase.can_probe(&self.game)
                    && let Some(tb_result) =
                        probe_tb_at_root(&self.game, &self.persistent_state.tablebase, time_control)
                {
                    self.reporter.report_search_progress(&self.game, &tb_result);
                    self.reporter.best_move(&self.game, tb_result.mv);
                    return Ok(ExecuteResult::KeepGoing);
                }

                self.threads
                    .thread_control
                    .start_search(self.options.threads as u32);

                self.persistent_state.new_search();

                let game = self.game.clone();
                let persistent_state = self.persistent_state.clone();
                let options = self.options.clone();
                let reporter = self.reporter.clone();
                let mut time_control = time_control.clone();
                let stop_control = self.threads.thread_control.clone();
                let results = Arc::new(SearchResults::new(self.options.threads));

                // Adapt time control for soft nodes if the corresponding option is set
                if options.soft_nodes
                    && let TimeControl::Nodes { hard, .. } = time_control
                {
                    let nodes = hard.expect("Hard nodes should be set after parsing go nodes");

                    time_control = TimeControl::Nodes {
                        soft: Some(nodes),
                        hard: options.soft_notes_hard_factor.map(|f| f as u64 * nodes),
                    }
                }

                self.threads.send(ThreadCommand::Search {
                    game,
                    time_control,
                    stop_control,
                    options,
                    persistent_state,
                    reporter,
                    results,
                });

                if self.block_on_threads {
                    self.threads.wait();
                }
            }
            UciCommand::Stop => {
                if !self.threads.busy() {
                    self.reporter.generic_report("no search to stop");
                    return Ok(ExecuteResult::KeepGoing);
                }

                self.threads.stop_and_wait();
            }
            UciCommand::PrintPosition => {
                println!("{:?}", self.game.board);
                println!("FEN: {}", self.game.to_fen());
                println!();
            }
            UciCommand::Move { moves } => {
                if self.threads.busy() {
                    self.reporter.generic_report("cannot move while searching");
                    return Ok(ExecuteResult::KeepGoing);
                }

                let mut validated_moves = vec![];

                // Validate that the moves play out correctly
                {
                    let mut game = self.game.clone();

                    for mv in moves {
                        let mut parsed_move: Option<Move> = None;

                        if let Ok(san_move) = san::parse_move(&game, mv) {
                            parsed_move = Some(san_move);
                        }

                        if let Ok(uci_move) = parser::uci_move(mv) {
                            let uci_move = game
                                .moves()
                                .into_iter()
                                .find(|m| {
                                    m.from() == uci_move.from
                                        && m.to() == uci_move.to
                                        && m.promotion() == uci_move.promotion
                                })
                                .copied();

                            parsed_move = uci_move;
                        }

                        let Some(parsed_move) = parsed_move else {
                            println!("Invalid or illegal move: {mv}");
                            return Ok(ExecuteResult::KeepGoing);
                        };

                        game.make_move(parsed_move);
                        validated_moves.push(parsed_move);
                    }
                }

                // If we reach this point, we've got a valid list of moves we can make, so make them all
                for mv in validated_moves {
                    self.game.make_move(mv);
                }

                println!("{:?}", self.game.board);
                println!("FEN: {}", crate::chess::notations::fen::write(&self.game));
                println!();
            }
            UciCommand::Perft { depth } => {
                let started_at = Instant::now();
                let result = perft::perft(*depth, &mut self.game);
                let time_taken = started_at.elapsed();

                let nodes_per_second =
                    util::metrics::nodes_per_second(u64::try_from(result).unwrap(), time_taken);

                println!("positions: {result}");
                println!("time taken: {time_taken:?}");
                println!("nps: {nodes_per_second:?}");
                println!();
            }
            UciCommand::PerftDiv { depth } => {
                let mut result = perft::perft_div(*depth, &mut self.game);
                result.sort_by_key(|r| format!("{:?}", r.0));

                let mut total = 0;

                for (mv, number_for_mv) in result {
                    println!("{mv:?}: {number_for_mv}");
                    total += number_for_mv;
                }

                println!("total: {total}");
                println!();
            }
            UciCommand::Eval => {
                let mut nnue = NNUE::default();
                let mut cache = AccumulatorCache::new();

                for player in Player::ALL {
                    nnue.refresh(&self.game.board, player, &mut cache);
                }

                let mut piece_contributions: [WhiteEval; Square::N] = [WhiteEval(0); Square::N];
                for sq in Bitboard::FULL {
                    let Some(piece) = self.game.board.piece_at(sq) else {
                        continue;
                    };
                    if piece.kind == PieceKind::King {
                        continue;
                    }

                    piece_contributions[sq] = nnue
                        .approx_contribution(&self.game.clone(), sq, Player::White)
                        .to_white_eval(Player::White);
                }

                println!("┌───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┐");

                for rank in Rank::ALL.iter().rev() {
                    print!("│");

                    for file in File::ALL {
                        let sq = Square::from_file_and_rank(file, *rank);
                        let piece = self.game.board.piece_at(sq);

                        match piece {
                            Some(piece) => print!("   {}   ", piece.char()),
                            None => print!("       "),
                        }

                        print!("│");
                    }

                    println!();

                    print!("│");

                    for file in File::ALL {
                        let sq = Square::from_file_and_rank(file, *rank);
                        let piece = self.game.board.piece_at(sq);

                        match piece {
                            Some(piece) if piece.kind != PieceKind::King => {
                                let normalized_contribution = wdl::normalize(
                                    piece_contributions[sq].for_player(Player::White),
                                    &self.game.board,
                                )
                                .to_white_eval(Player::White);

                                print!("{: ^7}", normalized_contribution.to_string());
                            }
                            _ => print!("       "),
                        }

                        print!("│");
                    }

                    println!();

                    if *rank == Rank::R1 {
                        println!(
                            "└───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┘"
                        );
                    } else {
                        println!(
                            "├───────┼───────┼───────┼───────┼───────┼───────┼───────┼───────┤"
                        );
                    }
                }

                let raw_eval = nnue.evaluate(Player::White, &self.game);
                let normalised_eval =
                    wdl::normalize(raw_eval, &self.game.board).to_white_eval(Player::White);

                println!();
                println!("Raw evaluation: {}", raw_eval.to_white_eval(Player::White));
                println!("Normalised evaluation: {normalised_eval}");
                println!();
            }
            // For OpenBench to understand NPS values for different workers
            UciCommand::Bench => {
                let (nodes, time_taken) = bench(None);
                let nps = util::metrics::nodes_per_second(nodes, time_taken);

                println!("{nodes} nodes {nps} nps");
            }
            UciCommand::BenchNodes => {
                let (nodes, _) = bench(None);
                println!("{nodes}");
            }
            UciCommand::Speedtest {
                threads,
                hash,
                duration,
            } => {
                speedtest(*threads, *hash, *duration);
            }

            #[cfg(not(feature = "datagen"))]
            UciCommand::GenFens { .. } => {
                log::crashlog("datagen feature is not enabled");

                self.reporter
                    .generic_report("datagen feature is not enabled");
            }
            #[cfg(feature = "datagen")]
            UciCommand::GenFens {
                n,
                seed,
                book,
                dfrc,
            } => {
                use crate::engine::util::datagen;

                let starting_positions =
                    datagen::generate_random_starting_positions(*n, *seed, book.to_owned(), *dfrc);
                for pos in starting_positions {
                    self.reporter
                        .generic_report(&format!("genfens {}", pos.to_fen()));
                }
            }
            #[cfg(not(feature = "spsa"))]
            UciCommand::Spsa => {
                self.reporter.generic_report("spsa feature is not enabled");
            }
            #[cfg(feature = "spsa")]
            UciCommand::Spsa => crate::engine::uci::spsa::print_spsa_input(),
            UciCommand::Quit => {
                if self.threads.busy() {
                    self.threads.stop_and_wait();
                }

                return Ok(ExecuteResult::Exit);
            }
        }

        Ok(ExecuteResult::KeepGoing)
    }

    fn run_line(&mut self, line: &str) -> Result<bool, String> {
        let command = parser::parse(line);

        let Ok(ref c) = command else {
            log::crashlog(format!("Invalid command: {line}"));
            eprintln!("Invalid command");
            return Ok(true);
        };

        let execute_result = self.execute(c)?;

        if execute_result == ExecuteResult::Exit {
            return Ok(false);
        }

        Ok(true)
    }

    fn main_loop_stdin(&mut self) -> Result<(), String> {
        let stdin_lines = std::io::stdin().lock().lines();

        for line in stdin_lines {
            let line = line.unwrap();
            let should_continue = self.run_line(&line).map_err(|e| format!("Error: {e}"))?;

            if !should_continue {
                break;
            }
        }

        Ok(())
    }

    fn main_loop_args(&mut self, lines: Vec<String>) -> Result<(), String> {
        for line in lines {
            let should_continue = self.run_line(&line)?;

            if !should_continue {
                break;
            }
        }

        Ok(())
    }

    fn main_loop(&mut self, uci_input_mode: UciInputMode) -> Result<(), String> {
        match uci_input_mode {
            UciInputMode::Stdin => self.main_loop_stdin(),
            UciInputMode::Commands(cmds) => self.main_loop_args(cmds),
        }
    }
}

#[derive(Debug, PartialEq)]
enum ExecuteResult {
    KeepGoing,
    Exit,
}

pub enum UciInputMode {
    Commands(Vec<String>),
    Stdin,
}

pub fn uci_options() -> Vec<UciOption> {
    let options = vec![
        UciOption::spin("Hash", |refs, value| {
            refs.options.hash_size = value.as_usize();
            refs.state
                .tt
                .resize(refs.options.hash_size, refs.options.threads);
        })
        .default(crate::engine::options::defaults::HASH_SIZE as i32)
        .with_bounds(0, 1024 * 1024)
        .build(),
        //
        UciOption::spin("Threads", |refs, value| {
            refs.options.threads = value.as_usize();
            refs.threads.scale(refs.options.threads);
        })
        .default(crate::engine::options::defaults::THREADS as i32)
        .with_bounds(1, 1024)
        .build(),
        //
        UciOption::check("Minimal", |refs, value| {
            refs.options.minimal = value;
        })
        .default(defaults::MINIMAL)
        .build(),
        //
        UciOption::check("UCI_Chess960", |refs, value| {
            refs.options.frc = value;
            refs.game.is_frc = value;
        })
        .default(false)
        .build(),
        //
        UciOption::check("UCI_ShowWDL", |refs, value| {
            refs.reporter.show_wdl = value;
        })
        .default(crate::engine::options::defaults::SHOW_WDL)
        .build(),
        //
        UciOption::spin("MoveOverhead", |refs, value| {
            refs.options.move_overhead = Duration::from_millis(value.as_u64());
        })
        .default(crate::engine::options::defaults::MOVE_OVERHEAD.as_millis() as i32)
        .with_bounds(0, 1000)
        .build(),
        //
        UciOption::string("SyzygyPath", |refs, value| {
            refs.state.tablebase.set_paths(&value);
        })
        .default(String::new())
        .build(),
        //
        UciOption::check("SoftNodes", |refs, value| refs.options.soft_nodes = value)
            .default(crate::engine::options::defaults::SOFT_NODES)
            .build(),
        //
        UciOption::spin("SoftNodesHardFactor", |refs, value| {
            let inner_value = value.as_usize();

            let mut value = Some(inner_value);
            if inner_value == 0 {
                value = None;
            }

            refs.options.soft_notes_hard_factor = value;
        })
        .with_bounds(0, 128)
        .default(0)
        .build(),
    ];

    let mut o = vec![];
    o.extend(options);
    #[cfg(feature = "spsa")]
    o.extend(crate::engine::params::spsa_params());
    o
}

fn worker_thread_loop(rx: &Receiver<ThreadCommand>, id: usize) {
    let mut thread_data = ThreadData::new(id);
    let is_main_thread = id == 0;

    while let Ok(command) = rx.recv() {
        match command {
            ThreadCommand::Search {
                game,
                time_control,
                stop_control,
                options,
                persistent_state,
                reporter,
                results,
            } => {
                thread_data.new_search(&game);

                // Only send messages from the main search thread
                let reporter = if is_main_thread {
                    reporter.clone()
                } else {
                    Arc::new(NullReporter)
                };

                // Only do time control on the main thread
                let time_control = if is_main_thread {
                    time_control
                } else {
                    TimeControl::Infinite
                };

                search(
                    &game,
                    &persistent_state,
                    &mut thread_data,
                    &results,
                    time_control,
                    &stop_control,
                    &options,
                    &*reporter,
                );
            }
            ThreadCommand::Ping => { /* pong */ }
            ThreadCommand::Reset => {
                thread_data.reset();
            }
        }
    }
}

pub fn uci(uci_input_mode: UciInputMode) -> Result<(), String> {
    let mut uci = Uci {
        game: Game::new(),
        threads: Threads::new(),
        persistent_state: Arc::new(PersistentState::new(EngineOptions::DEFAULT.hash_size)),
        reporter: Arc::new(UciReporter {
            pretty_output: AtomicBool::new(std::io::stdin().is_terminal()),
            show_wdl: defaults::SHOW_WDL,
        }),

        uci_options: uci_options(),
        options: EngineOptions::DEFAULT,

        block_on_threads: match uci_input_mode {
            UciInputMode::Stdin => false,
            UciInputMode::Commands(_) => true,
        },
        debug: false,
    };

    uci.main_loop(uci_input_mode)
}
