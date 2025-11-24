use crate::engine::{options::EngineOptions, search::PersistentState};

pub trait UciOption {
    fn name(&self) -> &'static str;
    fn uci_option_line(&self) -> String;
    fn set(
        &self,
        value: &str,
        options: &mut EngineOptions,
        state: &mut PersistentState,
    ) -> Result<(), String>;
}

#[expect(unused, reason = "No check options yet")]
pub struct UciCheckOption {
    pub name: &'static str,
    pub default: bool,
    pub set_fn: Box<dyn Fn(&mut EngineOptions, &mut PersistentState, bool)>,
}

pub struct UciSpinOption {
    pub name: &'static str,
    pub default: isize,
    pub min: isize,
    pub max: isize,
    pub set_fn: Box<dyn Fn(&mut EngineOptions, &mut PersistentState, isize)>,
}

impl UciOption for UciSpinOption {
    fn name(&self) -> &'static str {
        self.name
    }

    fn uci_option_line(&self) -> String {
        let name = self.name;
        let default = self.default;
        let min = self.min;
        let max = self.max;

        format!("option name {name} type spin default {default} min {min} max {max}")
    }

    fn set(
        &self,
        value: &str,
        options: &mut EngineOptions,
        state: &mut PersistentState,
    ) -> Result<(), String> {
        let value = value.parse::<isize>().map_err(|_| "Invalid value")?;

        if value > self.max {
            return Err("Value larger than max".to_string());
        }

        if value < self.min {
            return Err("Value smaller than min".to_string());
        }

        (self.set_fn)(options, state, value);
        Ok(())
    }
}

#[expect(unused, reason = "No combo options yet")]
pub struct UciComboOption {
    pub name: &'static str,
    pub default: &'static str,
    pub values: Vec<&'static str>,
    pub set_fn: Box<dyn Fn(&mut EngineOptions, &mut PersistentState, &str)>,
}

pub struct UciStringOption {
    pub name: &'static str,
    pub default: &'static str,
    pub set_fn: Box<dyn Fn(&mut EngineOptions, &mut PersistentState, &str)>,
}

impl UciOption for UciStringOption {
    fn name(&self) -> &'static str {
        self.name
    }

    fn uci_option_line(&self) -> String {
        let name = self.name;
        let default = self.default;

        format!("option name {name} type string default {default}")
    }

    fn set(
        &self,
        value: &str,
        options: &mut EngineOptions,
        state: &mut PersistentState,
    ) -> Result<(), String> {
        let value = value.parse::<String>().map_err(|_| "Invalid value")?;

        (self.set_fn)(options, state, &value);
        Ok(())
    }
}

#[expect(unused, reason = "No button options yet")]
pub struct UciButtonOption {
    pub name: &'static str,
    pub set_fn: Box<dyn Fn(&mut EngineOptions, &mut PersistentState)>,
}
