use crate::error::Result;
use crate::models::{
    CountryTotal, Kpis, LoaderCell, Overview, Platform, ProjectSummary, RevenuePoint, TimelinePoint,
};
use crate::store::metrics::{freshness, recent_events, snapshot_day_count, snapshot_deltas};
use crate::store::projects::{links, list};
use chrono::NaiveDate;
use rusqlite::{params, Connection};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

/// Ajoute `days` jours à une date `YYYY-MM-DD`. Renvoie la date inchangée si elle est invalide.
pub fn shift_day(day: &str, days: i64) -> String {
    NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.checked_add_signed(chrono::Duration::days(days)))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| day.to_string())
}

pub fn timeline(conn: &Connection, from: &str, to: &str) -> Result<Vec<TimelinePoint>> {
    let mut per_day: BTreeMap<String, (i64, i64)> = BTreeMap::new();

    let mut stmt = conn.prepare(
        "SELECT day, COALESCE(SUM(downloads), 0) FROM metrics_daily
         WHERE day >= ?1 AND day < ?2 GROUP BY day",
    )?;
    for row in stmt.query_map(params![from, to], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (day, downloads) = row?;
        per_day.entry(day).or_default().0 += downloads;
    }

    for ((_, day), delta) in snapshot_deltas(conn)? {
        if day.as_str() >= from && day.as_str() < to {
            per_day.entry(day).or_default().1 += delta;
        }
    }

    Ok(per_day
        .into_iter()
        .map(|(day, (modrinth, curseforge))| TimelinePoint {
            day,
            modrinth,
            curseforge,
        })
        .collect())
}

pub fn per_project(conn: &Connection, from: &str, to: &str) -> Result<Vec<ProjectSummary>> {
    let projects = list(conn)?;
    let link_rows = links(conn)?;
    let by_id: HashMap<i64, &_> = projects.iter().map(|p| (p.id, p)).collect();

    let mut spark_by_project: HashMap<i64, BTreeMap<String, i64>> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT project_id, day, COALESCE(downloads, 0) FROM metrics_daily
         WHERE day >= ?1 AND day < ?2",
    )?;
    for row in stmt.query_map(params![from, to], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })? {
        let (project_id, day, downloads) = row?;
        *spark_by_project
            .entry(project_id)
            .or_default()
            .entry(day)
            .or_insert(0) += downloads;
    }

    let cf_deltas = snapshot_deltas(conn)?;
    let mut consumed_cf: Vec<i64> = Vec::new();
    let mut out: Vec<ProjectSummary> = Vec::new();

    for project in projects.iter().filter(|p| p.platform == Platform::Modrinth) {
        let link = link_rows
            .iter()
            .find(|l| l.modrinth_project_id == project.id);
        let cf = link.and_then(|l| by_id.get(&l.cf_project_id).copied());
        if let Some(cf) = cf {
            consumed_cf.push(cf.id);
        }
        let mut spark: BTreeMap<String, i64> = spark_by_project
            .get(&project.id)
            .cloned()
            .unwrap_or_default();
        if let Some(cf) = cf {
            for ((cf_id, day), delta) in &cf_deltas {
                if *cf_id == cf.id && day.as_str() >= from && day.as_str() < to {
                    *spark.entry(day.clone()).or_insert(0) += delta;
                }
            }
        }
        out.push(ProjectSummary {
            key: format!("m{}", project.id),
            title: project.title.clone(),
            icon_url: project
                .icon_url
                .clone()
                .or_else(|| cf.and_then(|c| c.icon_url.clone())),
            modrinth_id: Some(project.id),
            curseforge_id: cf.map(|c| c.id),
            modrinth_downloads: project.total_downloads,
            curseforge_downloads: cf.map(|c| c.total_downloads).unwrap_or(0),
            followers: project.followers,
            link_confidence: link.map(|l| l.confidence),
            spark: spark.into_values().collect(),
        });
    }

    for project in projects
        .iter()
        .filter(|p| p.platform == Platform::CurseForge && !consumed_cf.contains(&p.id))
    {
        let spark: BTreeMap<String, i64> = cf_deltas
            .iter()
            .filter(|((cf_id, day), _)| {
                *cf_id == project.id && day.as_str() >= from && day.as_str() < to
            })
            .map(|((_, day), delta)| (day.clone(), *delta))
            .collect();
        out.push(ProjectSummary {
            key: format!("c{}", project.id),
            title: project.title.clone(),
            icon_url: project.icon_url.clone(),
            modrinth_id: None,
            curseforge_id: Some(project.id),
            modrinth_downloads: 0,
            curseforge_downloads: project.total_downloads,
            followers: 0,
            link_confidence: None,
            spark: spark.into_values().collect(),
        });
    }

    out.sort_by(|a, b| {
        (b.modrinth_downloads + b.curseforge_downloads)
            .cmp(&(a.modrinth_downloads + a.curseforge_downloads))
    });
    Ok(out)
}

