use rusqlite::{Connection, Result, params};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS playlists (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS playlist_tracks (
                id INTEGER PRIMARY KEY,
                playlist_id INTEGER,
                track_path TEXT NOT NULL,
                position INTEGER,
                FOREIGN KEY (playlist_id) REFERENCES playlists(id)
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS play_history (
                id INTEGER PRIMARY KEY,
                track_path TEXT NOT NULL,
                played_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        Ok(())
    }

    pub fn create_playlist(&self, name: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO playlists (name) VALUES (?1)",
            params![name],
        )?;
        Ok(())
    }

    pub fn delete_playlist(&self, name: &str) -> Result<()> {
        let playlist_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM playlists WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = playlist_id {
            self.conn.execute(
                "DELETE FROM playlist_tracks WHERE playlist_id = ?1",
                params![id],
            )?;
            self.conn
                .execute("DELETE FROM playlists WHERE id = ?1", params![id])?;
        }

        Ok(())
    }

    pub fn add_track_to_playlist(&self, playlist: &str, track: &str) -> Result<()> {
        self.create_playlist(playlist)?;

        let playlist_id: i64 = self.conn.query_row(
            "SELECT id FROM playlists WHERE name = ?1",
            params![playlist],
            |row| row.get(0),
        )?;

        let position: i64 = self.conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?1",
                params![playlist_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        self.conn.execute(
            "INSERT INTO playlist_tracks (playlist_id, track_path, position) VALUES (?1, ?2, ?3)",
            params![playlist_id, track, position],
        )?;

        Ok(())
    }

    pub fn remove_track_from_playlist(&self, playlist: &str, track: &str) -> Result<()> {
        let playlist_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM playlists WHERE name = ?1",
                params![playlist],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = playlist_id {
            self.conn.execute(
                "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_path = ?2",
                params![id, track],
            )?;
        }

        Ok(())
    }

    pub fn get_playlist_tracks(&self, playlist: &str) -> Result<Vec<String>> {
        let playlist_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM playlists WHERE name = ?1",
                params![playlist],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = playlist_id {
            let mut stmt = self.conn.prepare(
                "SELECT track_path FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position",
            )?;

            let tracks = stmt
                .query_map(params![id], |row| row.get(0))?
                .collect::<Result<Vec<String>>>()?;

            Ok(tracks)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn get_all_playlists(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM playlists ORDER BY name")?;
        let playlists = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>>>()?;
        Ok(playlists)
    }

    pub fn log_playback(&self, track: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO play_history (track_path) VALUES (?1)",
            params![track],
        )?;
        Ok(())
    }

    pub fn get_play_history(&self, limit: usize) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT track_path, played_at FROM play_history ORDER BY id DESC LIMIT ?1")?;

        let history = stmt
            .query_map(params![limit], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<(String, String)>>>()?;

        Ok(history)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::in_memory().unwrap();
        assert!(db.get_all_playlists().unwrap().is_empty());
    }

    #[test]
    fn test_playlist_operations() {
        let db = Database::in_memory().unwrap();

        db.create_playlist("test").unwrap();
        let playlists = db.get_all_playlists().unwrap();
        assert_eq!(playlists, vec!["test"]);

        db.add_track_to_playlist("test", "track1.mp3").unwrap();
        db.add_track_to_playlist("test", "track2.mp3").unwrap();

        let tracks = db.get_playlist_tracks("test").unwrap();
        assert_eq!(tracks, vec!["track1.mp3", "track2.mp3"]);

        db.remove_track_from_playlist("test", "track1.mp3").unwrap();
        let tracks = db.get_playlist_tracks("test").unwrap();
        assert_eq!(tracks, vec!["track2.mp3"]);

        db.delete_playlist("test").unwrap();
        assert!(db.get_all_playlists().unwrap().is_empty());
    }

    #[test]
    fn test_play_history() {
        let db = Database::in_memory().unwrap();

        db.log_playback("song1.mp3").unwrap();
        db.log_playback("song2.mp3").unwrap();

        let history = db.get_play_history(10).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].0, "song2.mp3");
        assert_eq!(history[1].0, "song1.mp3");
    }
}
