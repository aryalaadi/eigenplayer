mod core;
mod lua;
mod config;

use core::*;
use lua::{init_lua, run_script};
use crate::config::Config;

use std::sync::Arc;
use std::sync::Mutex;
use mlua::Result;

fn audio_callback(track: &String, _core: &Core) {
    println!("[Audio] Now playing '{}'", track);
}

fn ui_callback(track: &String, _core: &Core) {
    println!("[UI] Update display: '{}'", track);
}

fn play_command(params: Vec<String>, core: &mut Core) {
    if let Some(track) = params.get(0) {
        core.set_property("playing", track.clone());
    }
}

fn main() -> Result<()> {

    let mut core = Core::new();
    let config = match Config::load_from_lua_file("config.lua") {
	Ok(cfg) => {
	    println!("[Config] loaded config.lua");
	    cfg
	},
	Err(_err) => {
	    eprintln!("[Config] Could not find config.lua");
	    Config::new()
	}
    };

    let volume = config.get_number("volume")
	.unwrap_or(0.5)?;
    println!("Volume: {}", volume);
    
    core.add_property("playing", "none".to_string());

    if let Some(prop) = core.properties.get_mut("playing") {
        use std::sync::Arc;
        prop.subscribe(Arc::new(audio_callback));
        prop.subscribe(Arc::new(ui_callback));
    }

    core.subscribe_event(Arc::new(|event, _core| match event {
        EventType::PropertyChanged(name) => println!("[Event] '{}' property changed", name),
        EventType::CommandExecuted(name) => println!("[Event] Command '{}' executed", name),
    }));

    core.add_command(
        "play",
        Command {
            execute: Arc::new(play_command),
        },
    );

    let lua = init_lua(core)?;

    let script = r#"
        print("[Lua] Current track:", core:get_property("playing"))
        core:execute_command("play", {"LuaSong.mp3"})
        print("[Lua] Now playing:", core:get_property("playing"))
    "#;

    run_script(&lua, script)?;

    Ok(())
}
