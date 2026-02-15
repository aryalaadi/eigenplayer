use crate::core::{Core, PropertyValue};
use mlua::{Lua, Result, UserData, UserDataMethods, Value};
use std::sync::{Arc, Mutex};
use tracing::*;

/// Parses a Lua table into a Vec<String>, expecting an array-like table with string values.
fn parse_string_list(table: &mlua::Table) -> Result<Vec<String>> {
    let mut list = Vec::new();
    for pair in table.pairs::<Value, Value>() {
        let (_, val) = pair?;
        if let Value::String(s) = val {
            list.push(s.to_str()?.to_string());
        }
    }
    Ok(list)
}

/// Parses a Lua table into a Vec<[f32; 4]>, expecting an array-like table where each element
/// is a table with up to 4 numeric values (missing values default to 0.0).
fn parse_eq_band_list(table: &mlua::Table) -> Result<Vec<[f32; 4]>> {
    let mut bands = Vec::new();
    for pair in table.pairs::<Value, Value>() {
        let (_, val) = pair?;
        if let Value::Table(band_table) = val {
            let mut band = [0.0f32; 4];
            for i in 0..4 {
                if let Ok(val) = band_table.get::<f64>(i + 1) {
                    band[i] = val as f32;
                } else {
                    band[i] = 0.0;
                }
            }
            bands.push(band);
        }
    }
    Ok(bands)
}

/// Converts a Lua Value to a PropertyValue based on the property name.
/// For table values, dispatches to specific parsers based on the name.
/// Fails loudly for unsupported table property names.
fn value_to_property(name: &str, value: Value) -> Result<PropertyValue> {
    info!("[value_to_property] name: {} value: {:?}", name, value);
    match value {
        Value::String(s) => {Ok(PropertyValue::String(s.to_str()?.to_string()))},
        Value::Boolean(b) => Ok(PropertyValue::Bool(b)),
        Value::Number(n) => Ok(PropertyValue::Float(n as f32)),
	Value::Integer(n) => Ok(PropertyValue::Int(n as i32)),
        Value::Table(ref t) => match name {
            "playlist" => Ok(PropertyValue::StringList(parse_string_list(t)?)),
            "eq_bands" => Ok(PropertyValue::EqBandList(parse_eq_band_list(t)?)),
            _ => Err(mlua::Error::RuntimeError(format!(
                "Unsupported table property: '{}'. Supported table properties are: playlist, eq_bands",
                name
            ))),
        },
        _ => Err(mlua::Error::RuntimeError(
            "Unsupported property type".to_string(),
        )),
    }
}

pub struct LuaCore(pub Arc<Mutex<Core>>);

impl UserData for LuaCore {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "execute_command",
            |_, lua_core: &mut LuaCore, (name, params): (String, Vec<String>)| {
                let mut core = lua_core.0.lock().unwrap();
                core.execute_command(&name, params);
                Ok(())
            },
        );

        methods.add_method_mut(
            "set_property",
            |_lua, lua_core: &mut LuaCore, (name, value): (String, Value)| {
                let mut core = lua_core.0.lock().unwrap();
                let prop_value = value_to_property(&name, value)?;
                core.set_property(&name, prop_value);
                Ok(())
            },
        );

        methods.add_method("get_property", |lua, lua_core: &LuaCore, name: String| {
            let core = lua_core.0.lock().unwrap();
            match core.get_property(&name) {
                Some(PropertyValue::String(s)) => Ok(Value::String(lua.create_string(s)?)),
                Some(PropertyValue::Bool(b)) => Ok(Value::Boolean(*b)),
                Some(PropertyValue::Float(f)) => Ok(Value::Number(*f as f64)),
		Some(PropertyValue::Int(i)) => Ok(Value::Integer(*i as i64)),
                Some(PropertyValue::StringList(list)) => {
                    let table = lua.create_table()?;
                    for (i, item) in list.iter().enumerate() {
                        table.set(i + 1, item.clone())?;
                    }
                    Ok(Value::Table(table))
                }
                Some(PropertyValue::EqBandList(bands)) => {
                    let table = lua.create_table()?;
                    for (i, band) in bands.iter().enumerate() {
                        let band_table = lua.create_table()?;
                        for (j, &val) in band.iter().enumerate() {
                            band_table.set(j + 1, val as f64)?;
                        }
                        table.set(i + 1, band_table)?;
                    }
                    Ok(Value::Table(table))
                }
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("get_string", |_, lua_core: &LuaCore, name: String| {
            let core = lua_core.0.lock().unwrap();
            Ok(core.get_string(&name).cloned())
        });

        methods.add_method("get_bool", |_, lua_core: &LuaCore, name: String| {
            let core = lua_core.0.lock().unwrap();
            Ok(core.get_bool(&name))
        });

        methods.add_method("get_float", |_, lua_core: &LuaCore, name: String| {
            let core = lua_core.0.lock().unwrap();
            Ok(core.get_float(&name))
        });

        methods.add_method(
            "get_string_list",
            |lua, lua_core: &LuaCore, name: String| {
                let core = lua_core.0.lock().unwrap();
                match core.get_string_list(&name) {
                    Some(list) => {
                        let table = lua.create_table()?;
                        for (i, item) in list.iter().enumerate() {
                            table.set(i + 1, item.clone())?;
                        }
                        Ok(Some(table))
                    }
                    None => Ok(None),
                }
            },
        );
    }
}

pub fn init_lua(core: Arc<Mutex<Core>>) -> Result<Lua> {
    let lua = Lua::new();
    lua.globals().set("core", LuaCore(core))?;
    Ok(lua)
}

pub fn run_script(lua: &Lua, script: &str) -> Result<()> {
    lua.load(script).exec()
}