pub fn countries(conn: &Connection, from: &str, to: &str) -> Result<Vec<CountryTotal>> {
    let mut stmt = conn.prepare(
        "SELECT CASE WHEN country IN ('', 'XX') THEN '??' ELSE country END AS code,
                SUM(downloads)
         FROM countries_daily WHERE day >= ?1 AND day < ?2
         GROUP BY code ORDER BY SUM(downloads) DESC",
    )?;
    let rows = stmt
        .query_map(params![from, to], |r| {
            Ok(CountryTotal {
                country: r.get(0)?,
                downloads: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn loaders(conn: &Connection) -> Result<Vec<LoaderCell>> {
    let mut stmt = conn.prepare("SELECT game_versions, loaders, downloads FROM versions")?;
    let mut totals: HashMap<(String, String), i64> = HashMap::new();
    for row in stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })? {
        let (game_versions, loader_list, downloads) = row?;
        let game_versions: Vec<String> = serde_json::from_str(&game_versions).unwrap_or_default();
        let loader_list: Vec<String> = serde_json::from_str(&loader_list).unwrap_or_default();
        for game_version in &game_versions {
            for loader in &loader_list {
                *totals
                    .entry((game_version.clone(), loader.clone()))
                    .or_insert(0) += downloads;
            }
        }
    }
    let mut cells: Vec<LoaderCell> = totals
        .into_iter()
        .map(|((game_version, loader), downloads)| LoaderCell {
            game_version,
            loader,
            downloads,
        })
        .collect();
    cells.sort_by_key(|c| std::cmp::Reverse(c.downloads));
    Ok(cells)
}

pub fn revenue(conn: &Connection, from: &str, to: &str) -> Result<Vec<RevenuePoint>> {
    let mut stmt = conn.prepare(
        "SELECT day, revenue FROM metrics_daily
         WHERE day >= ?1 AND day < ?2 AND revenue IS NOT NULL ORDER BY day",
    )?;
    let mut per_day: BTreeMap<String, Decimal> = BTreeMap::new();
    for row in stmt.query_map(params![from, to], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })? {
        let (day, amount) = row?;
        let amount = Decimal::from_str(&amount).unwrap_or_default();
        *per_day.entry(day).or_default() += amount;
    }
    Ok(per_day
        .into_iter()
        .map(|(day, amount)| RevenuePoint {
            day,
            amount: amount.to_string(),
        })
        .collect())
}

pub fn kpis(conn: &Connection, today: &str) -> Result<Kpis> {
    let window_start = shift_day(today, -30);
    let previous_start = shift_day(today, -60);

    let sum_downloads = |from: &str, to: &str| -> Result<i64> {
        Ok(conn.query_row(
            "SELECT COALESCE(SUM(downloads), 0) FROM metrics_daily WHERE day >= ?1 AND day < ?2",
            params![from, to],
            |r| r.get(0),
        )?)
    };

    let per_platform = |platform: Platform| -> Result<i64> {
        Ok(conn.query_row(
            "SELECT COALESCE(SUM(total_downloads), 0) FROM projects
             WHERE platform = ?1 AND archived_at IS NULL",
            params![platform.as_str()],
            |r| r.get(0),
        )?)
    };

    let mut stmt = conn.prepare("SELECT revenue FROM metrics_daily WHERE revenue IS NOT NULL")?;
    let mut revenue_total = Decimal::ZERO;
    for row in stmt.query_map([], |r| r.get::<_, String>(0))? {
        revenue_total += Decimal::from_str(&row?).unwrap_or_default();
    }

    let downloads_modrinth = per_platform(Platform::Modrinth)?;
    let downloads_curseforge = per_platform(Platform::CurseForge)?;

    Ok(Kpis {
        downloads_total: downloads_modrinth + downloads_curseforge,
        downloads_modrinth,
        downloads_curseforge,
        downloads_30d: sum_downloads(&window_start, today)?,
        downloads_prev_30d: sum_downloads(&previous_start, &window_start)?,
        revenue_total: revenue_total.normalize().to_string(),
        revenue_pending: "0".into(),
        followers: conn.query_row(
            "SELECT COALESCE(SUM(followers), 0) FROM projects WHERE archived_at IS NULL",
            [],
            |r| r.get(0),
        )?,
        projects_active: conn.query_row(
            "SELECT COUNT(*) FROM projects WHERE archived_at IS NULL",
            [],
            |r| r.get(0),
        )?,
    })
}

pub fn overview(conn: &Connection, today: &str, range_days: i64) -> Result<Overview> {
    let from = shift_day(today, -range_days);
    let to = shift_day(today, 1);
    let mut kpis = kpis(conn, today)?;
    if let Some(balance) = crate::store::metrics::get_meta(conn, "modrinth_balance")? {
        kpis.revenue_pending = balance;
    }

    Ok(Overview {
        kpis,
        timeline: timeline(conn, &from, &to)?,
        per_project: per_project(conn, &from, &to)?,
        countries: countries(conn, &from, &to)?,
        loaders: loaders(conn)?,
        revenue: revenue(conn, &from, &to)?,
        events: recent_events(conn, 40)?,
        freshness: freshness(conn)?,
        curseforge_history_days: snapshot_day_count(conn)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::metrics::{insert_snapshot, upsert_country, upsert_daily, upsert_version};
    use crate::store::projects::{upsert, upsert_link, ProjectUpsert};
    use crate::store::schema::migrate;
    use rusqlite::Connection;

    fn seed() -> (Connection, i64, i64) {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let mk = |platform, ext: &str, title: &str, total| ProjectUpsert {
            platform,
            ext_id: ext.into(),
            slug: Some(title.to_lowercase().replace(' ', "-")),
            title: title.into(),
            project_type: Some("mod".into()),
            url: None,
            icon_url: None,
            created_at: None,
            total_downloads: total,
            followers: 5,
        };
        let m = upsert(&conn, &mk(Platform::Modrinth, "m1", "Mobs Blocker", 23_225)).unwrap();
        let c = upsert(
            &conn,
            &mk(Platform::CurseForge, "1002185", "Mobs Blocker", 86_753),
        )
        .unwrap();
        upsert_link(&conn, m, c, 1.0, false).unwrap();
        (conn, m, c)
    }

    #[test]
    fn timeline_merges_modrinth_series_and_curseforge_deltas() {
        let (conn, m, c) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(40), None, None).unwrap();
        insert_snapshot(&conn, c, "2026-08-09T00:00:00Z", 100, None).unwrap();
        insert_snapshot(&conn, c, "2026-08-10T00:00:00Z", 175, None).unwrap();
        let points = timeline(&conn, "2026-08-01", "2026-08-11").unwrap();
        let day = points.iter().find(|p| p.day == "2026-08-10").unwrap();
        assert_eq!(day.modrinth, 40);
        assert_eq!(day.curseforge, 75);
    }

    #[test]
    fn per_project_groups_linked_projects_under_one_row() {
        let (conn, _, _) = seed();
        let rows = per_project(&conn, "2026-08-01", "2026-08-11").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].modrinth_downloads, 23_225);
        assert_eq!(rows[0].curseforge_downloads, 86_753);
        assert_eq!(rows[0].title, "Mobs Blocker");
    }

    #[test]
    fn unlinked_projects_appear_alone() {
        let (conn, _, _) = seed();
        upsert(
            &conn,
            &ProjectUpsert {
                platform: Platform::Modrinth,
                ext_id: "solo".into(),
                slug: Some("solo".into()),
                title: "Solo".into(),
                project_type: None,
                url: None,
                icon_url: None,
                created_at: None,
                total_downloads: 7,
                followers: 0,
            },
        )
        .unwrap();
        let rows = per_project(&conn, "2026-08-01", "2026-08-11").unwrap();
        assert_eq!(rows.len(), 2);
        let solo = rows.iter().find(|r| r.title == "Solo").unwrap();
        assert_eq!(solo.curseforge_downloads, 0);
        assert!(solo.curseforge_id.is_none());
    }

    #[test]
    fn countries_separate_unknown_from_real_codes() {
        let (conn, m, _) = seed();
        upsert_country(&conn, m, "2026-08-10", "DE", 88).unwrap();
        upsert_country(&conn, m, "2026-08-10", "XX", 558).unwrap();
        upsert_country(&conn, m, "2026-08-10", "", 454).unwrap();
        let rows = countries(&conn, "2026-08-01", "2026-08-11").unwrap();
        let unknown = rows.iter().find(|r| r.country == "??").unwrap();
        assert_eq!(
            unknown.downloads, 1012,
            "XX et la chaine vide sont fusionnes en ??"
        );
        assert!(rows.iter().any(|r| r.country == "DE" && r.downloads == 88));
    }

    #[test]
    fn loaders_expand_game_version_and_loader_pairs() {
        let (conn, m, _) = seed();
        upsert_version(
            &conn,
            m,
            "v1",
            Some("1.0"),
            &["1.20.1".into(), "1.21".into()],
            &["fabric".into()],
            30,
            None,
        )
        .unwrap();
        upsert_version(
            &conn,
            m,
            "v2",
            Some("1.1"),
            &["1.21".into()],
            &["fabric".into(), "neoforge".into()],
            10,
            None,
        )
        .unwrap();
        let cells = loaders(&conn).unwrap();
        let fabric_121 = cells
            .iter()
            .find(|c| c.game_version == "1.21" && c.loader == "fabric")
            .unwrap();
        assert_eq!(fabric_121.downloads, 40);
        assert!(cells
            .iter()
            .any(|c| c.loader == "neoforge" && c.downloads == 10));
    }

    #[test]
    fn kpis_compare_the_two_last_thirty_day_windows() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, Some("1.5")).unwrap();
        upsert_daily(&conn, m, "2026-06-20", Some(40), None, Some("0.5")).unwrap();
        let k = kpis(&conn, "2026-08-11").unwrap();
        assert_eq!(k.downloads_30d, 100);
        assert_eq!(k.downloads_prev_30d, 40);
        assert_eq!(k.revenue_total, "2");
        assert_eq!(k.downloads_modrinth, 23_225);
        assert_eq!(k.downloads_curseforge, 86_753);
    }
}
