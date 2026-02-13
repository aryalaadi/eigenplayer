use mlua::{Lua, Result, Table, Value};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ConfigValue {
    String(String),
    Number(f64),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct Config {
    pub values: HashMap<String, ConfigValue>,
    lua: Option<Lua>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            lua: None,
        }
    }

    pub fn load_from_lua_file(path: &str) -> Result<Self> {
        let lua = Lua::new();
        let script = std::fs::read_to_string(path)?;
        lua.load(&script).exec()?;

        let globals = lua.globals();

        let config_table: Table = globals.get("config")?;

        let mut config = Config::new();

        for pair in config_table.pairs::<String, Value>() {
            let (key, value) = pair?;
            let config_value = match value {
                Value::String(s) => ConfigValue::String(s.to_str()?.to_string()),
                Value::Number(n) => ConfigValue::Number(n),
                Value::Boolean(b) => ConfigValue::Bool(b),
                _ => continue,
            };
            config.values.insert(key, config_value);
        }

        config.lua = Some(lua);

        Ok(config)
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        match self.values.get(key)? {
            ConfigValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn get_number(&self, key: &str) -> Option<f64> {
        match self.values.get(key)? {
            ConfigValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.values.get(key)? {
            ConfigValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn get_nested_float(&self, table: &str, key: &str) -> Option<f64> {
        if let Some(lua) = &self.lua {
            if let Ok(globals) = lua.globals().get::<Table>("config") {
                if let Ok(nested) = globals.get::<Table>(table) {
                    if let Ok(value) = nested.get::<f64>(key) {
                        return Some(value);
                    }
                }
            }
        }
        None
    }
    pub fn get_nested_usize(&self, table: &str, key: &str) -> Option<usize> {
        if let Some(lua) = &self.lua {
            if let Ok(globals) = lua.globals().get::<Table>("config") {
                if let Ok(nested) = globals.get::<Table>(table) {
                    if let Ok(value) = nested.get::<usize>(key) {
                        return Some(value);
                    }
                }
            }
        }
        None
    }
}
