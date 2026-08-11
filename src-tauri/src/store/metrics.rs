use crate::error::Result;
use crate::models::Freshness;
use rusqlite::{params, Connection};
use std::collections::HashMap;

/// Valeur d'un point du programme de rémunération CurseForge, telle qu'annoncée
/// dans leur foire aux questions : 0,05 $ US. C'est une constante déclarée par
/// la plateforme, pas une estimation de notre part.
pub const CF_POINT_VALUE_USD: f64 = 0.05;

/// Un relevé manuel du solde de points CurseForge.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CfPointEntry {
    pub day: String,
    pub points: i64,
    /// Contre-valeur en dollars au tarif annoncé par CurseForge.
    pub value_usd: String,
}

/// Enregistre le solde relevé un jour donné. Un second relevé le même jour
/// remplace le premier : c'est une correction de saisie, pas un cumul.
pub fn record_cf_points(conn: &Connection, day: &str, points: i64, now: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO cf_points (day, points, recorded_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(day) DO UPDATE SET points = excluded.points, recorded_at = excluded.recorded_at",
        params![day, points.max(0), now],
    )?;
    Ok(())
}

pub fn delete_cf_points(conn: &Connection, day: &str) -> Result<usize> {
    Ok(conn.execute("DELETE FROM cf_points WHERE day = ?1", params![day])?)
}

