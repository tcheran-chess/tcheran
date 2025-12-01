//! Implementation of the Universal Chess Interface (UCI) protocol

mod bench;
pub mod commands;
mod r#move;
mod options;
pub mod parser;
pub mod responses;

use std::{
    io::{BufRead, IsTerminal},
    num::NonZero,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub use r#move::UciMove;

use self::{
    commands::{GoCmdArguments, UciCommand},
    responses::{IdParam, InfoFields, InfoScore, UciResponse},
};
use crate::{
    ENGINE_NAME,
    chess::{
        bitboard::Bitboard,
        game::Game,
        moves::{Move, MoveListExt},
        perft,
        piece::PieceKind,
        player::Player,
        san,
        square::{File, Rank, Square},
    },
    engine::{
        eval::{WhiteEval, nnue::NNUE},
        options::EngineOptions,
        search,
        search::{Clocks, PersistentState, Reporter, TimeControl, time_control::StopControl},
        uci::{bench::bench, commands::DebugCommand, options::UciOption},
        util,
        util::sync::LockLatch,
    },
};

#[derive(Clone)]
pub struct UciReporter {
    pub pretty_output: bool,
}

mod colors {
    pub const BRIGHT_BLACK: &str = if cfg!(unix) { "\x1B[90m" } else { "" };
    pub const BRIGHT_WHITE: &str = if cfg!(unix) { "\x1B[97m" } else { "" };
    pub const RED: &str = if cfg!(unix) { "\x1B[31m" } else { "" };
    pub const WHITE: &str = if cfg!(unix) { "\x1B[37m" } else { "" };
    pub const GREEN: &str = if cfg!(unix) { "\x1B[32m" } else { "" };
    pub const RESET: &str = if cfg!(unix) { "\x1B[0m" } else { "" };
}

impl UciReporter {
    fn uci_report_search_progress(progress: &search::SearchInfo) {
        let score = if let Some(nmoves) = progress.eval.is_mate_in_moves() {
            InfoScore::Mate(nmoves)
        } else {
            InfoScore::Centipawns(progress.eval.0)
        };

        send_response(&UciResponse::Info(InfoFields {
            depth: Some(progress.depth),
            seldepth: Some(progress.seldepth),
            score: Some(score),
            pv: Some(
                progress
                    .pv
                    .iter()
                    .copied()
                    .map(std::convert::Into::into)
                    .collect(),
            ),
            time: Some(progress.stats.time),
            nodes: Some(progress.stats.nodes),
            nps: Some(progress.stats.nodes_per_second),
            tbhits: Some(progress.stats.tbhits),
            hashfull: Some(progress.hashfull),
            ..Default::default()
        }));
    }

    // Inspired by Simbelmyne's lovely search output
    #[expect(clippy::cast_precision_loss, reason = "Various approximate calculations")]
    fn pretty_report_search_progress(game: &Game, progress: &search::SearchInfo) {
        use colors::*;

        let score = if let Some(nmoves) = progress.eval.is_mate_in_moves() {
            InfoScore::Mate(nmoves)
        } else {
            InfoScore::Centipawns(progress.eval.0)
        };

        let mut game = game.clone();

        print!(" {:>3}", progress.depth);
        print!("{BRIGHT_BLACK}/{:<3}{RESET}", progress.seldepth);

        let (score, score_color) = match score {
            InfoScore::Centipawns(cp) => {
                let friendly_score = format!("{:+.2}", f64::from(cp) / 100.0);

                let color = match cp {
                    i32::MIN..=-11 => RED,
                    -10..=10 => WHITE,
                    11..=i32::MAX => GREEN,
                };

                (friendly_score, color)
            }
            InfoScore::Mate(plies) => {
                let friendly_mate = format!("M{}", plies.abs());
                let color = match plies {
                    i32::MIN..=-1 => RED,
                    1..=i32::MAX => GREEN,
                    0 => unreachable!(),
                };

                (friendly_mate, color)
            }
        };

        print!(" {score_color}{score:>7}{RESET}");

        let time = if progress.stats.time >= Duration::from_secs(1) {
            format!("{:.2}s", progress.stats.time.as_secs_f32())
        } else {
            format!("{}ms", progress.stats.time.as_millis())
        };

        print!("  {BRIGHT_BLACK}{time:>6}{RESET}",);

        let nodes = if progress.stats.nodes < 1000 {
            format!("{}n", progress.stats.nodes)
        } else {
            format!("{:.0}kn", progress.stats.nodes as f64 / 1000.0)
        };

        print!(" {BRIGHT_BLACK}{nodes:>10}{RESET}",);

        print!(
            "  {BRIGHT_BLACK}{:>10}{RESET}",
            format!("{:.0}knps", progress.stats.nodes_per_second as f64 / 1000.0)
        );

        print!("  {BRIGHT_BLACK}{:>4}{RESET}", format!("{:.0}%", progress.hashfull as f64 / 10.0));

        print!("  ");
        for mv in progress.pv.iter() {
            let san_mv = san::format_move(&game, *mv);

            print!(
                " {}",
                match game.player {
                    Player::White => format!("{BRIGHT_WHITE}{san_mv}{RESET}"),
                    Player::Black => format!("{BRIGHT_BLACK}{san_mv}{RESET}"),
                }
            );

            game.make_move(*mv);
        }

        println!();
    }

    fn uci_best_move(mv: Move) {
        send_response(&UciResponse::BestMove {
            mv: mv.into(),
            ponder: None,
        });
    }

    fn pretty_best_move(game: &Game, mv: Move) {
        println!("bestmove {}", san::format_move(game, mv));
    }
}

impl Reporter for UciReporter {
    fn generic_report(&self, s: &str) {
        println!("{s}");
    }

    fn report_search_progress(&self, game: &Game, progress: search::SearchInfo) {
        if self.pretty_output {
            Self::pretty_report_search_progress(game, &progress);
        } else {
            Self::uci_report_search_progress(&progress);
        }
    }

    fn best_move(&self, game: &Game, mv: Move) {
        if self.pretty_output {
            Self::pretty_best_move(game, mv);
        } else {
            Self::uci_best_move(mv);
        }
    }
}

pub struct Uci {
    control: Option<StopControl>,
    is_stopped: Arc<LockLatch>,
    reporter: UciReporter,
    debug: bool,
    game: Game,
    engine_options: EngineOptions,

    options: Vec<UciOption>,

    persistent_state: Arc<Mutex<PersistentState>>,

    // If we're running without using stdin (i.e. passing the UCI commands as command line
    // args) then we need to block on anything taking place on other threads, otherwise we'll
    // exit immediately as the search takes place on another thread.
    block_on_threads: bool,
}

impl Uci {
    fn execute(&mut self, cmd: &UciCommand) -> Result<ExecuteResult, String> {
        match cmd {
            UciCommand::Uci => {
                self.game = Game::new();

                let version = crate::engine_version();
                send_response(&UciResponse::Id(IdParam::Name(format!("{ENGINE_NAME} {version}"))));
                send_response(&UciResponse::Id(IdParam::Author("Jonathan Gilchrist")));

                // Options
                for option in &self.options {
                    send_response(&UciResponse::Option(option));
                }

                send_response(&UciResponse::UciOk);
            }
            UciCommand::Debug(on) => {
                self.debug = *on;
            }
            UciCommand::IsReady => send_response(&UciResponse::ReadyOk),
            UciCommand::SetOption { name, value } => {
                let Some(option) = self.options.iter().find(|o| o.name == name) else {
                    return Err("Invalid option".into());
                };

                let Ok(mut state_handle) = self.persistent_state.try_lock() else {
                    self.reporter
                        .generic_report("Unable to set options during search");
                    return Ok(ExecuteResult::KeepGoing);
                };

                option.set(value, &mut self.engine_options, &mut state_handle)?;
            }
            UciCommand::UciNewGame => {
                self.game = Game::new();
                self.is_stopped.reset();

                let mut persistent_state_handle = self.persistent_state.lock().unwrap();
                persistent_state_handle.reset();
            }
            UciCommand::Position { position, moves } => {
                let mut game = match position {
                    commands::Position::StartPos => Game::new(),
                    commands::Position::Fen(fen) => {
                        Game::from_fen(fen).map_err(|e| e.to_string())?
                    }
                };

                for mv in moves {
                    let matching_move = game.moves().expect_matching(mv.src, mv.dst, mv.promotion);
                    game.make_move(matching_move);
                }

                self.game = game;
            }
            UciCommand::Go(GoCmdArguments {
                ponder: _,
                wtime,
                btime,
                winc,
                binc,
                movestogo,
                depth,
                nodes,
                movetime,
                infinite: _,
            }) => {
                let game = self.game.clone();
                let options = self.engine_options.clone();
                let reporter = self.reporter.clone();

                let clocks = Clocks {
                    white_clock: *wtime,
                    black_clock: *btime,
                    white_increment: *winc,
                    black_increment: *binc,
                    moves_to_go: *movestogo,
                };

                let mut time_control = TimeControl::Infinite;

                if let Some(move_time) = movetime {
                    time_control = TimeControl::ExactTime(*move_time);
                }

                if wtime.is_some() || btime.is_some() {
                    time_control = TimeControl::Clocks(clocks);
                }

                if let Some(d) = depth {
                    time_control = TimeControl::Depth(*d);
                }

                if let Some(n) = nodes {
                    time_control = TimeControl::Nodes(*n);
                }

                self.control = Some(StopControl::new());

                let persistent_state = self.persistent_state.clone();
                let control = self.control.clone();
                let is_stopped = self.is_stopped.clone();

                let join_handle = std::thread::spawn(move || {
                    let mut persistent_state_handle = persistent_state.lock().unwrap();

                    let best_move = search::search(
                        &game,
                        &mut persistent_state_handle,
                        time_control,
                        control,
                        &options,
                        &reporter,
                    );

                    reporter.best_move(&game, best_move);
                    is_stopped.set();
                });

                if self.block_on_threads {
                    join_handle.join().unwrap();
                }
            }
            UciCommand::Stop => {
                if let Some(c) = self.control.as_mut() {
                    c.stop();
                    self.is_stopped.wait();
                }

                self.control = None;
            }
            UciCommand::D(debug_cmd) => match debug_cmd {
                DebugCommand::PrintPosition => {
                    println!("{:?}", self.game.board);
                    println!("FEN: {}", self.game.to_fen());
                    println!();
                }
                DebugCommand::SetPosition { position } => match position.as_str() {
                    "kiwipete" => {
                        self.game = Game::from_fen(
                            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -",
                        )
                        .unwrap();

                        println!("{:?}", self.game.board);
                    }
                    _ => return Err("Unknown debug position".to_owned()),
                },
                DebugCommand::Move { moves } => {
                    for mv in moves {
                        let matching_move =
                            self.game
                                .moves()
                                .expect_matching(mv.src, mv.dst, mv.promotion);

                        self.game.make_move(matching_move);
                    }

                    println!("{:?}", self.game.board);
                    println!("FEN: {}", crate::chess::fen::write(&self.game));
                    println!();
                }
                DebugCommand::Perft { depth } => {
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
                DebugCommand::PerftDiv { depth } => {
                    let result = perft::perft_div(*depth, &mut self.game);
                    let mut total = 0;

                    for (mv, number_for_mv) in result {
                        println!("{mv:?}: {number_for_mv}");
                        total += number_for_mv;
                    }

                    println!("total: {total}");
                    println!();
                }
                DebugCommand::Eval => {
                    let mut nnue = NNUE::from_board(&self.game.board);

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
                                    print!("{: ^7}", piece_contributions[sq].to_string());
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

                    println!();
                    println!(
                        "Evaluation: {}",
                        nnue.evaluate(Player::White).to_white_eval(Player::White)
                    );
                    println!();
                }
            },
            UciCommand::PonderHit => {}
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
            UciCommand::Quit => return Ok(ExecuteResult::Exit),
        }

        Ok(ExecuteResult::KeepGoing)
    }

    fn run_line(&mut self, line: &str) -> Result<bool, String> {
        let command = parser::parse(line);

        match command {
            Ok(ref c) => {
                let execute_result = self.execute(c)?;

                if execute_result == ExecuteResult::Exit {
                    return Ok(false);
                }
            }
            Err(()) => {
                eprintln!("Invalid command");
            }
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

fn send_response(response: &UciResponse<'_>) {
    println!("{response}");
}

pub enum UciInputMode {
    #[allow(clippy::allow_attributes, reason = "Lint only present in non-release mode")]
    #[allow(
        unused,
        reason = "Passing a  list of UCI commands is not currently implemented for the CLI"
    )]
    Commands(Vec<String>),
    Stdin,
}

pub fn uci_options() -> Vec<UciOption> {
    vec![
        UciOption::spin("Hash", |options, state, value| {
            options.hash_size =
                usize::try_from(value).expect("min: 0 should prevent us getting negative values");
            state.tt.resize(options.hash_size);
        })
        .default(crate::engine::options::defaults::HASH_SIZE)
        .with_bounds(0, 1024)
        .build(),
        //
        UciOption::spin("Threads", |options, _state, value| {
            options.threads =
                usize::try_from(value).expect("min: 0 should prevent us getting negative values");
        })
        .default(crate::engine::options::defaults::THREADS)
        .with_bounds(
            1,
            std::thread::available_parallelism()
                .unwrap_or(NonZero::new(1).unwrap())
                .get(),
        )
        .build(),
        //
        UciOption::spin("Move Overhead", |options, _state, value| {
            options.move_overhead =
                usize::try_from(value).expect("min: 0 should prevent us getting negative values");
        })
        .default(crate::engine::options::defaults::MOVE_OVERHEAD)
        .with_bounds(0, 1000)
        .build(),
        //
        UciOption::string("SyzygyPath", |_options, state, value| {
            state.tablebase.set_paths(&value);
        })
        .default(String::new())
        .build(),
    ]
}

pub fn uci(uci_input_mode: UciInputMode) -> Result<(), String> {
    let mut uci = Uci {
        control: None,
        is_stopped: Arc::new(LockLatch::new()),
        reporter: UciReporter {
            pretty_output: std::io::stdin().is_terminal(),
        },
        debug: false,
        persistent_state: Arc::new(Mutex::new(PersistentState::new(
            EngineOptions::DEFAULT.hash_size,
        ))),

        game: Game::new(),
        engine_options: EngineOptions::DEFAULT,

        options: uci_options(),

        block_on_threads: match uci_input_mode {
            UciInputMode::Stdin => false,
            UciInputMode::Commands(_) => true,
        },
    };

    uci.main_loop(uci_input_mode)
}
