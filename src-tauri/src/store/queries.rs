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

/// Axe de jours dense de `from` inclus à `to` exclu.
/// Les séries par projet s'alignent dessus, ce qui évite qu'un projet sans
/// téléchargement un jour donné décale toute sa courbe.
pub fn day_axis(from: &str, to: &str) -> Vec<String> {
    let (Ok(start), Ok(end)) = (
        NaiveDate::parse_from_str(from, "%Y-%m-%d"),
        NaiveDate::parse_from_str(to, "%Y-%m-%d"),
    ) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut day = start;
    while day < end {
        out.push(day.format("%Y-%m-%d").to_string());
        day = match day.succ_opt() {
            Some(next) => next,
            None => break,
        };
    }
    out
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

    let axis = day_axis(from, to);
    let densify = |series: &BTreeMap<String, i64>| -> Vec<i64> {
        axis.iter()
            .map(|day| series.get(day).copied().unwrap_or(0))
            .collect()
    };

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
            spark: densify(&spark),
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
            spark: densify(&spark),
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

/// Revenus cumulés par projet Modrinth sur la fenêtre.
pub fn revenue_by_project(
    conn: &Connection,
    from: &str,
    to: &str,
) -> Result<Vec<crate::models::RevenueByProject>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.title, m.revenue FROM metrics_daily m
         JOIN projects p ON p.id = m.project_id
         WHERE m.day >= ?1 AND m.day < ?2 AND m.revenue IS NOT NULL",
    )?;
    let mut totals: BTreeMap<(i64, String), Decimal> = BTreeMap::new();
    for row in stmt.query_map(params![from, to], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })? {
        let (id, title, amount) = row?;
        *totals.entry((id, title)).or_default() += Decimal::from_str(&amount).unwrap_or_default();
    }

    let mut out: Vec<crate::models::RevenueByProject> = totals
        .into_iter()
        .map(|((id, title), amount)| crate::models::RevenueByProject {
            key: format!("m{id}"),
            title,
            amount: amount.normalize().to_string(),
        })
        .collect();
    out.sort_by(|a, b| {
        Decimal::from_str(&b.amount)
            .unwrap_or_default()
            .cmp(&Decimal::from_str(&a.amount).unwrap_or_default())
    });
    Ok(out)
}

