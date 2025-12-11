use std::{fmt::Formatter, time::Duration};

use crate::engine::uci::{
    UciMove,
    options::{UciOption, UciOptionType},
};

#[derive(Debug)]
pub(super) enum InfoScore {
    Centipawns(i32),
    Mate(i32),
}

#[derive(Debug)]
pub(super) enum IdParam {
    Name(String),
    Author(&'static str),
}

#[derive(Debug, Default)]
pub struct InfoFields {
    pub(super) depth: Option<u8>,
    pub(super) seldepth: Option<u8>,
    pub(super) time: Option<Duration>,
    pub(super) nodes: Option<u64>,
    pub(super) pv: Option<Vec<UciMove>>,
    pub(super) score: Option<InfoScore>,
    pub(super) hashfull: Option<usize>,
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
                    UciOptionType::Combo { default, .. } => {
                        write!(f, " type combo default {default}")?;
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
