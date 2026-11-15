use std::path::Path;

use exp_core::SessionSummary;
use rusqlite::{Connection, params};
use serde::Serialize;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sessions (
    id            INTEGER PRIMARY KEY,
    started_at_ms INTEGER NOT NULL,
    ended_at_ms   INTEGER NOT NULL,
    active_secs   REAL    NOT NULL,
    character     TEXT,
    start_level   INTEGER,
    end_level     INTEGER,
    exp_gained    INTEGER NOT NULL,
    levels_gained INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS sessions_started ON sessions (started_at_ms DESC);
";

/// A persisted session row.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoredSession {
    pub id: i64,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub active_secs: f64,
    pub character: Option<String>,
    pub start_level: Option<u32>,
    pub end_level: Option<u32>,
    pub exp_gained: u64,
    pub levels_gained: u32,
}

/// Local SQLite session history.
pub struct Db(Connection);

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(conn)
    }

    #[cfg(test)]
    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute_batch(SCHEMA)?;
        Ok(Self(conn))
    }

    pub fn insert(&self, started_at_ms: i64, ended_at_ms: i64, s: &SessionSummary) -> rusqlite::Result<i64> {
        self.0.execute(
            "INSERT INTO sessions (started_at_ms, ended_at_ms, active_secs, character, start_level, end_level, exp_gained, levels_gained)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                started_at_ms,
                ended_at_ms,
                s.active_secs,
                s.character,
                s.start_level,
                s.end_level,
                s.exp_gained as i64,
                s.levels_gained,
            ],
        )?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn recent(&self, limit: u32) -> rusqlite::Result<Vec<StoredSession>> {
        let mut stmt = self.0.prepare_cached(
            "SELECT id, started_at_ms, ended_at_ms, active_secs, character, start_level, end_level, exp_gained, levels_gained
             FROM sessions ORDER BY started_at_ms DESC LIMIT ?1",
        )?;
        stmt.query_map([limit], |r| {
            Ok(StoredSession {
                id: r.get(0)?,
                started_at_ms: r.get(1)?,
                ended_at_ms: r.get(2)?,
                active_secs: r.get(3)?,
                character: r.get(4)?,
                start_level: r.get(5)?,
                end_level: r.get(6)?,
                exp_gained: r.get::<_, i64>(7)? as u64,
                levels_gained: r.get(8)?,
            })
        })?
        .collect()
    }

    /// Best EXP per second among sessions of at least `min_secs` that covered `level`.
    pub fn best_rate(&self, level: u32, character: Option<&str>, min_secs: f64) -> rusqlite::Result<Option<f64>> {
        self.0.query_row(
            "SELECT MAX(exp_gained / active_secs) FROM sessions
             WHERE active_secs >= ?3 AND start_level IS NOT NULL
               AND ?1 BETWEEN start_level AND COALESCE(end_level, start_level)
               AND (?2 IS NULL OR character = ?2)",
            params![level, character, min_secs],
            |r| r.get(0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_list_newest_first() {
        let db = Db::in_memory().unwrap();
        let summary = |gained| SessionSummary {
            character: Some("Tester".into()),
            start_level: Some(30),
            end_level: Some(31),
            exp_gained: gained,
            levels_gained: 1,
            active_secs: 60.0,
        };
        db.insert(1_000, 2_000, &summary(10)).unwrap();
        db.insert(5_000, 6_000, &summary(20)).unwrap();
        let rows = db.recent(10).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].exp_gained, 20);
        assert_eq!(rows[1].character.as_deref(), Some("Tester"));
    }

    #[test]
    fn best_rate_matches_level_character_and_length() {
        let db = Db::in_memory().unwrap();
        let row = |who: &str, from, to, gained, secs| SessionSummary {
            character: Some(who.into()),
            start_level: Some(from),
            end_level: Some(to),
            exp_gained: gained,
            levels_gained: to - from,
            active_secs: secs,
        };
        db.insert(0, 0, &row("A", 30, 31, 6_000, 600.0)).unwrap();
        db.insert(0, 0, &row("A", 31, 31, 9_000, 600.0)).unwrap();
        db.insert(0, 0, &row("A", 31, 31, 90_000, 60.0)).unwrap();
        db.insert(0, 0, &row("B", 31, 31, 60_000, 600.0)).unwrap();
        assert_eq!(db.best_rate(31, Some("A"), 300.0).unwrap(), Some(15.0));
        assert_eq!(db.best_rate(30, Some("A"), 300.0).unwrap(), Some(10.0));
        assert_eq!(db.best_rate(31, None, 300.0).unwrap(), Some(100.0));
        assert_eq!(db.best_rate(40, Some("A"), 300.0).unwrap(), None);
    }
}
