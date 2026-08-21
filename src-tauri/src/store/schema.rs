use crate::error::Result;
use rusqlite::Connection;

pub const SCHEMA_VERSION: i64 = 7;

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

/// `solo` marque un projet qui n'existe pas sur l'autre plateforme. Sans lui,
/// un mod publié sur un seul site reste éternellement dans la liste des
/// appariements à faire, alors qu'il n'y a rien à apparier.
const V2: &str = r#"
ALTER TABLE projects ADD COLUMN solo INTEGER NOT NULL DEFAULT 0;
"#;

/// Relevés manuels du solde de points CurseForge.
///
/// Leur programme de rémunération n'expose aucune interface : le solde ne se lit
/// que sur le tableau de bord auteur. L'utilisateur le recopie ici, un relevé
/// par jour, et l'application en tire une courbe.
const V3: &str = r#"
CREATE TABLE cf_points (
  day TEXT PRIMARY KEY,
  points INTEGER NOT NULL,
  recorded_at TEXT NOT NULL
);
"#;

/// Revenus mensuels CurseForge, en dollars.
///
/// Le tableau de bord les sert par mois, jamais par jour, et ne remonte que sur
/// une poignée de mois : chaque collecte complète la table sans rien effacer,
/// de sorte que l'historique se conserve au-delà de ce que le site montre.
const V4: &str = r#"
CREATE TABLE cf_revenue (
  month TEXT PRIMARY KEY,
  amount_usd TEXT NOT NULL,
  recorded_at TEXT NOT NULL
);
"#;

/// Abonnés CurseForge, relevés sur la fiche publique du compte.
///
/// La plateforme ne dit pas depuis quand chacun suit : elle les classe du plus
/// récent au plus ancien, sans date. On garde donc le jour où chacun est apparu
/// et celui où on l'a vu pour la dernière fois, ce que le site ne dira jamais,
/// mais que l'application peut constater d'un relevé à l'autre. Un abonné parti
/// garde sa ligne : `lost_on` note le jour où il a cessé de figurer.
const V5: &str = r#"
CREATE TABLE cf_followers (
  name TEXT PRIMARY KEY,
  avatar_url TEXT,
  seniority TEXT,
  first_seen TEXT NOT NULL,
  last_seen TEXT NOT NULL,
  lost_on TEXT,
  rank INTEGER NOT NULL DEFAULT 0
);
"#;

/// Nombre d'abonnés relevé jour par jour, plateforme par plateforme.
///
/// Ni Modrinth ni CurseForge ne tiennent d'historique : ils annoncent un
/// compte, celui de l'instant. La courbe se construit donc ici, un relevé par
/// jour, et ne remonte pas plus loin que le premier d'entre eux.
const V6: &str = r#"
CREATE TABLE followers_daily (
  day TEXT NOT NULL,
  platform TEXT NOT NULL,
  count INTEGER NOT NULL,
  PRIMARY KEY(day, platform)
);
"#;

/// Efface le compte de sa propre liste d'abonnés.
///
/// Le premier relevé partait des liens vers `/members/`, et celui de l'en-tête
/// mène au compte lui-même : il se comptait donc parmi ceux qui le suivent. Le
/// relevé l'écarte désormais, mais les bases déjà remplies gardent la ligne, et
/// aucune d'elles ne serait corrigée avant le prochain passage.
const V7: &str = r#"
DELETE FROM cf_followers WHERE LOWER(name) IN (
  SELECT LOWER(value) FROM meta
  WHERE key IN ('curseforge_account', 'curseforge_author', 'curseforge_username')
);
"#;

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

    if current < 1 {
        conn.execute_batch(V1)?;
    }

    if current < 2 {
        conn.execute_batch(V2)?;
    }

    if current < 3 {
        conn.execute_batch(V3)?;
    }

    if current < 4 {
        conn.execute_batch(V4)?;
    }

    if current < 5 {
        conn.execute_batch(V5)?;
    }

    if current < 6 {
        conn.execute_batch(V6)?;
    }

    if current < 7 {
        conn.execute_batch(V7)?;
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
            "cf_followers",
            "cf_points",
            "cf_revenue",
            "cf_snapshots",
            "countries_daily",
            "events",
            "followers_daily",
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
    fn migrate_adds_the_solo_column_to_an_existing_v1_base() {
        let conn = Connection::open_in_memory().unwrap();
        // Base au format d'origine, telle qu'installée chez un utilisateur.
        conn.execute_batch(V1).unwrap();
        conn.execute_batch("PRAGMA user_version = 1;").unwrap();

        migrate(&conn).unwrap();

        let solo: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name = 'solo'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(solo, 1, "la colonne solo doit être ajoutée");
        assert_eq!(version(&conn), SCHEMA_VERSION);
    }

    #[test]
    fn migrate_sets_user_version() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        assert_eq!(version(&conn), SCHEMA_VERSION);
    }
}
