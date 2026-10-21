/// SQLite persistence for playlists and settings.
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{Connection, Result as SqlResult, params};

/// A minimal playlist row from the database.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlaylistRow {
    pub id: i64,
    pub name: String,
    pub track_count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlaylistWithTracks {
    pub id: i64,
    pub name: String,
    pub track_ids: Vec<i64>,
}

/// Database wrapper — single connection behind a Mutex.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) the database at `~/.config/caw/caw.db`.
    pub fn open() -> Self {
        let path = db_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(&path).expect("caw: failed to open database");
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate();
        db
    }

    fn migrate(&self) {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS playlists (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT NOT NULL,
                created_at  TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS playlist_tracks (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                playlist_id INTEGER NOT NULL,
                track_id    INTEGER NOT NULL,
                position    INTEGER NOT NULL,
                FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS music_dirs (
                path        TEXT PRIMARY KEY
            );
            ",
        )
        .expect("caw: database migration failed");
    }

    // ── Music Dirs ──

    pub fn get_music_dirs(&self) -> Vec<PathBuf> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT path FROM music_dirs ORDER BY rowid ASC")
            .unwrap();
        let dirs: Vec<PathBuf> = stmt
            .query_map([], |row| {
                let s: String = row.get(0)?;
                Ok(PathBuf::from(s))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        dirs
    }

    pub fn add_music_dir(&self, path: &Path) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO music_dirs (path) VALUES (?1)",
            params![path.to_string_lossy().to_string()],
        )?;
        Ok(())
    }

    pub fn remove_music_dir(&self, path: &Path) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM music_dirs WHERE path = ?1",
            params![path.to_string_lossy().to_string()],
        )?;
        Ok(())
    }

    // ── CRUD Playlists ──

    pub fn list_playlists(&self) -> Vec<PlaylistRow> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT p.id, p.name, COALESCE(cnt.c, 0) AS track_count
                 FROM playlists p
                 LEFT JOIN (SELECT playlist_id, COUNT(*) AS c FROM playlist_tracks GROUP BY playlist_id) cnt
                   ON cnt.playlist_id = p.id
                 ORDER BY p.created_at ASC",
            )
            .unwrap();
        stmt.query_map([], |row| {
            Ok(PlaylistRow {
                id: row.get(0)?,
                name: row.get(1)?,
                track_count: row.get(2)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    pub fn get_playlist(&self, id: i64) -> Option<PlaylistWithTracks> {
        let conn = self.conn.lock().unwrap();
        let name: String = conn
            .query_row(
                "SELECT name FROM playlists WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok()?;

        let mut stmt = conn
            .prepare(
                "SELECT track_id FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position ASC",
            )
            .unwrap();
        let track_ids = stmt
            .query_map(params![id], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        Some(PlaylistWithTracks {
            id,
            name,
            track_ids,
        })
    }

    pub fn create_playlist(&self, name: &str) -> SqlResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO playlists (name) VALUES (?1)", params![name])?;
        Ok(conn.last_insert_rowid())
    }

    #[allow(dead_code)] // playlist rename UI lands later
    pub fn rename_playlist(&self, id: i64, new_name: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE playlists SET name = ?1 WHERE id = ?2",
            params![new_name, id],
        )?;
        Ok(())
    }

    pub fn delete_playlist(&self, id: i64) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM playlists WHERE id = ?1", params![id])?;
        conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn add_tracks(&self, playlist_id: i64, track_ids: &[i64]) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let max_pos: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), 0) FROM playlist_tracks WHERE playlist_id = ?1",
                params![playlist_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        for (i, tid) in track_ids.iter().enumerate() {
            conn.execute(
                "INSERT INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
                params![playlist_id, tid, max_pos + i as i64 + 1],
            )?;
        }
        Ok(())
    }

    #[allow(dead_code)] // called from PlayerApp::remove_track_from_playlist
    pub fn remove_tracks(&self, playlist_id: i64, track_ids: &[i64]) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        for tid in track_ids {
            conn.execute(
                "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
                params![playlist_id, tid],
            )?;
        }
        conn.execute(
            "UPDATE playlist_tracks SET position = (
                SELECT seq FROM (
                  SELECT id, ROW_NUMBER() OVER (ORDER BY position) AS seq
                  FROM playlist_tracks WHERE playlist_id = ?1
                ) AS sub WHERE sub.id = playlist_tracks.id
             ) WHERE playlist_id = ?1",
            params![playlist_id],
        )?;
        Ok(())
    }
}

fn db_path() -> PathBuf {
    crate::paths::db_path()
}
