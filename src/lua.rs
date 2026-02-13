use crate::core::{Core, PropertyValue};
use mlua::{Lua, Result, UserData, UserDataMethods, Value};

impl UserData for Core {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "execute_command",
            |_, core: &mut Core, (name, params): (String, Vec<String>)| {
                core.execute_command(&name, params);
                Ok(())
            },
        );

        methods.add_method_mut(
            "set_property",
            |_lua, core: &mut Core, (name, value): (String, Value)| {
                let prop_value = match value {
                    Value::String(s) => PropertyValue::String(s.to_str()?.to_string()),
                    Value::Boolean(b) => PropertyValue::Bool(b),
                    Value::Number(n) => PropertyValue::Float(n as f32),
                    Value::Table(t) => {
                        let mut list = Vec::new();
                        for pair in t.pairs::<i32, String>() {
                            let (_, val) = pair?;
                            list.push(val);
                        }
                        PropertyValue::StringList(list)
                    }
                    _ => {
                        return Err(mlua::Error::RuntimeError(
                            "Unsupported property type".to_string(),
                        ));
                    }
                };
                core.set_property(&name, prop_value);
                Ok(())
            },
        );

        methods.add_method("get_property", |lua, core: &Core, name: String| match core
            .get_property(&name)
        {
            Some(PropertyValue::String(s)) => Ok(Value::String(lua.create_string(s)?)),
            Some(PropertyValue::Bool(b)) => Ok(Value::Boolean(*b)),
            Some(PropertyValue::Float(f)) => Ok(Value::Number(*f as f64)),
            Some(PropertyValue::StringList(list)) => {
                let table = lua.create_table()?;
                for (i, item) in list.iter().enumerate() {
                    table.set(i + 1, item.clone())?;
                }
                Ok(Value::Table(table))
            }
            None => Ok(Value::Nil),
        });

        methods.add_method("get_string", |_, core: &Core, name: String| {
            Ok(core.get_string(&name).cloned())
        });

        methods.add_method("get_bool", |_, core: &Core, name: String| {
            Ok(core.get_bool(&name))
        });

        methods.add_method("get_float", |_, core: &Core, name: String| {
            Ok(core.get_float(&name))
        });

        methods.add_method(
            "get_string_list",
            |lua, core: &Core, name: String| match core.get_string_list(&name) {
                Some(list) => {
                    let table = lua.create_table()?;
                    for (i, item) in list.iter().enumerate() {
                        table.set(i + 1, item.clone())?;
                    }
                    Ok(Some(table))
                }
                None => Ok(None),
            },
        );
    }
}

pub fn init_lua(core: Core) -> Result<Lua> {
    let lua = Lua::new();
    lua.globals().set("core", core)?;
    Ok(lua)
}

pub fn run_script(lua: &Lua, script: &str) -> Result<()> {
    lua.load(script).exec()
}
