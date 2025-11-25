use crate::engine::{options::EngineOptions, search::PersistentState};

pub struct UciOption {
    pub name: &'static str,
    pub t: UciOptionType,
}

type SetFn<T> = Box<dyn Fn(&mut EngineOptions, &mut PersistentState, T)>;

pub enum UciOptionType {
    Check {
        default: bool,
        set_fn: SetFn<bool>,
    },
    Spin {
        default: isize,
        min: isize,
        max: isize,
        set_fn: SetFn<isize>,
    },
    Combo {
        default: &'static str,
        values: Vec<&'static str>,
        set_fn: SetFn<String>,
    },
    String {
        default: &'static str,
        set_fn: SetFn<String>,
    },
    Button {
        set_fn: SetFn<()>,
    },
}

impl UciOption {
    pub fn set(
        &self,
        value: &str,
        options: &mut EngineOptions,
        state: &mut PersistentState,
    ) -> Result<(), String> {
        match &self.t {
            UciOptionType::Check { set_fn: _, .. } => {
                todo!()
            }
            UciOptionType::Spin {
                min, max, set_fn, ..
            } => {
                let value = value.parse::<isize>().map_err(|_| "Invalid value")?;

                if value > *max {
                    return Err("Value larger than max".to_string());
                }

                if value < *min {
                    return Err("Value smaller than min".to_string());
                }

                set_fn(options, state, value);
                Ok(())
            }
            UciOptionType::Combo { set_fn: _, .. } => {
                todo!()
            }
            UciOptionType::String { set_fn, .. } => {
                let value = value.parse::<String>().map_err(|_| "Invalid value")?;

                set_fn(options, state, value);
                Ok(())
            }
            UciOptionType::Button { set_fn: _ } => {
                todo!()
            }
        }
    }
}
