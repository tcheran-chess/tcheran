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
        default: String,
        set_fn: SetFn<String>,
    },
    Button {
        set_fn: SetFn<()>,
    },
}

impl UciOption {
    pub fn spin(
        name: &'static str,
        f: impl Fn(&mut EngineOptions, &mut PersistentState, isize) + 'static,
    ) -> UciSpinOptionBuilder {
        UciSpinOptionBuilder {
            name,
            set_fn: Box::new(f),

            default: None,
            min: None,
            max: None,
        }
    }

    pub fn string(
        name: &'static str,
        f: impl Fn(&mut EngineOptions, &mut PersistentState, String) + 'static,
    ) -> UciStringOptionBuilder {
        UciStringOptionBuilder {
            name,
            set_fn: Box::new(f),

            default: None,
        }
    }

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

pub struct UciSpinOptionBuilder {
    name: &'static str,
    set_fn: SetFn<isize>,

    default: Option<isize>,
    min: Option<isize>,
    max: Option<isize>,
}

pub trait ToUciSpinOptionValue {
    fn convert(self) -> isize;
}

impl ToUciSpinOptionValue for usize {
    fn convert(self) -> isize {
        isize::try_from(self).expect("Value should fit in an isize")
    }
}

impl UciSpinOptionBuilder {
    pub fn default(mut self, value: impl ToUciSpinOptionValue) -> Self {
        self.default = Some(value.convert());
        self
    }

    pub fn with_bounds(
        mut self,
        min: impl ToUciSpinOptionValue,
        max: impl ToUciSpinOptionValue,
    ) -> Self {
        self.min = Some(min.convert());
        self.max = Some(max.convert());
        self
    }

    pub fn build(self) -> UciOption {
        let default = self
            .default
            .unwrap_or_else(|| panic!("No default value provided for {}", self.name));
        let min = self
            .min
            .unwrap_or_else(|| panic!("No min value provided for {}", self.name));
        let max = self
            .max
            .unwrap_or_else(|| panic!("No max value provided for {}", self.name));

        UciOption {
            name: self.name,
            t: UciOptionType::Spin {
                default,
                min,
                max,

                set_fn: self.set_fn,
            },
        }
    }
}

pub struct UciStringOptionBuilder {
    name: &'static str,
    set_fn: SetFn<String>,

    default: Option<String>,
}

impl UciStringOptionBuilder {
    pub fn default(mut self, value: String) -> Self {
        self.default = Some(value);
        self
    }

    pub fn build(self) -> UciOption {
        let default = self
            .default
            .unwrap_or_else(|| panic!("No default value provided for {}", self.name));

        UciOption {
            name: self.name,
            t: UciOptionType::String {
                default,

                set_fn: self.set_fn,
            },
        }
    }
}
