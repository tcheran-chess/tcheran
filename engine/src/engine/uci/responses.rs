use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use crate::{
    chess::{notations::san, prelude::*},
    engine::{
        eval::{Eval, wdl, wdl::WdlProbabilities},
        search,
        search::Reporter,
        uci::{
            UciMove,
            options::{UciOption, UciOptionType},
        },
        util::metrics,
    },
};

#[derive(Debug)]
pub(super) enum InfoScore {
    Centipawns(i32),
    Mate(i32),
}

impl InfoScore {
    const TB_DISPLAY_SCORE: i32 = 20000;

    pub fn from(eval: Eval, game: &Game) -> Self {
        if eval.is_decisive() {
            if eval.is_tb() {
                Self::Centipawns(if eval.is_win() {
                    Self::TB_DISPLAY_SCORE
                } else {
                    -Self::TB_DISPLAY_SCORE
                })
            } else {
                Self::Mate(eval.moves_to_mate())
            }
        } else {
            let normalized_eval = wdl::normalize(eval, &game.board);
            Self::Centipawns(normalized_eval.0)
        }
    }
}

#[derive(Debug)]
pub(super) enum IdParam {
    Name(String),
    Author(&'static str),
}

#[derive(Debug)]
pub struct InfoFields {
    pub(super) depth: Option<u8>,
    pub(super) seldepth: Option<u8>,
    pub(super) time: Option<Duration>,
    pub(super) nodes: Option<u64>,
    pub(super) pv: Option<Vec<UciMove>>,
    pub(super) score: Option<InfoScore>,
    pub(super) wdl: Option<WdlProbabilities>,
    pub(super) hashfull: Option<u64>,
    pub(super) nps: Option<u64>,
    pub(super) tbhits: Option<u64>,
    pub(super) string: Option<String>,
}

pub(super) enum UciResponse<'uci> {
    Id(IdParam),
    UciOk,
    ReadyOk,
    BestMove { mv: UciMove },
    Info(InfoFields),
    Option(&'uci UciOption),
}

pub struct UciReporter {
    pub pretty_output: AtomicBool,
    pub show_wdl: bool,
}

mod colors {
    pub const BRIGHT_BLACK: &str = if cfg!(unix) { "\x1B[90m" } else { "" };
    pub const BRIGHT_WHITE: &str = if cfg!(unix) { "\x1B[97m" } else { "" };
    pub const WHITE: &str = if cfg!(unix) { "\x1B[37m" } else { "" };
    pub const RED: &str = if cfg!(unix) { "\x1B[31m" } else { "" };
    pub const GREEN: &str = if cfg!(unix) { "\x1B[32m" } else { "" };
    pub const YELLOW: &str = if cfg!(unix) { "\x1B[33m" } else { "" };
    pub const BLUE: &str = if cfg!(unix) { "\x1B[34m" } else { "" };
    pub const RESET: &str = if cfg!(unix) { "\x1B[0m" } else { "" };
}

impl UciReporter {
    pub(super) fn send(&self, msg: &UciResponse<'_>) {
        match msg {
            UciResponse::Id(i) => match i {
                IdParam::Name(name) => print!("id name {name}"),
                IdParam::Author(author) => print!("id author {author}"),
            },
            UciResponse::UciOk => print!("uciok"),
            UciResponse::ReadyOk => print!("readyok"),
            UciResponse::BestMove { mv } => {
                print!("bestmove {}", mv.notation());
            }
            UciResponse::Info(InfoFields {
                depth,
                seldepth,
                time,
                nodes,
                pv,
                score,
                wdl,
                hashfull,
                nps,
                tbhits,
                string,
            }) => {
                print!("info");

                if let Some(depth) = depth {
                    print!(" depth {depth}");
                }

                if let Some(seldepth) = seldepth {
                    print!(" seldepth {seldepth}");
                }

                if let Some(score) = score {
                    match score {
                        InfoScore::Centipawns(centipawns) => {
                            print!(" score cp {centipawns}");
                        }
                        InfoScore::Mate(turns) => {
                            print!(" score mate {turns}");
                        }
                    }
                }

                if let Some(wdl) = wdl
                    && self.show_wdl
                {
                    let format_wdl = |n: f64| (1000.0 * n).round() as i32;
                    print!(
                        " wdl {} {} {}",
                        format_wdl(wdl.win),
                        format_wdl(wdl.draw),
                        format_wdl(wdl.loss)
                    );
                }

                if let Some(time) = time {
                    print!(" time {}", time.as_millis());
                }

                if let Some(nodes) = nodes {
                    print!(" nodes {nodes}");
                }

                if let Some(nps) = nps {
                    print!(" nps {nps}");
                }

                if let Some(hashfull) = hashfull {
                    print!(" hashfull {hashfull}");
                }

                if let Some(tbhits) = tbhits {
                    print!(" tbhits {tbhits}");
                }

                if let Some(pv) = pv {
                    print!(" pv");

                    for mv in pv {
                        print!(" {}", mv.notation());
                    }
                }

                if let Some(s) = string {
                    print!(" string {s}");
                }
            }
            UciResponse::Option(option) => {
                print!("option name {}", option.name);

                match &option.t {
                    UciOptionType::Check { default, .. } => {
                        print!(" type check default {default}");
                    }
                    UciOptionType::Spin {
                        default, min, max, ..
                    } => {
                        print!(" type spin default {default} min {min} max {max}");
                    }
                    UciOptionType::String { default, .. } => {
                        print!(" type string default {default}");
                    }
                    UciOptionType::Button { .. } => {
                        print!(" type button");
                    }
                }
            }
        }

        println!();
    }