pub fn cf_points(conn: &Connection) -> Result<Vec<CfPointEntry>> {
    let mut stmt = conn.prepare("SELECT day, points FROM cf_points ORDER BY day")?;
    let rows = stmt
        .query_map([], |r| {
            let points: i64 = r.get(1)?;
            Ok(CfPointEntry {
                day: r.get(0)?,
                points,
                value_usd: format!("{:.2}", points as f64 * CF_POINT_VALUE_USD),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[derive(Debug, Clone)]
pub struct DailyRow {
    pub project_id: i64,
    pub day: String,
    pub downloads: Option<i64>,
    pub views: Option<i64>,
    pub revenue: Option<String>,
}

pub fn upsert_daily(
    conn: &Connection,
    project_id: i64,
    day: &str,
    downloads: Option<i64>,
    views: Option<i64>,
    revenue: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO metrics_daily (project_id, day, downloads, views, revenue)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(project_id, day) DO UPDATE SET
           downloads = COALESCE(excluded.downloads, metrics_daily.downloads),
           views = COALESCE(excluded.views, metrics_daily.views),
           revenue = COALESCE(excluded.revenue, metrics_daily.revenue)",
        params![project_id, day, downloads, views, revenue],
    )?;
    Ok(())
}

pub fn daily_range(conn: &Connection, from: &str, to: &str) -> Result<Vec<DailyRow>> {
    let mut stmt = conn.prepare(
        "SELECT project_id, day, downloads, views, revenue
         FROM metrics_daily WHERE day >= ?1 AND day < ?2 ORDER BY day",
    )?;
    let rows = stmt
        .query_map(params![from, to], |r| {
            Ok(DailyRow {
                project_id: r.get(0)?,
                day: r.get(1)?,
                downloads: r.get(2)?,
                views: r.get(3)?,
                revenue: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn last_metrics_day(conn: &Connection) -> Result<Option<String>> {
    Ok(conn.query_row("SELECT MAX(day) FROM metrics_daily", [], |r| r.get(0))?)
}

pub fn upsert_country(
    conn: &Connection,
    project_id: i64,
    day: &str,
    country: &str,
    downloads: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO countries_daily (project_id, day, country, downloads)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(project_id, day, country) DO UPDATE SET downloads = excluded.downloads",
        params![project_id, day, country, downloads],
    )?;
    Ok(())
}

pub fn insert_snapshot(
    conn: &Connection,
    project_id: i64,
    taken_at: &str,
    total: i64,
    monthly: Option<i64>,
) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO cf_snapshots (project_id, taken_at, total_downloads, monthly_downloads)
         VALUES (?1, ?2, ?3, ?4)",
        params![project_id, taken_at, total, monthly],
    )?;
    Ok(())
}

/// Delta quotidien par projet CurseForge, borné à zéro.
/// La clé est (project_id, jour du snapshot courant).
pub fn snapshot_deltas(conn: &Connection) -> Result<HashMap<(i64, String), i64>> {
    let mut stmt = conn.prepare(
        "SELECT project_id, substr(taken_at, 1, 10) AS day, MAX(total_downloads)
         FROM cf_snapshots GROUP BY project_id, day ORDER BY project_id, day",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut out = HashMap::new();
    let mut previous: Option<(i64, i64)> = None;
    for (project_id, day, total) in rows {
        if let Some((prev_project, prev_total)) = previous {
            if prev_project == project_id {
                out.insert((project_id, day.clone()), (total - prev_total).max(0));
            }
        }
        previous = Some((project_id, total));
    }
    Ok(out)
}

pub fn snapshot_day_count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(DISTINCT substr(taken_at, 1, 10)) FROM cf_snapshots",
        [],
        |r| r.get(0),
    )?)
}

#[allow(clippy::too_many_arguments)]
pub fn upsert_version(
    conn: &Connection,
    project_id: i64,
    version_id: &str,
    version_number: Option<&str>,
    game_versions: &[String],
    loaders: &[String],
    downloads: i64,
    date_published: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO versions (project_id, version_id, version_number, game_versions, loaders, downloads, date_published)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(project_id, version_id) DO UPDATE SET
           version_number = excluded.version_number,
           game_versions = excluded.game_versions,
           loaders = excluded.loaders,
           downloads = excluded.downloads,
           date_published = excluded.date_published",
        params![
            project_id,
            version_id,
            version_number,
            serde_json::to_string(game_versions)?,
            serde_json::to_string(loaders)?,
            downloads,
            date_published
        ],
    )?;
    Ok(())
}

pub fn insert_event(
    conn: &Connection,
    source: &str,
    occurred_at: &str,
    kind: &str,
    project_id: Option<i64>,
    title: &str,
    detail: &str,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO events (source, occurred_at, kind, project_id, title, detail)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![source, occurred_at, kind, project_id, title, detail],
    )?;
    Ok(())
}

pub fn recent_events(conn: &Connection, limit: i64) -> Result<Vec<crate::models::EventRow>> {
    let mut stmt = conn.prepare(
        "SELECT occurred_at, kind, title, detail FROM events ORDER BY occurred_at DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![limit], |r| {
            Ok(crate::models::EventRow {
                occurred_at: r.get(0)?,
                kind: r.get(1)?,
                title: r.get(2)?,
                detail: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn start_sync_run(conn: &Connection, provider: &str, started_at: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO sync_runs (started_at, provider, status) VALUES (?1, ?2, 'running')",
        params![started_at, provider],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn finish_sync_run(
    conn: &Connection,
    id: i64,
    finished_at: &str,
    status: &str,
    detail: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE sync_runs SET finished_at = ?1, status = ?2, detail = ?3 WHERE id = ?4",
        params![finished_at, status, detail, id],
    )?;
    Ok(())
}

/// Dernier cycle terminé par provider.
pub fn freshness(conn: &Connection) -> Result<Vec<Freshness>> {
    let mut stmt = conn.prepare(
        "SELECT provider, status, finished_at, detail FROM sync_runs r
         WHERE r.id = (SELECT MAX(id) FROM sync_runs WHERE provider = r.provider)
         ORDER BY provider",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Freshness {
                provider: r.get(0)?,
                status: r.get(1)?,
                finished_at: r.get(2)?,
                detail: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[cfg(test)]
mod point_tests {
    use super::*;
    use crate::store::schema::migrate;

    fn base() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    #[test]
    fn a_second_reading_the_same_day_corrects_the_first() {
        let conn = base();
        record_cf_points(&conn, "2026-08-11", 120, "2026-08-11T10:00:00Z").unwrap();
        record_cf_points(&conn, "2026-08-11", 132, "2026-08-11T18:00:00Z").unwrap();

        let entries = cf_points(&conn).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].points, 132);
    }

    #[test]
    fn points_convert_at_the_rate_curseforge_publishes() {
        let conn = base();
        record_cf_points(&conn, "2026-08-11", 132, "2026-08-11T10:00:00Z").unwrap();
        // 132 points × 0,05 $
        assert_eq!(cf_points(&conn).unwrap()[0].value_usd, "6.60");
    }

    #[test]
    fn readings_come_back_in_chronological_order() {
        let conn = base();
        record_cf_points(&conn, "2026-08-11", 3, "x").unwrap();
        record_cf_points(&conn, "2026-06-01", 1, "x").unwrap();
        record_cf_points(&conn, "2026-07-01", 2, "x").unwrap();

        let days: Vec<String> = cf_points(&conn).unwrap().into_iter().map(|e| e.day).collect();
        assert_eq!(days, ["2026-06-01", "2026-07-01", "2026-08-11"]);
    }

    #[test]
    fn a_negative_reading_is_clamped_rather_than_stored() {
        let conn = base();
        record_cf_points(&conn, "2026-08-11", -40, "x").unwrap();
        assert_eq!(cf_points(&conn).unwrap()[0].points, 0);
    }

    #[test]
    fn forgetting_a_reading_removes_it() {
        let conn = base();
        record_cf_points(&conn, "2026-08-11", 10, "x").unwrap();
        assert_eq!(delete_cf_points(&conn, "2026-08-11").unwrap(), 1);
        assert!(cf_points(&conn).unwrap().is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Platform;
    use crate::store::projects::{upsert, ProjectUpsert};
    use crate::store::schema::migrate;
    use rusqlite::Connection;

    fn db() -> (Connection, i64) {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let id = upsert(
            &conn,
            &ProjectUpsert {
                platform: Platform::Modrinth,
                ext_id: "abc".into(),
                slug: Some("abc".into()),
                title: "ABC".into(),
                project_type: Some("mod".into()),
                url: None,
                icon_url: None,
                created_at: None,
                total_downloads: 0,
                followers: 0,
            },
        )
        .unwrap();
        (conn, id)
    }

    #[test]
    fn upsert_daily_is_idempotent_and_updates() {
        let (conn, id) = db();
        upsert_daily(&conn, id, "2026-08-01", Some(10), Some(3), None).unwrap();
        upsert_daily(&conn, id, "2026-08-01", Some(12), None, Some("0.5")).unwrap();
        let rows = daily_range(&conn, "2026-08-01", "2026-08-02").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].downloads, Some(12));
        assert_eq!(
            rows[0].views,
            Some(3),
            "une valeur absente ne doit pas effacer l'existante"
        );
        assert_eq!(rows[0].revenue.as_deref(), Some("0.5"));
    }

    #[test]
    fn last_day_returns_none_on_empty_then_max_day() {
        let (conn, id) = db();
        assert_eq!(last_metrics_day(&conn).unwrap(), None);
        upsert_daily(&conn, id, "2026-07-30", Some(1), None, None).unwrap();
        upsert_daily(&conn, id, "2026-08-02", Some(1), None, None).unwrap();
        assert_eq!(
            last_metrics_day(&conn).unwrap().as_deref(),
            Some("2026-08-02")
        );
    }

    #[test]
    fn snapshot_deltas_never_go_negative() {
        let (conn, id) = db();
        insert_snapshot(&conn, id, "2026-08-01T00:00:00Z", 100, Some(10)).unwrap();
        insert_snapshot(&conn, id, "2026-08-02T00:00:00Z", 150, Some(10)).unwrap();
        insert_snapshot(&conn, id, "2026-08-03T00:00:00Z", 140, Some(10)).unwrap();
        let d = snapshot_deltas(&conn).unwrap();
        assert_eq!(d.get(&(id, "2026-08-02".to_string())).copied(), Some(50));
        assert_eq!(d.get(&(id, "2026-08-03".to_string())).copied(), Some(0));
        assert_eq!(
            d.get(&(id, "2026-08-01".to_string())),
            None,
            "le premier snapshot n'a pas de delta"
        );
    }

    #[test]
    fn events_are_deduplicated() {
        let (conn, id) = db();
        insert_event(
            &conn,
            "modrinth",
            "2026-08-01T00:00:00Z",
            "status_change",
            Some(id),
            "ABC",
            "approuve",
        )
        .unwrap();
        insert_event(
            &conn,
            "modrinth",
            "2026-08-01T00:00:00Z",
            "status_change",
            Some(id),
            "ABC",
            "approuve",
        )
        .unwrap();
        assert_eq!(recent_events(&conn, 10).unwrap().len(), 1);
    }

    #[test]
    fn meta_reads_back_what_was_written() {
        let (conn, _) = db();
        assert_eq!(get_meta(&conn, "balance").unwrap(), None);
        set_meta(&conn, "balance", "12.34").unwrap();
        set_meta(&conn, "balance", "56.78").unwrap();
        assert_eq!(
            get_meta(&conn, "balance").unwrap().as_deref(),
            Some("56.78")
        );
    }

    #[test]
    fn sync_run_records_start_then_finish() {
        let (conn, _) = db();
        let run = start_sync_run(&conn, "modrinth", "2026-08-11T10:00:00Z").unwrap();
        finish_sync_run(&conn, run, "2026-08-11T10:00:05Z", "ok", "3 projets").unwrap();
        let f = freshness(&conn).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].status, "ok");
        assert_eq!(f[0].finished_at.as_deref(), Some("2026-08-11T10:00:05Z"));
    }
}
