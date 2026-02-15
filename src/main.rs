use eigenplayer::audio::AudioBackend;
use eigenplayer::commands::*;
use eigenplayer::config::Config;
use eigenplayer::core::*;
use eigenplayer::db::Database;
use eigenplayer::property::*;
use eigenplayer::repl::Repl;
use std::sync::{Arc, Mutex};
use tracing::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logging_level = if cfg!(debug_assertions) {
        Level::TRACE
    } else {
        Level::WARN
    };

    tracing_subscriber::fmt()
        .with_max_level(logging_level)
        .with_file(true)
        .with_line_number(true)
        .init();

    let config = match Config::load_from_lua_file("config.lua") {
        Ok(cfg) => {
            info!("[Config] Loaded config.lua");
            cfg
        }
        Err(_) => {
            warn!("[Config] Using default configuration");
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

    let enable_eq = config
        .get_nested_bool("audio", "enable_eq")
        .unwrap_or(false) as bool;
    let eq_bands: Vec<[f32; 4]> = config
        .get_nested_eq_bands("audio", "eq_bands")
        .unwrap_or_default();

    let mut core = Core::new();
    register_property(&mut core, default_volume, enable_eq);

    let db = Database::new("playlists.db")?;
    info!("[Database] Initialized playlists.db");

    if let Ok(tracks) = db.get_playlist_tracks("default") {
        if !tracks.is_empty() {
            core.set_property("playlist", PropertyValue::StringList(tracks.clone()));
            info!(
                "[Database] Loaded default playlist with {} tracks",
                tracks.len()
            );
        }
    }

    let audio_backend = Arc::new(Mutex::new(AudioBackend::with_ring_buffer_size(
        ring_buffer_size,
        default_volume,
        enable_eq,
        eq_bands,
    )?));

    println!(
        "[Audio] Initialized audio backend with {} prebuffer packets",
        ring_buffer_size
    );

    let audio_for_track = Arc::clone(&audio_backend);
    if let Some(prop) = core.properties.get_mut("current_track") {
        prop.subscribe(Arc::new(move |value, core| {
            if let Some(track) = value.as_string() {
                if track != "none" {
                    info!("[Audio] Loading track: {}", track);
                    let mut audio = audio_for_track.lock().unwrap();
                    if let Err(e) = audio.load_track(track) {
                        warn!("[Audio] Failed to load track: {}", e);
                    } else {
                        if let Some(true) = core.get_bool("playing") {
                            if let Err(e) = audio.play() {
                                warn!("[Audio] Failed to start playback: {}", e);
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
                        warn!("[Audio] Failed to start playback: {}", e);
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

    register_commands(&mut core);

    core.subscribe_event(Arc::new(|event, _core| match event {
        EventType::PropertyChanged(name) => {
            if name != "playing" {
                info!("[Core] Property '{}' changed", name);
            }
        }
        EventType::CommandExecuted(name) => {
            info!("[Core] Command '{}' executed", name);
        }
    }));

    println!("\nInitialization complete!\n");

    let mut repl = Repl::new(db);
    repl.run(&mut core)?;

    Ok(())
}
