use crate::{
    chess::game::Game,
    engine::{eval::Eval, options::EngineOptions, search::PersistentState},
};

pub struct UciOption {
    pub name: &'static str,
    pub t: UciOptionType,
}

type SetFn<T> = Box<dyn Fn(&mut OptionCallbackRefs<'_>, T)>;

pub enum UciOptionType {
    Check {
        default: bool,
        set_fn: SetFn<bool>,
    },
    Spin {
        default: i32,
        min: i32,
        max: i32,
        set_fn: SetFn<SpinValue>,
    },
    String {
        default: String,
        set_fn: SetFn<String>,
    },
    Button {
        set_fn: SetFn<()>,
    },
}

pub struct SpinValue(i32);

impl SpinValue {
    pub fn new(value: i32) -> Self {
        Self(value)
    }

    pub fn as_i32(&self) -> i32 {
        self.0
    }

    pub fn as_u32(&self) -> u32 {
        u32::try_from(self.0).expect("Could not convert value to u32")
    }

    pub fn as_u64(&self) -> u64 {
        u64::try_from(self.0).expect("Could not convert value to u64")
    }

    pub fn as_usize(&self) -> usize {
        usize::try_from(self.0).expect("Could not convert value to usize")
    }

    pub fn as_depth(&self) -> u8 {
        u8::try_from(self.0).expect("Could not convert value to depth")
    }

    pub fn as_eval(&self) -> Eval {
        Eval(self.0)
    }
}

pub struct OptionCallbackRefs<'uci> {
    pub options: &'uci mut EngineOptions,
    pub state: &'uci mut PersistentState,
    pub game: &'uci mut Game,
}

impl UciOption {
    pub fn check(
        name: &'static str,
        f: impl Fn(&mut OptionCallbackRefs<'_>, bool) + 'static,
    ) -> UciCheckOptionBuilder {
        UciCheckOptionBuilder {
            name,
            set_fn: Box::new(f),

            default: None,
        }
    }

    pub fn spin(
        name: &'static str,
        f: impl Fn(&mut OptionCallbackRefs<'_>, SpinValue) + 'static,
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
        f: impl Fn(&mut OptionCallbackRefs<'_>, String) + 'static,
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
        game: &mut Game,
    ) -> Result<(), String> {
        match &self.t {
            UciOptionType::Check { set_fn, .. } => {
                let value = value.parse::<bool>().map_err(|_| "Invalid value")?;

                set_fn(
                    &mut OptionCallbackRefs {
                        options,
                        state,
                        game,
                    },
                    value,
                );

                Ok(())
            }
            UciOptionType::Spin {
                min, max, set_fn, ..
            } => {
                let value = value.parse::<i32>().map_err(|_| "Invalid value")?;

                if value > *max {
                    return Err("Value larger than max".to_string());
                }

                if value < *min {
                    return Err("Value smaller than min".to_string());
                }

                set_fn(
                    &mut OptionCallbackRefs {
                        options,
                        state,
                        game,
                    },
                    SpinValue::new(value),
                );

                Ok(())
            }
            UciOptionType::String { set_fn, .. } => {
                let value = value.parse::<String>().map_err(|_| "Invalid value")?;

                set_fn(
                    &mut OptionCallbackRefs {
                        options,
                        state,
                        game,
                    },
                    value,
                );

                Ok(())
            }
            UciOptionType::Button { set_fn } => {
                set_fn(
                    &mut OptionCallbackRefs {
                        options,
                        state,
                        game,
                    },
                    (),
                );

                Ok(())
            }
        }
    }
}

pub struct UciCheckOptionBuilder {
    name: &'static str,
    set_fn: SetFn<bool>,

    default: Option<bool>,
}

impl UciCheckOptionBuilder {
    pub fn default(mut self, value: bool) -> Self {
        self.default = Some(value);
        self
    }

    pub fn build(self) -> UciOption {
        let default = self
            .default
            .unwrap_or_else(|| panic!("No default value provided for {}", self.name));

        UciOption {
            name: self.name,
            t: UciOptionType::Check {
                default,

                set_fn: self.set_fn,
            },
        }
    }
}

pub struct UciSpinOptionBuilder {
    name: &'static str,
    set_fn: SetFn<SpinValue>,

    default: Option<i32>,
    min: Option<i32>,
    max: Option<i32>,
}

impl From<SpinValue> for i32 {
    fn from(value: SpinValue) -> Self {
        value.0
    }
}

impl From<SpinValue> for u8 {
    fn from(value: SpinValue) -> Self {
        Self::try_from(value.0).expect("Value should fit in a u8")
    }
}

impl From<SpinValue> for u32 {
    fn from(value: SpinValue) -> Self {
        Self::try_from(value.0).expect("Value should fit in a u32")
    }
}

impl From<usize> for SpinValue {
    fn from(value: usize) -> Self {
        Self(i32::try_from(value).expect("Value should fit in an i32"))
    }
}

impl From<u8> for SpinValue {
    fn from(value: u8) -> Self {
        Self(i32::from(value))
    }
}

impl From<i32> for SpinValue {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl UciSpinOptionBuilder {
    pub fn default(mut self, value: impl Into<i32>) -> Self {
        self.default = Some(value.into());
        self
    }

    pub fn with_bounds(mut self, min: impl Into<i32>, max: impl Into<i32>) -> Self {
        self.min = Some(min.into());
        self.max = Some(max.into());
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