    fn uci_report_search_progress(&self, game: &Game, result: &search::SearchResult) {
        self.send(&UciResponse::Info(InfoFields {
            depth: Some(result.depth),
            seldepth: Some(result.seldepth),
            score: Some(InfoScore::from(result.score, game)),
            wdl: Some(wdl::wdl(result.score, &game.board)),
            pv: Some(
                result
                    .pv
                    .iter()
                    .copied()
                    .map(|m| UciMove::from_move(m, game.is_frc))
                    .collect(),
            ),
            time: Some(result.stats.time),
            nodes: Some(result.stats.nodes),
            nps: Some(metrics::nodes_per_second(result.stats.nodes, result.stats.time)),
            tbhits: Some(result.stats.tbhits),
            hashfull: Some(result.stats.hashfull),
            string: None,
        }));
    }

    // Inspired by Simbelmyne's lovely search output
    fn pretty_report_search_progress(game: &Game, result: &search::SearchResult) {
        use colors::*;

        let score = InfoScore::from(result.score, game);
        let mut game = game.clone();

        print!(" {:>3}", result.depth);
        print!("{BRIGHT_BLACK}/{:<3}{RESET}", result.seldepth);

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

        let as_percentage = |n: f64| (100.0 * n).round() as i32;
        let wdl = wdl::wdl(result.score, &game.board);
        let formatted_wdl = format!(
            "({}/{}/{})",
            as_percentage(wdl.win),
            as_percentage(wdl.draw),
            as_percentage(wdl.loss)
        );

        print!(" {BRIGHT_BLACK}{formatted_wdl:<10}{RESET}");

        let time = if result.stats.time >= Duration::from_secs(1) {
            format!("{:.2}s", result.stats.time.as_secs_f32())
        } else {
            format!("{}ms", result.stats.time.as_millis())
        };

        print!("  {BRIGHT_BLACK}{time:>6}{RESET}");

        let (nodes, nodes_unit) = metrics::unit_suffix(result.stats.nodes);
        print!(" {BRIGHT_BLACK}{:>7}{RESET}", format!("{nodes}{}n", nodes_unit.str()));

        let nps = metrics::nodes_per_second(result.stats.nodes, result.stats.time);
        let (nps, nps_unit) = metrics::unit_suffix(nps);
        print!("  {BRIGHT_BLACK}{:>8}{RESET}", format!("{}{}nps", nps, nps_unit.str()));

        print!(
            "  {BRIGHT_BLACK}{:>4}{RESET}",
            format!("{:.0}%", result.stats.hashfull as f64 / 10.0)
        );

        let first_ten_moves: Vec<Move> = result.pv.iter().take(10).copied().collect();
        let remaining_plies: Vec<Move> = result.pv.iter().skip(10).copied().collect();

        print!("  ");
        for mv in &first_ten_moves {
            let san_mv = san::format_move(&game, *mv);

            let san_mv = san_mv.replace('=', &format!("{GREEN}={RESET}"));
            let san_mv = san_mv.replace('+', &format!("{YELLOW}+{RESET}"));
            let san_mv = san_mv.replace('#', &format!("{BLUE}#{RESET}"));

            print!(
                " {}",
                match game.player {
                    White => format!("{BRIGHT_WHITE}{san_mv}{RESET}"),
                    Black => format!("{BRIGHT_BLACK}{san_mv}{RESET}"),
                }
            );

            game.make_move(*mv);
        }

        if !remaining_plies.is_empty() {
            let mut checkmate_move = None;

            for mv in &remaining_plies {
                let san_mv = san::format_move(&game, *mv);
                if san_mv.contains('#') {
                    let san_mv = san_mv.replace('=', &format!("{GREEN}={RESET}"));
                    let san_mv = san_mv.replace('+', &format!("{YELLOW}+{RESET}"));
                    let san_mv = san_mv.replace('#', &format!("{BLUE}#{RESET}"));

                    checkmate_move = Some(format!(
                        " {}",
                        match game.player {
                            White => format!("{BRIGHT_WHITE}{san_mv}{RESET}"),
                            Black => format!("{BRIGHT_BLACK}{san_mv}{RESET}"),
                        }
                    ));
                }

                game.make_move(*mv);
            }

            let mut remaining_plies = remaining_plies.len();
            if checkmate_move.is_some() {
                remaining_plies -= 1;
            }

            print!(" {BRIGHT_BLACK}[{remaining_plies} plies]{RESET}");

            if let Some(checkmate_move) = checkmate_move {
                print!("{checkmate_move}");
            }
        }

        println!();
    }

    fn uci_best_move(&self, game: &Game, mv: Move) {
        self.send(&UciResponse::BestMove {
            mv: UciMove::from_move(mv, game.is_frc),
        });
    }

    fn pretty_best_move(game: &Game, mv: Move) {
        println!("bestmove {}", san::format_move(game, mv));
    }
}

impl Reporter for UciReporter {
    fn generic_report(&self, s: &str) {
        println!("info string {s}");
    }

    fn report_search_progress(&self, game: &Game, result: &search::SearchResult) {
        if self.pretty_output.load(Ordering::Relaxed) {
            Self::pretty_report_search_progress(game, result);
        } else {
            self.uci_report_search_progress(game, result);
        }
    }

    fn best_move(&self, game: &Game, mv: Move) {
        if self.pretty_output.load(Ordering::Relaxed) {
            Self::pretty_best_move(game, mv);
        } else {
            self.uci_best_move(game, mv);
        }
    }
}
