use crate::error::Result;
use rusqlite::Connection;

pub const SCHEMA_VERSION: i64 = 1;

const V1: &str = r#"
CREATE TABLE projects (
  id INTEGER PRIMARY KEY,
  platform TEXT NOT NULL,
  ext_id TEXT NOT NULL,
  slug TEXT,
  title TEXT NOT NULL,
  project_type TEXT,
  url TEXT,
  icon_url TEXT,
  created_at TEXT,
  total_downloads INTEGER NOT NULL DEFAULT 0,
  followers INTEGER NOT NULL DEFAULT 0,
  archived_at TEXT,
  UNIQUE(platform, ext_id)
);

CREATE TABLE links (
  modrinth_project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  cf_project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  confidence REAL NOT NULL,
  manual INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(modrinth_project_id, cf_project_id)
);

CREATE TABLE metrics_daily (
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  day TEXT NOT NULL,
  downloads INTEGER,
  views INTEGER,
  revenue TEXT,
  PRIMARY KEY(project_id, day)
);

CREATE TABLE countries_daily (
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  day TEXT NOT NULL,
  country TEXT NOT NULL,
  downloads INTEGER NOT NULL,
  PRIMARY KEY(project_id, day, country)
);

CREATE TABLE cf_snapshots (
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  taken_at TEXT NOT NULL,
  total_downloads INTEGER NOT NULL,
  monthly_downloads INTEGER,
  PRIMARY KEY(project_id, taken_at)
);

CREATE TABLE versions (
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  version_id TEXT NOT NULL,
  version_number TEXT,
  game_versions TEXT,
  loaders TEXT,
  downloads INTEGER NOT NULL DEFAULT 0,
  date_published TEXT,
  PRIMARY KEY(project_id, version_id)
);

CREATE TABLE events (
  id INTEGER PRIMARY KEY,
  source TEXT NOT NULL,
  occurred_at TEXT NOT NULL,
  kind TEXT NOT NULL,
  project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
  title TEXT NOT NULL,
  detail TEXT NOT NULL DEFAULT '',
  UNIQUE(source, occurred_at, kind, title)
);

CREATE TABLE sync_runs (
  id INTEGER PRIMARY KEY,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  provider TEXT NOT NULL,
  status TEXT NOT NULL,
  detail TEXT NOT NULL DEFAULT ''
);

CREATE TABLE meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE INDEX idx_metrics_day ON metrics_daily(day);
CREATE INDEX idx_countries_day ON countries_daily(day);
CREATE INDEX idx_snapshots_taken ON cf_snapshots(taken_at);
CREATE INDEX idx_events_occurred ON events(occurred_at DESC);
"#;

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

    if current < 1 {
        conn.execute_batch(V1)?;
    }

    if current < SCHEMA_VERSION {
        conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn version(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn migrate_creates_all_tables() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        for expected in [
            "cf_snapshots",
            "countries_daily",
            "events",
            "links",
            "meta",
            "metrics_daily",
            "projects",
            "sync_runs",
            "versions",
        ] {
            assert!(
                names.contains(&expected.to_string()),
                "table manquante : {expected}"
            );
        }
    }

    #[test]
    fn migrate_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let first = version(&conn);
        migrate(&conn).unwrap();
        assert_eq!(version(&conn), first);
    }

    #[test]
    fn migrate_sets_user_version() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        assert_eq!(version(&conn), SCHEMA_VERSION);
    }
}
