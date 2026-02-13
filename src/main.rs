use eigenplayer::audio::AudioBackend;
use eigenplayer::config::Config;
use eigenplayer::core::*;
use eigenplayer::db::Database;
use eigenplayer::repl::Repl;

use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== EigenPlayer ===\n");

    let config = match Config::load_from_lua_file("config.lua") {
        Ok(cfg) => {
            println!("[Config] Loaded config.lua");
            cfg
        }
        Err(_) => {
            println!("[Config] Using default configuration");
            Config::new()
        }
    };

    let default_volume = config
        .get_nested_float("audio", "default_volume")
        .or_else(|| config.get_number("volume"))
        .unwrap_or(0.5) as f32;

    // 88200 = 2 seconds at 44.1 kHz
    // its the standard audio quality
    // config.lua lets you set this to whatever
    let ring_buffer_size = config
        .get_nested_usize("audio", "ring_buffer_size")
        .unwrap_or(88200) as usize;

    let mut core = Core::new();
    core.add_property("playing", PropertyValue::Bool(false));
    core.add_property("current_track", PropertyValue::String("none".to_string()));
    core.add_property("volume", PropertyValue::Float(default_volume));
    core.add_property("playlist", PropertyValue::StringList(Vec::new()));

    let db = Database::new("playlists.db")?;
    println!("[Database] Initialized playlists.db");

    if let Ok(tracks) = db.get_playlist_tracks("default") {
        if !tracks.is_empty() {
            core.set_property("playlist", PropertyValue::StringList(tracks.clone()));
            println!(
                "[Database] Loaded default playlist with {} tracks",
                tracks.len()
            );
        }
    }

    let audio_backend = Arc::new(Mutex::new(AudioBackend::with_ring_buffer_size(ring_buffer_size,
										default_volume)?));
    println!("[Audio] Initialized audio backend with {} prebuffer packets", ring_buffer_size);

    let audio_for_track = Arc::clone(&audio_backend);
    if let Some(prop) = core.properties.get_mut("current_track") {
        prop.subscribe(Arc::new(move |value, core| {
            if let Some(track) = value.as_string() {
                if track != "none" {
                    println!("[Audio] Loading track: {}", track);
                    let mut audio = audio_for_track.lock().unwrap();
                    if let Err(e) = audio.load_track(track) {
                        eprintln!("[Audio] Failed to load track: {}", e);
                    } else {
                        if let Some(true) = core.get_bool("playing") {
                            if let Err(e) = audio.play() {
                                eprintln!("[Audio] Failed to start playback: {}", e);
                            }
                        }
                    }
                }
            }
        }));
    }

    let audio_for_playing = Arc::clone(&audio_backend);
    if let Some(prop) = core.properties.get_mut("playing") {
        prop.subscribe(Arc::new(move |value, _core| {
            if let Some(playing) = value.as_bool() {
                let mut audio = audio_for_playing.lock().unwrap();
                if playing {
                    if let Err(e) = audio.play() {
                        eprintln!("[Audio] Failed to start playback: {}", e);
                    }
                } else {
                    audio.pause();
                }
            }
        }));
    }

    let audio_for_volume = Arc::clone(&audio_backend);
    if let Some(prop) = core.properties.get_mut("volume") {
        prop.subscribe(Arc::new(move |value, _core| {
            if let Some(vol) = value.as_float() {
                let mut audio = audio_for_volume.lock().unwrap();
                audio.set_volume(vol);
            }
        }));
    }

    core.add_command(
        "play",
        Command {
            execute: Arc::new(|params, core| {
                if let Some(track) = params.get(0) {
                    core.set_property("current_track", PropertyValue::String(track.clone()));
                    core.set_property("playing", PropertyValue::Bool(true));
                }
            }),
        },
    );

    core.add_command(
        "pause",
        Command {
            execute: Arc::new(|_params, core| {
                core.set_property("playing", PropertyValue::Bool(false));
            }),
        },
    );

    core.add_command(
        "stop",
        Command {
            execute: Arc::new(|_params, core| {
                core.set_property("playing", PropertyValue::Bool(false));
                core.set_property("current_track", PropertyValue::String("none".to_string()));
            }),
        },
    );

    core.add_command(
        "volume",
        Command {
            execute: Arc::new(|params, core| {
                if let Some(vol_str) = params.get(0) {
                    if let Ok(vol) = vol_str.parse::<f32>() {
                        core.set_property("volume", PropertyValue::Float(vol.clamp(0.0, 1.0)));
                    }
                }
            }),
        },
    );

    core.add_command(
        "add",
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
        },
    );

    core.add_command(
        "remove",
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
        },
    );

    core.add_command(
        "next",
        Command {
            execute: Arc::new(|_params, core| {
                if let (Some(current), Some(playlist)) = (
                    core.get_string("current_track"),
                    core.get_string_list("playlist"),
                ) {
                    if let Some(idx) = playlist.iter().position(|t| t == current) {
                        if idx + 1 < playlist.len() {
                            let next_track = playlist[idx + 1].clone();
                            core.set_property("current_track", PropertyValue::String(next_track));
                            core.set_property("playing", PropertyValue::Bool(true));
                        }
                    }
                }
            }),
        },
    );

    core.add_command(
        "prev",
        Command {
            execute: Arc::new(|_params, core| {
                if let (Some(current), Some(playlist)) = (
                    core.get_string("current_track"),
                    core.get_string_list("playlist"),
                ) {
                    if let Some(idx) = playlist.iter().position(|t| t == current) {
                        if idx > 0 {
                            let prev_track = playlist[idx - 1].clone();
                            core.set_property("current_track", PropertyValue::String(prev_track));
                            core.set_property("playing", PropertyValue::Bool(true));
                        }
                    }
                }
            }),
        },
    );

    core.subscribe_event(Arc::new(|event, _core| match event {
        EventType::PropertyChanged(name) => {
            if name != "playing" {
                println!("[Core] Property '{}' changed", name);
            }
        }
        EventType::CommandExecuted(name) => {
            println!("[Core] Command '{}' executed", name);
        }
    }));

    println!("\nInitialization complete!\n");

    let mut repl = Repl::new(db);
    repl.run(&mut core)?;

    Ok(())
}
