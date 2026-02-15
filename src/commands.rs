use crate::core::*;
use std::sync::Arc;

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

fn volume_command() -> Command {
    Command {
        execute: Arc::new(|params, core| {
            if let Some(vol_str) = params.get(0) {
                if let Ok(vol) = vol_str.parse::<f32>() {
                    core.set_property("volume", PropertyValue::Float(vol.clamp(0.0, 1.0)));
                }
            }
        }),
    }
}

fn add_command() -> Command {
    Command {
        execute: Arc::new(|params, core| {
            if let Some(track) = params.get(0) {
                if let Some(playlist) = core.get_string_list("playlist") {
                    let mut new_playlist = playlist.clone();
                    new_playlist.push(track.clone());
                    core.set_property("playlist", PropertyValue::StringList(new_playlist));
                }
            }
        }),
    }
}

fn remove_command() -> Command {
    Command {
        execute: Arc::new(|params, core| {
            if let Some(track) = params.get(0) {
                if let Some(playlist) = core.get_string_list("playlist") {
                    let new_playlist: Vec<String> =
                        playlist.iter().filter(|t| *t != track).cloned().collect();

                    core.set_property("playlist", PropertyValue::StringList(new_playlist));
                }
            }
        }),
    }
}

fn next_command() -> Command {
    Command {
        execute: Arc::new(|_params, core| {
            if let (Some(current), Some(playlist)) = (
                core.get_string("current_track"),
                core.get_string_list("playlist"),
            ) {
                if let Some(idx) = playlist.iter().position(|t| t == current) {
                    if idx + 1 < playlist.len() {
                        core.set_property(
                            "current_track",
                            PropertyValue::String(playlist[idx + 1].clone()),
                        );
                        core.set_property("playing", PropertyValue::Bool(true));
                    }
                }
            }
        }),
    }
}

fn prev_command() -> Command {
    Command {
        execute: Arc::new(|_params, core| {
            if let (Some(current), Some(playlist)) = (
                core.get_string("current_track"),
                core.get_string_list("playlist"),
            ) {
                if let Some(idx) = playlist.iter().position(|t| t == current) {
                    if idx > 0 {
                        core.set_property(
                            "current_track",
                            PropertyValue::String(playlist[idx - 1].clone()),
                        );
                        core.set_property("playing", PropertyValue::Bool(true));
                    }
                }
            }
        }),
    }
}

pub fn register_commands(core: &mut Core) {
    core.add_command("play", play_command());
    core.add_command("pause", pause_command());
    core.add_command("volume", volume_command());
    core.add_command("add", add_command());
    core.add_command("remove", remove_command());
    core.add_command("next", next_command());
    core.add_command("prev", prev_command());
}
