mod core;
use core::*;

use std::sync::Arc;

fn audio_callback(track: &String, _core: &Core) {
    println!("[AUDIO] Now playing '{}'", track);
}

fn ui_callback(track: &String, _core: &Core) {
    println!("[UI] Update display: '{}'", track);
}

fn play_command(params: Vec<String>, core: &mut Core) {
    if let Some(track) = params.get(0) {
        core.set_property("playing", track.clone());
    }
}

fn main() {
    let mut core = Core::new();
    core.add_property("playing", "none".to_string());
    if let Some(prop) = core.properties.get_mut("playing") {
        prop.subscribe(Arc::new(audio_callback));
        prop.subscribe(Arc::new(ui_callback));
    }

    core.subscribe_event(Arc::new(|event, _core| match event {
        EventType::PropertyChanged(name) => {
            println!("[Event] '{}' property changed", name)
        }
        EventType::CommandExecuted(name) => {
            println!("[Event] Command '{}' executed", name)
        }
    }));

    core.add_command(
        "play",
        Command {
            execute: Arc::new(play_command),
        },
    );

    core.execute_command("play", vec!["MySong.mp3".to_string()]);
}
