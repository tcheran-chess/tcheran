//! Implementation of the Universal Chess Interface (UCI) protocol

mod bench;
pub mod commands;
mod r#move;
pub mod options;
pub mod parser;
pub mod responses;
mod spsa;

use std::{
    io::{BufRead, IsTerminal},
    num::NonZero,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub use r#move::UciMove;

use self::{
    commands::UciCommand,
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
        eval::{WhiteEval, nnue::NNUE, wdl},
        options::EngineOptions,
        search,
        search::{PersistentState, Reporter, time_control::StopControl},
        uci::{bench::bench, options::UciOption, spsa::print_spsa_input},
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
        send_response(&UciResponse::Info(InfoFields {
            depth: Some(progress.depth),
            seldepth: Some(progress.seldepth),
            score: Some(InfoScore::from(progress.eval, &progress.game)),
            wdl: Some(wdl::wdl(progress.eval, &progress.game.board)),
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
            string: None,
        }));
    }

    // Inspired by Simbelmyne's lovely search output
    #[expect(clippy::cast_precision_loss, reason = "Various approximate calculations")]
    fn pretty_report_search_progress(game: &Game, progress: &search::SearchInfo) {
        use colors::*;

        let score = InfoScore::from(progress.eval, &progress.game);
        let mut game = game.clone();

        print!(" {:>3}", progress.depth);
        print!("{BRIGHT_BLACK}/{:<3}{RESET}", progress.seldepth);

        let (formatted_score, score_color) = match score {
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

        print!(" {score_color}{formatted_score:>7}{RESET}");

        #[expect(clippy::cast_possible_truncation, reason = "Approximate calculation")]
        let as_percentage = |n: f64| (100.0 * n).round() as i32;
        let wdl = wdl::wdl(progress.eval, &progress.game.board);
        let formatted_wdl = format!(
            "({}/{}/{})",
            as_percentage(wdl.win),
            as_percentage(wdl.draw),
            as_percentage(wdl.loss)
        );

        print!(" {BRIGHT_BLACK}{formatted_wdl:<10}{RESET}");

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
        send_response(&UciResponse::BestMove { mv: mv.into() });
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
            UciCommand::Go { time_control } => {
                let game = self.game.clone();
                let options = self.engine_options.clone();
                let reporter = self.reporter.clone();
                let time_control = time_control.clone();

                let stop_control = StopControl::new();
                self.control = Some(stop_control.clone());

                let persistent_state = self.persistent_state.clone();
                let is_stopped = self.is_stopped.clone();

                let join_handle = std::thread::spawn(move || {
                    let mut persistent_state_handle = persistent_state.lock().unwrap();

                    search::search(
                        &game,
                        &mut persistent_state_handle,
                        time_control,
                        stop_control,
                        &options,
                        &reporter,
                    );

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
            UciCommand::PrintPosition => {
                println!("{:?}", self.game.board);
                println!("FEN: {}", self.game.to_fen());
                println!();
            }
            UciCommand::Move { moves } => {
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
                                    m.src() == uci_move.src
                                        && m.dst() == uci_move.dst
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
                println!("FEN: {}", crate::chess::fen::write(&self.game));
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
                let result = perft::perft_div(*depth, &mut self.game);
                let mut total = 0;

                for (mv, number_for_mv) in result {
                    println!("{mv:?}: {number_for_mv}");
                    total += number_for_mv;
                }

                println!("total: {total}");
                println!();
            }
            UciCommand::Eval => {
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
            UciCommand::Spsa => print_spsa_input(),
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

#[expect(clippy::cast_possible_truncation, reason = "Default values are too small to be truncated")]
#[expect(clippy::cast_possible_wrap, reason = "Default values are too small to be wrapped")]
pub fn uci_options() -> Vec<UciOption> {
    let options = vec![
        UciOption::spin("Hash", |options, state, value| {
            options.hash_size = value.as_usize();
            state.tt.resize(options.hash_size);
        })
        .default(crate::engine::options::defaults::HASH_SIZE as i32)
        .with_bounds(0, 1024 * 1024)
        .build(),
        //
        UciOption::spin("Threads", |options, state, value| {
            options.threads = value.as_usize();
            state.scale_threads(options.threads);
        })
        .default(crate::engine::options::defaults::THREADS as i32)
        .with_bounds(
            1,
            std::thread::available_parallelism()
                .unwrap_or(NonZero::new(1).unwrap())
                .get() as i32,
        )
        .build(),
        //
        UciOption::spin("Move Overhead", |options, _state, value| {
            options.move_overhead = Duration::from_millis(value.as_u64());
        })
        .default(crate::engine::options::defaults::MOVE_OVERHEAD.as_millis() as i32)
        .with_bounds(0, 1000)
        .build(),
        //
        UciOption::string("SyzygyPath", |_options, state, value| {
            state.tablebase.set_paths(&value);
        })
        .default(String::new())
        .build(),
    ];

    let mut o = vec![];
    o.extend(options);
    #[cfg(feature = "spsa")]
    o.extend(crate::engine::params::spsa_params());
    o
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
