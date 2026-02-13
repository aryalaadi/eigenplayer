use crate::core::{Core, PropertyValue};
use crate::db::Database;
use std::io::{self, Write};

pub struct Repl {
    db: Database,
}

impl Repl {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn run(&mut self, core: &mut Core) -> io::Result<()> {
        println!("EigenPlayer REPL");
        println!("Type 'help' for available commands, 'quit' to exit\n");

        loop {
            print!("> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            let parts: Vec<&str> = input.split_whitespace().collect();
            let command = parts[0];
            let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

            match command {
                "quit" | "exit" | "q" => {
                    println!("Goodbye!");
                    break;
                }
                "help" | "h" => {
                    self.print_help();
                }
                "status" => {
                    self.print_status(core);
                }
                "playlist" | "pl" => {
                    self.show_playlist(core);
                }
                "playlists" => {
                    self.show_all_playlists();
                }
                "history" => {
                    self.show_history();
                }
                "play" => {
                    if args.is_empty() {
                        core.set_property("playing", PropertyValue::Bool(true));
                        println!("Resumed playback");
                    } else {
                        let track = args.join(" ");
                        core.execute_command("play", vec![track]);
                    }
                }
                "pause" => {
                    core.execute_command("pause", vec![]);
                    println!("Paused");
                }
                "stop" => {
                    core.execute_command("stop", vec![]);
                    println!("Stopped");
                }
                "next" | "n" => {
                    core.execute_command("next", vec![]);
                }
                "prev" | "p" => {
                    core.execute_command("prev", vec![]);
                }
                "add" | "a" => {
                    if args.is_empty() {
                        println!("Usage: add <track_path>");
                    } else {
                        let track = args.join(" ");
                        core.execute_command("add", vec![track.clone()]);
                        if let Err(e) = self.db.add_track_to_playlist("default", &track) {
                            eprintln!("Failed to add to database: {}", e);
                        }
                        println!("Added: {}", track);
                    }
                }
                "remove" | "rm" => {
                    if args.is_empty() {
                        println!("Usage: remove <track_path>");
                    } else {
                        let track = args.join(" ");
                        core.execute_command("remove", vec![track.clone()]);
                        if let Err(e) = self.db.remove_track_from_playlist("default", &track) {
                            eprintln!("Failed to remove from database: {}", e);
                        }
                        println!("Removed: {}", track);
                    }
                }
                "volume" | "vol" | "v" => {
                    if args.is_empty() {
                        if let Some(vol) = core.get_float("volume") {
                            println!("Volume: {:.0}%", vol * 100.0);
                        }
                    } else {
                        core.execute_command("volume", args);
                    }
                }
                "load" => {
                    if args.is_empty() {
                        println!("Usage: load <playlist_name>");
                    } else {
                        let playlist_name = &args[0];
                        match self.db.get_playlist_tracks(playlist_name) {
                            Ok(tracks) => {
                                core.set_property(
                                    "playlist",
                                    PropertyValue::StringList(tracks.clone()),
                                );
                                println!(
                                    "Loaded playlist '{}' with {} tracks",
                                    playlist_name,
                                    tracks.len()
                                );
                            }
                            Err(e) => {
                                eprintln!("Failed to load playlist: {}", e);
                            }
                        }
                    }
                }
                "save" => {
                    if args.is_empty() {
                        println!("Usage: save <playlist_name>");
                    } else {
                        let playlist_name = &args[0];
                        if let Some(tracks) = core.get_string_list("playlist") {
                            if let Err(e) = self.db.create_playlist(playlist_name) {
                                eprintln!("Failed to create playlist: {}", e);
                            } else {
                                for track in tracks {
                                    if let Err(e) =
                                        self.db.add_track_to_playlist(playlist_name, track)
                                    {
                                        eprintln!("Failed to add track: {}", e);
                                    }
                                }
                                println!(
                                    "Saved playlist '{}' with {} tracks",
                                    playlist_name,
                                    tracks.len()
                                );
                            }
                        }
                    }
                }
                _ => {
                    println!(
                        "Unknown command: '{}'. Type 'help' for available commands.",
                        command
                    );
                }
            }
        }

        Ok(())
    }

    fn print_help(&self) {
        println!("\nAvailable commands:");
        println!("  play [track]      - Play a track or resume playback");
        println!("  pause             - Pause playback");
        println!("  stop              - Stop playback");
        println!("  next (n)          - Play next track");
        println!("  prev (p)          - Play previous track");
        println!("  add (a) <track>   - Add track to current playlist");
        println!("  remove (rm) <tr>  - Remove track from playlist");
        println!("  volume (v) [0-1]  - Get or set volume");
        println!("  playlist (pl)     - Show current playlist");
        println!("  playlists         - Show all saved playlists");
        println!("  load <name>       - Load a saved playlist");
        println!("  save <name>       - Save current playlist");
        println!("  history           - Show play history");
        println!("  status            - Show player status");
        println!("  help (h)          - Show this help");
        println!("  quit (q)          - Exit\n");
    }

    fn print_status(&self, core: &Core) {
        println!("\n=== Player Status ===");

        if let Some(playing) = core.get_bool("playing") {
            println!("Playing: {}", if playing { "Yes" } else { "No" });
        }

        if let Some(track) = core.get_string("current_track") {
            println!("Current track: {}", track);
        }

        if let Some(vol) = core.get_float("volume") {
            println!("Volume: {:.0}%", vol * 100.0);
        }

        if let Some(playlist) = core.get_string_list("playlist") {
            println!("Playlist size: {} tracks", playlist.len());
        }

        println!();
    }

    fn show_playlist(&self, core: &Core) {
        if let Some(playlist) = core.get_string_list("playlist") {
            if playlist.is_empty() {
                println!("Playlist is empty");
            } else {
                println!("\n=== Current Playlist ({} tracks) ===", playlist.len());
                for (i, track) in playlist.iter().enumerate() {
                    let marker = if Some(track) == core.get_string("current_track") {
                        "▶"
                    } else {
                        " "
                    };
                    println!("{} {}. {}", marker, i + 1, track);
                }
                println!();
            }
        }
    }

    fn show_all_playlists(&self) {
        match self.db.get_all_playlists() {
            Ok(playlists) => {
                if playlists.is_empty() {
                    println!("No saved playlists");
                } else {
                    println!("\n=== Saved Playlists ===");
                    for playlist in playlists {
                        match self.db.get_playlist_tracks(&playlist) {
                            Ok(tracks) => {
                                println!("  {} ({} tracks)", playlist, tracks.len());
                            }
                            Err(_) => {
                                println!("  {}", playlist);
                            }
                        }
                    }
                    println!();
                }
            }
            Err(e) => {
                eprintln!("Failed to get playlists: {}", e);
            }
        }
    }

    fn show_history(&self) {
        match self.db.get_play_history(10) {
            Ok(history) => {
                if history.is_empty() {
                    println!("No play history");
                } else {
                    println!("\n=== Play History (last 10) ===");
                    for (track, timestamp) in history {
                        println!("  {} - {}", timestamp, track);
                    }
                    println!();
                }
            }
            Err(e) => {
                eprintln!("Failed to get history: {}", e);
            }
        }
    }
}