/// Relit l'échéancier de reversement stocké au dernier cycle et marque
/// les échéances postérieures à aujourd'hui comme revenus à venir.
pub fn payout(conn: &Connection, today: &str) -> Result<crate::models::Payout> {
    use crate::models::{Payout, PayoutPoint};
    use crate::providers::modrinth::PayoutBalance;

    let Some(raw) = crate::store::metrics::get_meta(conn, "modrinth_payout")? else {
        return Ok(Payout::default());
    };
    let balance: PayoutBalance = serde_json::from_str(&raw).unwrap_or_default();

    Ok(Payout {
        available: balance.available,
        pending: balance.pending,
        withdrawn_lifetime: balance.withdrawn_lifetime,
        withdrawn_ytd: balance.withdrawn_ytd,
        schedule: balance
            .dates
            .into_iter()
            .map(|(date, amount)| PayoutPoint {
                future: date.get(..10).is_some_and(|d| d > today),
                date,
                amount,
            })
            .collect(),
    })
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

/// Séries détaillées d'un seul projet, alignées sur l'axe dense de la fenêtre.
pub fn project_detail(
    conn: &Connection,
    from: &str,
    to: &str,
    modrinth_id: Option<i64>,
    curseforge_id: Option<i64>,
) -> Result<crate::models::ProjectDetail> {
    use crate::models::{ProjectDetail, VersionRow};

    let summary = per_project(conn, from, to)?
        .into_iter()
        .find(|p| {
            (modrinth_id.is_some() && p.modrinth_id == modrinth_id)
                || (modrinth_id.is_none() && p.curseforge_id == curseforge_id)
        })
        .ok_or_else(|| crate::error::AppError::Data("projet introuvable".into()))?;

    let axis = day_axis(from, to);
    let mut downloads: BTreeMap<String, i64> = BTreeMap::new();
    let mut views: BTreeMap<String, i64> = BTreeMap::new();
    let mut revenue: BTreeMap<String, Decimal> = BTreeMap::new();

    if let Some(id) = modrinth_id {
        let mut stmt = conn.prepare(
            "SELECT day, COALESCE(downloads, 0), COALESCE(views, 0), revenue
             FROM metrics_daily WHERE project_id = ?1 AND day >= ?2 AND day < ?3",
        )?;
        for row in stmt.query_map(params![id, from, to], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })? {
            let (day, d, v, rev) = row?;
            downloads.insert(day.clone(), d);
            views.insert(day.clone(), v);
            if let Some(rev) = rev {
                revenue.insert(day, Decimal::from_str(&rev).unwrap_or_default());
            }
        }
    }

    let mut curseforge: BTreeMap<String, i64> = BTreeMap::new();
    if let Some(id) = summary.curseforge_id {
        for ((cf_id, day), delta) in snapshot_deltas(conn)? {
            if cf_id == id && day.as_str() >= from && day.as_str() < to {
                curseforge.insert(day, delta);
            }
        }
    }

    let mut countries = Vec::new();
    if let Some(id) = modrinth_id {
        let mut stmt = conn.prepare(
            "SELECT CASE WHEN country IN ('', 'XX') THEN '??' ELSE country END AS code,
                    SUM(downloads)
             FROM countries_daily WHERE project_id = ?1 AND day >= ?2 AND day < ?3
             GROUP BY code ORDER BY SUM(downloads) DESC",
        )?;
        countries = stmt
            .query_map(params![id, from, to], |r| {
                Ok(CountryTotal {
                    country: r.get(0)?,
                    downloads: r.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
    }

    let mut versions = Vec::new();
    if let Some(id) = modrinth_id {
        let mut stmt = conn.prepare(
            "SELECT version_number, game_versions, loaders, downloads, date_published
             FROM versions WHERE project_id = ?1 ORDER BY date_published DESC",
        )?;
        versions = stmt
            .query_map(params![id], |r| {
                Ok(VersionRow {
                    version_number: r.get(0)?,
                    game_versions: serde_json::from_str(&r.get::<_, String>(1)?)
                        .unwrap_or_default(),
                    loaders: serde_json::from_str(&r.get::<_, String>(2)?).unwrap_or_default(),
                    downloads: r.get(3)?,
                    date_published: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
    }

    let pick = |m: &BTreeMap<String, i64>| -> Vec<i64> {
        axis.iter()
            .map(|d| m.get(d).copied().unwrap_or(0))
            .collect()
    };

    Ok(ProjectDetail {
        summary,
        downloads: pick(&downloads),
        views: pick(&views),
        curseforge: pick(&curseforge),
        revenue: axis
            .iter()
            .map(|d| revenue.get(d).copied().unwrap_or_default().to_string())
            .collect(),
        days: axis,
        countries,
        versions,
    })
}

/// Vrai si la chaîne est une date `YYYY-MM-DD` réelle.
fn valid_day(day: &str) -> bool {
    NaiveDate::parse_from_str(day, "%Y-%m-%d").is_ok()
}

/// Résout la fenêtre demandée en bornes internes `(début inclus, fin exclue)`.
///
/// L'interface raisonne en dates incluses : choisir « du 1er au 31 août » doit
/// contenir le 31. Les requêtes, elles, comparent avec `day < to`. La conversion
/// se fait ici, à un seul endroit. Sans bornes explicites on retombe sur la
/// fenêtre glissante de `range_days` jours qui se termine aujourd'hui.
pub fn resolve_range(
    today: &str,
    range_days: i64,
    from: Option<&str>,
    to: Option<&str>,
) -> (String, String) {
    let start = from
        .filter(|d| valid_day(d))
        .map(|d| d.to_string())
        .unwrap_or_else(|| shift_day(today, -range_days));
    let end = to
        .filter(|d| valid_day(d))
        .map(|d| shift_day(d, 1))
        .unwrap_or_else(|| shift_day(today, 1));
    if end <= start {
        let repaired = shift_day(&start, 1);
        return (start, repaired);
    }
    (start, end)
}

/// Mois `YYYY-MM` couverts par au moins une mesure, toutes sources confondues.
pub fn available_months(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT substr(day, 1, 7) AS month FROM metrics_daily
         UNION SELECT DISTINCT substr(taken_at, 1, 7) FROM cf_snapshots
         ORDER BY month",
    )?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn overview(conn: &Connection, today: &str, from: &str, to: &str) -> Result<Overview> {
    let (from, to) = (from.to_string(), to.to_string());
    let mut kpis = kpis(conn, today)?;
    let payout = payout(conn, today)?;
    if !payout.available.is_empty() {
        kpis.revenue_pending = payout.available.clone();
    }

    Ok(Overview {
        kpis,
        // La borne haute est exclue en interne : on la ramène au dernier jour
        // réellement affiché avant de la rendre à l'interface.
        to: shift_day(&to, -1),
        from: from.clone(),
        available_months: available_months(conn)?,
        days: day_axis(&from, &to),
        timeline: timeline(conn, &from, &to)?,
        per_project: per_project(conn, &from, &to)?,
        countries: countries(conn, &from, &to)?,
        loaders: loaders(conn)?,
        revenue: revenue(conn, &from, &to)?,
        revenue_by_project: revenue_by_project(conn, &from, &to)?,
        payout,
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
    fn resolve_range_falls_back_on_the_sliding_window() {
        let (from, to) = resolve_range("2026-08-11", 30, None, None);
        assert_eq!(from, "2026-07-12");
        assert_eq!(to, "2026-08-12");
    }

    #[test]
    fn resolve_range_includes_the_chosen_last_day() {
        let (from, to) = resolve_range("2026-08-11", 30, Some("2026-08-01"), Some("2026-08-31"));
        assert_eq!(from, "2026-08-01");
        // Borne haute exclue : le 31 doit rester dans la fenêtre.
        assert_eq!(to, "2026-09-01");
    }

    #[test]
    fn resolve_range_ignores_malformed_bounds() {
        let (from, to) = resolve_range("2026-08-11", 7, Some("hier"), Some(""));
        assert_eq!(from, "2026-08-04");
        assert_eq!(to, "2026-08-12");
    }

    #[test]
    fn resolve_range_repairs_an_inverted_window() {
        let (from, to) = resolve_range("2026-08-11", 30, Some("2026-08-20"), Some("2026-08-01"));
        assert_eq!(from, "2026-08-20");
        assert_eq!(to, "2026-08-21");
    }

    #[test]
    fn available_months_merges_both_sources() {
        let (conn, m, c) = seed();
        upsert_daily(&conn, m, "2026-07-30", Some(12), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-02", Some(9), None, None).unwrap();
        // Le mois de juin n'apparaît que côté CurseForge : il doit remonter aussi.
        insert_snapshot(&conn, c, "2026-06-28T00:00:00Z", 100, None).unwrap();

        let months = available_months(&conn).unwrap();
        assert_eq!(months, vec!["2026-06", "2026-07", "2026-08"]);
    }

    #[test]
    fn day_axis_is_dense_and_excludes_the_upper_bound() {
        let axis = day_axis("2026-08-08", "2026-08-11");
        assert_eq!(axis, vec!["2026-08-08", "2026-08-09", "2026-08-10"]);
        assert!(day_axis("pas une date", "2026-08-11").is_empty());
    }

    #[test]
    fn spark_is_aligned_on_the_dense_axis() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-09", Some(7), None, None).unwrap();
        let rows = per_project(&conn, "2026-08-08", "2026-08-11").unwrap();
        assert_eq!(
            rows[0].spark,
            vec![0, 7, 0],
            "les jours sans donnee valent zero au lieu d'etre absents"
        );
    }

    #[test]
    fn project_detail_gathers_versions_and_countries() {
        let (conn, m, c) = seed();
        upsert_daily(&conn, m, "2026-08-09", Some(7), Some(3), Some("0.25")).unwrap();
        upsert_country(&conn, m, "2026-08-09", "XX", 4).unwrap();
        upsert_version(
            &conn,
            m,
            "v1",
            Some("1.0"),
            &["1.21".into()],
            &["fabric".into()],
            30,
            Some("2026-08-01T00:00:00Z"),
        )
        .unwrap();
        insert_snapshot(&conn, c, "2026-08-08T00:00:00Z", 100, None).unwrap();
        insert_snapshot(&conn, c, "2026-08-09T00:00:00Z", 130, None).unwrap();

        let detail = project_detail(&conn, "2026-08-08", "2026-08-11", Some(m), Some(c)).unwrap();
        assert_eq!(detail.days.len(), 3);
        assert_eq!(detail.downloads, vec![0, 7, 0]);
        assert_eq!(detail.views, vec![0, 3, 0]);
        assert_eq!(detail.curseforge, vec![0, 30, 0]);
        assert_eq!(detail.revenue[1], "0.25");
        assert_eq!(detail.versions.len(), 1);
        assert_eq!(detail.versions[0].loaders, vec!["fabric".to_string()]);
        assert_eq!(detail.countries[0].country, "??");
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
