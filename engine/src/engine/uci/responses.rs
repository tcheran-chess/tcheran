use std::{fmt::Formatter, time::Duration};

use crate::{
    chess::{game::Game, moves::Move, player::Player, san},
    engine::{
        eval::{Eval, wdl, wdl::WdlProbabilities},
        search,
        search::Reporter,
        uci::{
            UciMove,
            options::{UciOption, UciOptionType},
        },
        util::{metrics, metrics::UnitPrefix},
    },
};

pub(super) fn send_response(response: &UciResponse<'_>) {
    println!("{response}");
}

#[derive(Debug)]
pub(super) enum InfoScore {
    Centipawns(i32),
    Mate(i32),
}

impl InfoScore {
    pub fn from(eval: Eval, game: &Game) -> Self {
        if let Some(nmoves) = eval.is_mate_in_moves() {
            Self::Mate(nmoves)
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

impl std::fmt::Display for UciResponse<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id(i) => match i {
                IdParam::Name(name) => write!(f, "id name {name}")?,
                IdParam::Author(author) => write!(f, "id author {author}")?,
            },
            Self::UciOk => write!(f, "uciok")?,
            Self::ReadyOk => write!(f, "readyok")?,
            Self::BestMove { mv } => {
                write!(f, "bestmove {}", mv.notation())?;
            }
            Self::Info(InfoFields {
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
                write!(f, "info")?;

                if let Some(depth) = depth {
                    write!(f, " depth {depth}")?;
                }

                if let Some(seldepth) = seldepth {
                    write!(f, " seldepth {seldepth}")?;
                }

                if let Some(score) = score {
                    match score {
                        InfoScore::Centipawns(centipawns) => {
                            write!(f, " score cp {centipawns}")?;
                        }
                        InfoScore::Mate(turns) => {
                            write!(f, " score mate {turns}")?;
                        }
                    }
                }

                if let Some(wdl) = wdl {
                    #[expect(clippy::cast_possible_truncation, reason = "Approximate calculation")]
                    let format_wdl = |n: f64| (1000.0 * n).round() as i32;
                    write!(
                        f,
                        " wdl {} {} {}",
                        format_wdl(wdl.win),
                        format_wdl(wdl.draw),
                        format_wdl(wdl.loss)
                    )?;
                }

                if let Some(time) = time {
                    write!(f, " time {}", time.as_millis())?;
                }

                if let Some(nodes) = nodes {
                    write!(f, " nodes {nodes}")?;
                }

                if let Some(nps) = nps {
                    write!(f, " nps {nps}")?;
                }

                if let Some(hashfull) = hashfull {
                    write!(f, " hashfull {hashfull}")?;
                }

                if let Some(tbhits) = tbhits {
                    write!(f, " tbhits {tbhits}")?;
                }

                if let Some(pv) = pv {
                    write!(f, " pv")?;

                    for mv in pv {
                        write!(f, " {}", mv.notation())?;
                    }
                }

                if let Some(s) = string {
                    write!(f, " string {s}")?;
                }
            }
            Self::Option(option) => {
                write!(f, "option name {}", option.name)?;

                match &option.t {
                    UciOptionType::Check { default, .. } => {
                        write!(f, " type check default {default}")?;
                    }
                    UciOptionType::Spin {
                        default, min, max, ..
                    } => {
                        write!(f, " type spin default {default} min {min} max {max}")?;
                    }
                    UciOptionType::String { default, .. } => {
                        write!(f, " type string default {default}")?;
                    }
                    UciOptionType::Button { .. } => {
                        write!(f, " type button")?;
                    }
                }
            }
        }

        Ok(())
    }
}

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
    fn uci_report_search_progress(progress: &search::SearchInfo<'_>) {
        send_response(&UciResponse::Info(InfoFields {
            depth: Some(progress.stats.depth),
            seldepth: Some(progress.stats.seldepth),
            score: Some(InfoScore::from(progress.eval, progress.game)),
            wdl: Some(wdl::wdl(progress.eval, &progress.game.board)),
            pv: Some(
                progress
                    .pv
                    .iter()
                    .copied()
                    .map(|m| UciMove::from_move(m, progress.game.is_frc))
                    .collect(),
            ),
            time: Some(progress.stats.time),
            nodes: Some(progress.stats.nodes),
            nps: Some(progress.stats.nodes_per_second),
            tbhits: Some(progress.stats.tbhits),
            hashfull: Some(progress.stats.hashfull),
            string: None,
        }));
    }

    // Inspired by Simbelmyne's lovely search output
    #[expect(clippy::cast_precision_loss, reason = "Various approximate calculations")]
    fn pretty_report_search_progress(progress: &search::SearchInfo<'_>) {
        use colors::*;

        let score = InfoScore::from(progress.eval, progress.game);
        let mut game = progress.game.clone();

        print!(" {:>3}", progress.stats.depth);
        print!("{BRIGHT_BLACK}/{:<3}{RESET}", progress.stats.seldepth);

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

        let (nodes, nodes_unit) = metrics::unit_suffix(progress.stats.nodes);
        let nodes_suffix = match nodes_unit {
            UnitPrefix::None => "n",
            UnitPrefix::Kilo => "kn",
            UnitPrefix::Mega => "mn",
            UnitPrefix::Giga => "gn",
            UnitPrefix::Tera => "tn",
        };

        print!(" {BRIGHT_BLACK}{:>7}{RESET}", format!("{nodes}{nodes_suffix}"));

        let (nps, nps_unit) = metrics::unit_suffix(progress.stats.nodes_per_second);
        let nps_suffix = match nps_unit {
            UnitPrefix::None => "nps",
            UnitPrefix::Kilo => "knps",
            UnitPrefix::Mega => "mnps",
            UnitPrefix::Giga => "gnps",
            UnitPrefix::Tera => "tnps",
        };

        print!("  {BRIGHT_BLACK}{:>8}{RESET}", format!("{}{}", nps, nps_suffix));

        print!(
            "  {BRIGHT_BLACK}{:>4}{RESET}",
            format!("{:.0}%", progress.stats.hashfull as f64 / 10.0)
        );

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

    fn uci_best_move(game: &Game, mv: Move) {
        send_response(&UciResponse::BestMove {
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

    fn report_search_progress(&self, progress: search::SearchInfo<'_>) {
        if self.pretty_output {
            Self::pretty_report_search_progress(&progress);
        } else {
            Self::uci_report_search_progress(&progress);
        }
    }

    fn best_move(&self, game: &Game, mv: Move) {
        if self.pretty_output {
            Self::pretty_best_move(game, mv);
        } else {
            Self::uci_best_move(game, mv);
        }
    }
}
