use mlua::{Lua, UserData, UserDataMethods, Result};
use crate::core::Core;

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
            |_, core: &mut Core, (name, value): (String, String)| {
                core.set_property(&name, value);
                Ok(())
            },
        );

        methods.add_method(
            "get_property",
            |_, core: &Core, name: String| {
                Ok(core.get_property(&name).cloned().unwrap_or_default())
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
