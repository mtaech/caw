/// SQLite persistence for playlists (and future: library cache, play counts).
use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::{params, Connection, Result as SqlResult};

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
    /// Open (or create) the database at `~/.local/share/caw/caw.db`.
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
            ",
        )
        .expect("caw: database migration failed");
    }

    // ── CRUD ──

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
        let track_ids: Vec<i64> = stmt
            .query_map(params![id], |row| row.get::<_, i64>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        Some(PlaylistWithTracks { id, name, track_ids })
    }

    pub fn create_playlist(&self, name: &str) -> SqlResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO playlists (name) VALUES (?1)", params![name])?;
        Ok(conn.last_insert_rowid())
    }

    pub fn rename_playlist(&self, id: i64, name: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE playlists SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        Ok(())
    }

    pub fn delete_playlist(&self, id: i64) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM playlist_tracks WHERE playlist_id = ?1", params![id])?;
        conn.execute("DELETE FROM playlists WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn add_tracks(&self, playlist_id: i64, track_ids: &[i64]) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        // Get current max position
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

    pub fn remove_tracks(&self, playlist_id: i64, track_ids: &[i64]) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        for tid in track_ids {
            conn.execute(
                "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
                params![playlist_id, tid],
            )?;
        }
        // Re-index positions
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

    pub fn reorder(&self, playlist_id: i64, track_id: i64, new_pos: i64) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        // Get current position of the track
        let curr_pos: Option<i64> = conn
            .query_row(
                "SELECT position FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
                params![playlist_id, track_id],
                |row| row.get(0),
            )
            .ok();

        let curr_pos = match curr_pos {
            Some(p) => p,
            None => return Ok(()),
        };

        if curr_pos == new_pos {
            return Ok(());
        }

        // Shift other tracks
        if new_pos < curr_pos {
            conn.execute(
                "UPDATE playlist_tracks SET position = position + 1
                 WHERE playlist_id = ?1 AND position >= ?2 AND position < ?3",
                params![playlist_id, new_pos, curr_pos],
            )?;
        } else {
            conn.execute(
                "UPDATE playlist_tracks SET position = position - 1
                 WHERE playlist_id = ?1 AND position > ?2 AND position <= ?3",
                params![playlist_id, curr_pos, new_pos],
            )?;
        }

        conn.execute(
            "UPDATE playlist_tracks SET position = ?1 WHERE playlist_id = ?2 AND track_id = ?3",
            params![new_pos, playlist_id, track_id],
        )?;

        Ok(())
    }
}

fn db_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("caw").join("caw.db")
}
