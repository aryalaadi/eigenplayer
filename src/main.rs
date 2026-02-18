use eigenplayer::audio::AudioBackend;
use eigenplayer::commands::*;
use eigenplayer::core::*;
use eigenplayer::db::Database;
use eigenplayer::lua::{init_lua, run_script};
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

    let core = Arc::new(Mutex::new(Core::new()));

    // Register default properties
    {
        let mut core_lock = core.lock().unwrap();
        register_property(&mut *core_lock);
    }

    // Load and execute config.lua to set config properties
    match std::fs::read_to_string("config.lua") {
        Ok(script) => match init_lua(Arc::clone(&core)) {
            Ok(lua) => match run_script(&lua, &script) {
                Ok(_) => info!("[Config] Successfully loaded and executed config.lua"),
                Err(e) => warn!("[Config] Failed to execute config.lua: {}", e),
            },
            Err(e) => warn!("[Config] Failed to initialize Lua for config: {}", e),
        },
        Err(_) => {
            warn!("[Config] config.lua not found, using default configuration");
        }
    }

    // Now get the values from properties
    let (default_volume, ring_buffer_size, enable_eq, eq_bands) = {
        let core_lock = core.lock().unwrap();
        let default_volume = core_lock.get_float("default_volume").unwrap_or(0.5);
        let ring_buffer_size = core_lock.get_float("ring_buffer_size").unwrap_or(88200.0) as usize;
        let enable_eq = core_lock.get_bool("enable_eq").unwrap_or(false);
        let eq_bands = core_lock
            .get_property("eq_bands")
            .and_then(|v| v.as_eq_band_list())
            .cloned()
            .unwrap_or_default();
        (default_volume, ring_buffer_size, enable_eq, eq_bands)
    };

    let db = Database::new("playlists.db")?;
    info!("[Database] Initialized playlists.db");

    if let Ok(tracks) = db.get_playlist_tracks("default") {
        if !tracks.is_empty() {
            {
                let mut core_lock = core.lock().unwrap();
                core_lock.set_property("playlist", PropertyValue::StringList(tracks.clone()));
            }
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
    {
        let mut core_lock = core.lock().unwrap();
        if let Some(prop) = core_lock.properties.get_mut("current_track") {
            prop.subscribe(Arc::new(move |value, _core| {
                if let Some(track) = value.as_string() {
                    if track != "none" {
                        info!("[Audio] Loading track: {}", track);
                        let mut audio = audio_for_track.lock().unwrap();
                        if let Err(e) = audio.load_track(track) {
                            warn!("[Audio] Failed to load track: {}", e);
                        }
                    }
                }
            }));
        }
    }

    let audio_for_playing = Arc::clone(&audio_backend);
    {
        let mut core_lock = core.lock().unwrap();
        if let Some(prop) = core_lock.properties.get_mut("playing") {
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
    }

    let audio_for_volume = Arc::clone(&audio_backend);
    {
        let mut core_lock = core.lock().unwrap();
        if let Some(prop) = core_lock.properties.get_mut("volume") {
            prop.subscribe(Arc::new(move |value, _core| {
                if let Some(vol) = value.as_float() {
                    let mut audio = audio_for_volume.lock().unwrap();
                    audio.set_volume(vol);
                }
            }));
        }
    }

    {
        let mut core_lock = core.lock().unwrap();
        register_commands(&mut *core_lock);
    }

    {
        let mut core_lock = core.lock().unwrap();
        core_lock.subscribe_event(Arc::new(|event, _core| match event {
            EventType::PropertyChanged(name) => {
                if name != "playing" {
                    info!("[Core] Property '{}' changed", name);
                }
            }
            EventType::CommandExecuted(name) => {
                info!("[Core] Command '{}' executed", name);
            }
        }));
    }

    println!("\nInitialization complete!\n");

    let mut repl = Repl::new(db);
    {
        let mut core_lock = core.lock().unwrap();
        repl.run(&mut *core_lock)?;
    }

    Ok(())
}
