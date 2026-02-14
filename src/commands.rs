use crate::core::*;
use std::sync::{Arc, Mutex};

fn play_command() -> Command {
        Command {
            execute: Arc::new(|params, core| {
                if let Some(track) = params.get(0) {
                    core.set_property("current_track", PropertyValue::String(track.clone()));
                    core.set_property("playing", PropertyValue::Bool(true));
                }
            }),
        }
}

fn pause_command() -> Command {
        Command {
            execute: Arc::new(|_params, core| {
                core.set_property("playing", PropertyValue::Bool(false));
            }),
        }
}

pub fn register_commands(core: &mut Core) {
    core.add_command("play", play_command());
    core.add_command("pause", pause_command());
}

