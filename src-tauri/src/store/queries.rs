use crate::error::Result;
use crate::models::{
    CountryTotal, Kpis, LoaderCell, Overview, Platform, ProjectSummary, RevenuePoint, TimelinePoint,
};
use crate::store::metrics::{freshness, recent_events, snapshot_day_count, snapshot_deltas};
use crate::store::projects::{links, list};
use chrono::NaiveDate;
use rusqlite::{params, Connection};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap, HashSet};
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

/// Plateformes retenues à l'affichage. Masquer une plateforme ne supprime rien
/// en base : les relevés continuent, seule la lecture est filtrée.
#[derive(Debug, Clone, Copy)]
pub struct PlatformFilter {
    pub modrinth: bool,
    pub curseforge: bool,
}

impl Default for PlatformFilter {
    fn default() -> Self {
        Self {
            modrinth: true,
            curseforge: true,
        }
    }
}

impl PlatformFilter {
    /// Construit le filtre depuis la liste envoyée par l'interface. Une liste
    /// absente ou vide vaut « tout afficher » : mieux vaut trop montrer que
    /// présenter un écran vide sur une valeur mal formée.
    pub fn from_names(names: Option<&[String]>) -> Self {
        let Some(names) = names.filter(|n| !n.is_empty()) else {
            return Self::default();
        };
        Self {
            modrinth: names.iter().any(|n| n == "modrinth"),
            curseforge: names.iter().any(|n| n == "curseforge"),
        }
    }

    pub fn shows(&self, platform: Platform) -> bool {
        match platform {
            Platform::Modrinth => self.modrinth,
            Platform::CurseForge => self.curseforge,
        }
    }
}

pub fn timeline(
    conn: &Connection,
    from: &str,
    to: &str,
    filter: PlatformFilter,
) -> Result<Vec<TimelinePoint>> {
    let mut per_day: BTreeMap<String, (i64, i64)> = BTreeMap::new();

    // Les mesures quotidiennes viennent surtout de Modrinth, mais un historique
    // CurseForge rapporté du tableau de bord auteur atterrit dans la même table :
    // la plateforme du projet décide de la série qu'elle alimente.
    let mut stmt = conn.prepare(
        "SELECT p.platform, m.day, COALESCE(SUM(m.downloads), 0)
         FROM metrics_daily m JOIN projects p ON p.id = m.project_id
         WHERE m.day >= ?1 AND m.day < ?2 GROUP BY p.platform, m.day",
    )?;
    let mut measured_cf_days: HashSet<String> = HashSet::new();
    for row in stmt.query_map(params![from, to], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })? {
        let (platform, day, downloads) = row?;
        match Platform::from_str_lossy(&platform) {
            Platform::Modrinth if filter.modrinth => per_day.entry(day).or_default().0 += downloads,
            Platform::CurseForge if filter.curseforge => {
                measured_cf_days.insert(day.clone());
                per_day.entry(day).or_default().1 += downloads;
            }
            _ => {}
        }
    }

    if filter.curseforge {
        for ((_, day), delta) in snapshot_deltas(conn)? {
            // Un jour déjà mesuré prime sur l'écart entre deux snapshots :
            // sans cela, la journée serait comptée deux fois.
            if day.as_str() >= from && day.as_str() < to && !measured_cf_days.contains(&day) {
                per_day.entry(day).or_default().1 += delta;
            }
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

pub fn per_project(
    conn: &Connection,
    from: &str,
    to: &str,
    filter: PlatformFilter,
) -> Result<Vec<ProjectSummary>> {
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

    // Les deltas CurseForge arrivent à plat. On les indexe par projet et on
    // écarte tout de suite les jours hors fenêtre : sans cet index, chaque
    // projet reparcourait la totalité des deltas, soit un coût quadratique dès
    // quelques centaines de mods.
    let mut deltas_by_project: HashMap<i64, Vec<(String, i64)>> = HashMap::new();
    for ((cf_id, day), delta) in snapshot_deltas(conn)? {
        if day.as_str() >= from && day.as_str() < to {
            deltas_by_project
                .entry(cf_id)
                .or_default()
                .push((day, delta));
        }
    }

    let mut consumed_cf: HashSet<i64> = HashSet::new();
    let mut out: Vec<ProjectSummary> = Vec::new();

    for project in projects.iter().filter(|p| p.platform == Platform::Modrinth) {
        let link = link_rows
            .iter()
            .find(|l| l.modrinth_project_id == project.id);
        let cf = link.and_then(|l| by_id.get(&l.cf_project_id).copied());
        if let Some(cf) = cf {
            consumed_cf.insert(cf.id);
        }
        let mut spark: BTreeMap<String, i64> = if filter.modrinth {
            spark_by_project
                .get(&project.id)
                .cloned()
                .unwrap_or_default()
        } else {
            BTreeMap::new()
        };
        if filter.curseforge {
            if let Some(deltas) = cf.and_then(|c| deltas_by_project.get(&c.id)) {
                for (day, delta) in deltas {
                    *spark.entry(day.clone()).or_insert(0) += delta;
                }
            }
        }
        // Un mod dont la seule plateforme est masquée disparaît de la liste :
        // l'y laisser à zéro donnerait une ligne vide sans signification.
        if !filter.modrinth && cf.is_none() {
            continue;
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
            modrinth_ext_id: Some(project.ext_id.clone()),
            curseforge_ext_id: cf.and_then(|c| c.ext_id.parse().ok()),
            modrinth_downloads: if filter.modrinth {
                project.total_downloads
            } else {
                0
            },
            curseforge_downloads: match (filter.curseforge, cf) {
                (true, Some(cf)) => cf.total_downloads,
                _ => 0,
            },
            // Les abonnés d'un mod ne se lisent que sur Modrinth : CurseForge
            // n'en publie qu'un total, pour le compte entier.
            followers: project.followers,
            link_confidence: link.map(|l| l.confidence),
            spark: densify(&spark),
        });
    }

    if filter.curseforge {
        for project in projects
            .iter()
            .filter(|p| p.platform == Platform::CurseForge && !consumed_cf.contains(&p.id))
        {
            let spark: BTreeMap<String, i64> = deltas_by_project
                .get(&project.id)
                .map(|deltas| deltas.iter().cloned().collect())
                .unwrap_or_default();
            out.push(ProjectSummary {
                key: format!("c{}", project.id),
                title: project.title.clone(),
                icon_url: project.icon_url.clone(),
                modrinth_id: None,
                curseforge_id: Some(project.id),
                modrinth_ext_id: None,
                curseforge_ext_id: project.ext_id.parse().ok(),
                modrinth_downloads: 0,
                curseforge_downloads: project.total_downloads,
                followers: project.followers,
                link_confidence: None,
                spark: densify(&spark),
            });
        }
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

/// Répartition des téléchargements par version de jeu et chargeur.
///
/// Les deux plateformes alimentent cette table : Modrinth par ses versions,
/// CurseForge par ses fichiers publiés. Les compteurs CurseForge par fichier
/// sont partiels, la carte donne donc une répartition, pas un total.
pub fn loaders(conn: &Connection, filter: PlatformFilter) -> Result<Vec<LoaderCell>> {
    let mut stmt = conn.prepare(
        "SELECT v.game_versions, v.loaders, v.downloads FROM versions v
         JOIN projects p ON p.id = v.project_id
         WHERE p.platform IN (?1, ?2)",
    )?;
    let mut totals: HashMap<(String, String), i64> = HashMap::new();
    // Une plateforme masquée est comparée à une valeur qu'aucune ligne ne porte.
    let modrinth = if filter.modrinth { "modrinth" } else { "" };
    let curseforge = if filter.curseforge { "curseforge" } else { "" };
    for row in stmt.query_map(params![modrinth, curseforge], |r| {
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

/// Tarif d'un point CurseForge, en dollars.
fn point_value() -> Decimal {
    Decimal::from_str(&crate::store::metrics::CF_POINT_VALUE_USD.to_string()).unwrap_or_default()
}

/// Contre-valeur en dollars du dernier solde de points relevé.
fn cf_balance_usd(conn: &Connection) -> Result<Decimal> {
    let Some(points) = crate::store::metrics::latest_cf_points(conn)? else {
        return Ok(Decimal::ZERO);
    };
    Ok(Decimal::from(points) * point_value())
}

/// Tout ce que CurseForge dit de l'argent : solde de points, sa contre-valeur,
/// les mois relevés et les deux estimations affichées sur son tableau de bord.
pub fn curseforge_revenue(conn: &Connection) -> Result<crate::models::CfRevenue> {
    let points = crate::store::metrics::latest_cf_points(conn)?.unwrap_or(0);
    // Une estimation absente vaut mieux qu'une case vide : le front n'affiche
    // la ligne que si le tableau de bord l'a vraiment donnée.
    let read = |key: &str| -> Result<Option<String>> {
        Ok(crate::store::metrics::get_meta(conn, key)?.filter(|value| !value.is_empty()))
    };
    Ok(crate::models::CfRevenue {
        points,
        points_usd: format!(
            "{:.2}",
            points as f64 * crate::store::metrics::CF_POINT_VALUE_USD
        ),
        last_month: read("curseforge_revenue_last_month")?,
        year_to_date: read("curseforge_revenue_ytd")?,
        monthly: crate::store::metrics::cf_revenue(conn)?,
    })
}

/// Indicateurs de tête.
///
/// Deux lectures cohabitent, et c'est voulu : les mesures d'état — cumul depuis
/// l'origine, solde retirable, abonnés — ne dépendent que du jour, tandis que
/// les mesures de période suivent `from`..`to`, les bornes choisies dans la
/// barre de filtres. L'interface bascule de l'une à l'autre ; les deux sont
/// calculées ici, car un aller-retour de plus coûterait davantage que ces
/// quelques sommes.
///
/// `to` est exclu, comme partout ailleurs dans ce module.
pub fn kpis(
    conn: &Connection,
    today: &str,
    from: &str,
    to: &str,
    filter: PlatformFilter,
) -> Result<Kpis> {
    let window_start = shift_day(today, -30);
    let previous_start = shift_day(today, -60);

    // `metrics_daily` porte désormais les deux plateformes : la collecte du
    // tableau de bord CurseForge y écrit ses journées. La somme suit donc le
    // filtre, plateforme par plateforme.
    let window_of = |from: &str, to: &str, platform: Platform| -> Result<i64> {
        if !filter.shows(platform) {
            return Ok(0);
        }
        Ok(conn.query_row(
            "SELECT COALESCE(SUM(m.downloads), 0) FROM metrics_daily m
             JOIN projects p ON p.id = m.project_id
             WHERE m.day >= ?1 AND m.day < ?2 AND p.platform = ?3",
            params![from, to, platform.as_str()],
            |r| r.get(0),
        )?)
    };
    let sum_downloads = |from: &str, to: &str| -> Result<i64> {
        Ok(window_of(from, to, Platform::Modrinth)? + window_of(from, to, Platform::CurseForge)?)
    };

    let per_platform = |platform: Platform| -> Result<i64> {
        if !filter.shows(platform) {
            return Ok(0);
        }
        Ok(conn.query_row(
            "SELECT COALESCE(SUM(total_downloads), 0) FROM projects
             WHERE platform = ?1 AND archived_at IS NULL",
            params![platform.as_str()],
            |r| r.get(0),
        )?)
    };

    // Les revenus sont un dispositif Modrinth : masquer cette plateforme les
    // retire de l'affichage plutôt que de laisser un total sans source visible.
    let mut revenue_window = Decimal::ZERO;
    if filter.modrinth {
        let mut stmt =
            conn.prepare("SELECT revenue FROM metrics_daily WHERE revenue IS NOT NULL")?;
        for row in stmt.query_map([], |r| r.get::<_, String>(0))? {
            revenue_window += Decimal::from_str(&row?).unwrap_or_default();
        }
    }

    // Période affichée, et celle de même durée qui la précède : l'écart n'a de
    // sens qu'entre deux fenêtres de longueur égale.
    let span = day_axis(from, to).len() as i64;
    let previous_from = shift_day(from, -span);
    let range_modrinth = window_of(from, to, Platform::Modrinth)?;
    let range_curseforge = window_of(from, to, Platform::CurseForge)?;

    // CurseForge ne publie aucun revenu par jour, mais son solde de points est
    // relevé à chaque passage : l'écart entre deux relevés dit ce qui a été
    // gagné entre eux, converti au tarif publié.
    let range_revenue_curseforge = if filter.curseforge {
        let points = crate::store::metrics::cf_points_gained(conn, from, to)?;
        Decimal::from(points) * point_value()
    } else {
        Decimal::ZERO
    };

    let mut range_revenue_modrinth = Decimal::ZERO;
    if filter.modrinth {
        let mut stmt = conn.prepare(
            "SELECT m.revenue FROM metrics_daily m
             JOIN projects p ON p.id = m.project_id
             WHERE m.revenue IS NOT NULL AND m.day >= ?1 AND m.day < ?2
               AND p.platform = ?3",
        )?;
        let rows = stmt.query_map(params![from, to, Platform::Modrinth.as_str()], |r| {
            r.get::<_, String>(0)
        })?;
        for row in rows {
            range_revenue_modrinth += Decimal::from_str(&row?).unwrap_or_default();
        }
    }

    // Le cumul réel vient du solde de reversement, pas des analytics : celles-ci
    // ne remontent que sur une fenêtre récente et sous-estiment lourdement ce
    // qu'un projet a rapporté depuis ses débuts.
    let balance = if filter.modrinth {
        payout(conn, today)?
    } else {
        crate::models::Payout::default()
    };
    let decimal = |raw: &str| Decimal::from_str(raw).unwrap_or_default();
    let earned = decimal(&balance.withdrawn_lifetime)
        + decimal(&balance.available)
        + decimal(&balance.pending);
    // Sans solde relevé, le cumul de la fenêtre reste la seule mesure connue.
    let revenue_modrinth = if earned.is_zero() {
        revenue_window
    } else {
        earned
    };

    // CurseForge paie en points, convertis au tarif qu'il publie. Ce solde est
    // retirable immédiatement, sans maturation : il s'ajoute donc au cumul comme
    // à la somme disponible.
    let revenue_curseforge = if filter.curseforge {
        cf_balance_usd(conn)?
    } else {
        Decimal::ZERO
    };
    let revenue_total = revenue_modrinth + revenue_curseforge;
    let available = decimal(&balance.available) + revenue_curseforge;

    let downloads_modrinth = per_platform(Platform::Modrinth)?;
    let downloads_curseforge = per_platform(Platform::CurseForge)?;

    let projects_of = |platform: Platform| -> Result<i64> {
        if !filter.shows(platform) {
            return Ok(0);
        }
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM projects WHERE archived_at IS NULL AND platform = ?1",
            params![platform.as_str()],
            |r| r.get(0),
        )?)
    };
    let projects_modrinth = projects_of(Platform::Modrinth)?;
    let projects_curseforge = projects_of(Platform::CurseForge)?;

    let followers_modrinth = if filter.modrinth {
        conn.query_row(
            "SELECT COALESCE(SUM(followers), 0) FROM projects
             WHERE archived_at IS NULL AND platform = ?1",
            params![Platform::Modrinth.as_str()],
            |r| r.get(0),
        )?
    } else {
        0
    };
    // CurseForge ne publie aucun compte par projet : sa fiche de compte annonce
    // un total, relevé à la collecte. On le rend tel quel plutôt que de le
    // répartir arbitrairement entre les mods.
    let followers_curseforge = if filter.curseforge {
        crate::store::metrics::get_meta(conn, "curseforge_followers")?
            .and_then(|raw| raw.parse::<i64>().ok())
            .unwrap_or(0)
    } else {
        0
    };

    Ok(Kpis {
        downloads_total: downloads_modrinth + downloads_curseforge,
        downloads_modrinth,
        downloads_curseforge,
        downloads_30d: sum_downloads(&window_start, today)?,
        downloads_30d_modrinth: window_of(&window_start, today, Platform::Modrinth)?,
        downloads_30d_curseforge: window_of(&window_start, today, Platform::CurseForge)?,
        downloads_prev_30d: sum_downloads(&previous_start, &window_start)?,
        revenue_total: revenue_total.normalize().to_string(),
        revenue_modrinth: revenue_modrinth.normalize().to_string(),
        revenue_curseforge: revenue_curseforge.normalize().to_string(),
        revenue_available: available.normalize().to_string(),
        revenue_available_modrinth: decimal(&balance.available).normalize().to_string(),
        revenue_available_curseforge: revenue_curseforge.normalize().to_string(),
        revenue_pending: balance.pending.clone(),
        revenue_window: revenue_window.normalize().to_string(),
        range_downloads: range_modrinth + range_curseforge,
        range_downloads_modrinth: range_modrinth,
        range_downloads_curseforge: range_curseforge,
        range_downloads_prev: sum_downloads(&previous_from, from)?,
        range_revenue: (range_revenue_modrinth + range_revenue_curseforge)
            .normalize()
            .to_string(),
        range_revenue_modrinth: range_revenue_modrinth.normalize().to_string(),
        range_revenue_curseforge: range_revenue_curseforge.normalize().to_string(),
        followers: followers_modrinth + followers_curseforge,
        followers_modrinth,
        followers_curseforge,
        projects_active: projects_modrinth + projects_curseforge,
        projects_modrinth,
        projects_curseforge,
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

    // Le détail d'un mod montre toujours ses deux plateformes : le filtre de la
    // page de vision ne s'applique pas ici, on y vient justement pour comparer.
    let summary = per_project(conn, from, to, PlatformFilter::default())?
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

    // Les deux plateformes publient : les versions Modrinth et les fichiers
    // CurseForge du mod sont rassemblés dans une seule chronologie.
    let mut versions = Vec::new();
    for id in [modrinth_id, curseforge_id].into_iter().flatten() {
        let mut stmt = conn.prepare(
            "SELECT version_number, game_versions, loaders, downloads, date_published
             FROM versions WHERE project_id = ?1 ORDER BY date_published DESC",
        )?;
        let rows = stmt
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
        versions.extend(rows);
    }
    // Publications les plus récentes en tête, toutes plateformes mêlées.
    versions.sort_by(|a, b| b.date_published.cmp(&a.date_published));

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

pub fn overview(
    conn: &Connection,
    today: &str,
    from: &str,
    to: &str,
    filter: PlatformFilter,
) -> Result<Overview> {
    let (from, to) = (from.to_string(), to.to_string());
    let kpis = kpis(conn, today, &from, &to, filter)?;
    // Origine, revenus et reversements ne sont relevés que sur Modrinth :
    // masquer cette plateforme les vide au lieu de laisser des chiffres dont la
    // source n'est plus visible.
    let payout = if filter.modrinth {
        payout(conn, today)?
    } else {
        crate::models::Payout::default()
    };

    Ok(Overview {
        kpis,
        // La borne haute est exclue en interne : on la ramène au dernier jour
        // réellement affiché avant de la rendre à l'interface.
        to: shift_day(&to, -1),
        from: from.clone(),
        available_months: available_months(conn)?,
        days: day_axis(&from, &to),
        timeline: timeline(conn, &from, &to, filter)?,
        per_project: per_project(conn, &from, &to, filter)?,
        countries: if filter.modrinth {
            countries(conn, &from, &to)?
        } else {
            Vec::new()
        },
        loaders: loaders(conn, filter)?,
        revenue: if filter.modrinth {
            revenue(conn, &from, &to)?
        } else {
            Vec::new()
        },
        revenue_by_project: if filter.modrinth {
            revenue_by_project(conn, &from, &to)?
        } else {
            Vec::new()
        },
        payout,
        events: recent_events(conn, 40)?,
        freshness: freshness(conn)?,
        curseforge_history_days: snapshot_day_count(conn)?,
        curseforge_revenue: if filter.curseforge {
            curseforge_revenue(conn)?
        } else {
            crate::models::CfRevenue::default()
        },
        // La devise choisie vit dans les réglages, hors de la base : la commande
        // qui appelle cette vue la complète avant de la rendre à l'interface.
        currency: crate::models::CurrencyView::default(),
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
            followers: Some(5),
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
    fn an_imported_curseforge_day_beats_the_snapshot_gap() {
        let (conn, _, c) = seed();
        // Deux snapshots donnent un écart de 75 pour le 10.
        insert_snapshot(&conn, c, "2026-08-09T00:00:00Z", 100, None).unwrap();
        insert_snapshot(&conn, c, "2026-08-10T00:00:00Z", 175, None).unwrap();
        // Le tableau de bord auteur, lui, donne le chiffre exact.
        upsert_daily(&conn, c, "2026-08-10", Some(64), None, None).unwrap();

        let points =
            timeline(&conn, "2026-08-01", "2026-08-11", PlatformFilter::default()).unwrap();
        let tenth = points.iter().find(|p| p.day == "2026-08-10").unwrap();
        assert_eq!(tenth.curseforge, 64, "la mesure prime sur l'écart estimé");
    }

    #[test]
    fn timeline_merges_modrinth_series_and_curseforge_deltas() {
        let (conn, m, c) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(40), None, None).unwrap();
        insert_snapshot(&conn, c, "2026-08-09T00:00:00Z", 100, None).unwrap();
        insert_snapshot(&conn, c, "2026-08-10T00:00:00Z", 175, None).unwrap();
        let points =
            timeline(&conn, "2026-08-01", "2026-08-11", PlatformFilter::default()).unwrap();
        let day = points.iter().find(|p| p.day == "2026-08-10").unwrap();
        assert_eq!(day.modrinth, 40);
        assert_eq!(day.curseforge, 75);
    }

    #[test]
    fn per_project_groups_linked_projects_under_one_row() {
        let (conn, _, _) = seed();
        let rows =
            per_project(&conn, "2026-08-01", "2026-08-11", PlatformFilter::default()).unwrap();
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
                followers: None,
            },
        )
        .unwrap();
        let rows =
            per_project(&conn, "2026-08-01", "2026-08-11", PlatformFilter::default()).unwrap();
        assert_eq!(rows.len(), 2);
        let solo = rows.iter().find(|r| r.title == "Solo").unwrap();
        assert_eq!(solo.curseforge_downloads, 0);
        assert!(solo.curseforge_id.is_none());
    }

    #[test]
    fn platform_filter_reads_the_names_sent_by_the_interface() {
        let both =
            PlatformFilter::from_names(Some(&["modrinth".to_string(), "curseforge".to_string()]));
        assert!(both.modrinth && both.curseforge);

        let only_cf = PlatformFilter::from_names(Some(&["curseforge".to_string()]));
        assert!(!only_cf.modrinth && only_cf.curseforge);

        // Absente ou vide, la liste ne doit jamais vider l'écran.
        assert!(PlatformFilter::from_names(None).modrinth);
        assert!(PlatformFilter::from_names(Some(&[])).curseforge);
    }

    #[test]
    fn hiding_a_platform_empties_its_series() {
        let (conn, m, c) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(40), None, None).unwrap();
        insert_snapshot(&conn, c, "2026-08-09T00:00:00Z", 100, None).unwrap();
        insert_snapshot(&conn, c, "2026-08-10T00:00:00Z", 175, None).unwrap();

        let hidden = PlatformFilter {
            modrinth: false,
            curseforge: true,
        };
        let points = timeline(&conn, "2026-08-01", "2026-08-11", hidden).unwrap();
        assert!(points.iter().all(|p| p.modrinth == 0));
        assert_eq!(points.iter().map(|p| p.curseforge).sum::<i64>(), 75);

        let rows = per_project(&conn, "2026-08-01", "2026-08-11", hidden).unwrap();
        assert!(rows.iter().all(|r| r.modrinth_downloads == 0));
        assert!(rows.iter().any(|r| r.curseforge_downloads > 0));
    }

    #[test]
    fn hiding_modrinth_drops_projects_that_only_live_there() {
        let (conn, _, _) = seed();
        upsert(
            &conn,
            &ProjectUpsert {
                platform: Platform::Modrinth,
                ext_id: "solo".into(),
                slug: Some("no-night-skip".into()),
                title: "No Night Skip".into(),
                project_type: Some("mod".into()),
                url: None,
                icon_url: None,
                created_at: None,
                total_downloads: 1_800,
                followers: Some(1),
            },
        )
        .unwrap();

        let rows = per_project(
            &conn,
            "2026-08-01",
            "2026-08-11",
            PlatformFilter {
                modrinth: false,
                curseforge: true,
            },
        )
        .unwrap();
        assert!(
            !rows.iter().any(|r| r.title == "No Night Skip"),
            "un mod sans jumeau CurseForge n'a rien à montrer quand Modrinth est masqué"
        );
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
        let rows =
            per_project(&conn, "2026-08-08", "2026-08-11", PlatformFilter::default()).unwrap();
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
        let cells = loaders(&conn, PlatformFilter::default()).unwrap();
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
    fn revenue_total_follows_the_payout_balance_not_the_analytics() {
        let (conn, m, _) = seed();
        // Les analytics ne couvrent qu'une fenêtre récente.
        upsert_daily(&conn, m, "2026-08-10", Some(10), None, Some("38.18")).unwrap();
        crate::store::metrics::set_meta(
            &conn,
            "modrinth_payout",
            r#"{"available":"12.63","pending":"4.84","withdrawn_lifetime":"70.42",
                "withdrawn_ytd":"15.36","dates":{}}"#,
        )
        .unwrap();

        let k = kpis(&conn, "2026-08-11", "2026-07-12", "2026-08-11",PlatformFilter::default()).unwrap();
        // 70,42 déjà retirés + 12,63 retirables + 4,84 en maturation.
        assert_eq!(k.revenue_total, "87.89");
        assert_eq!(k.revenue_available, "12.63");
        assert_eq!(k.revenue_pending, "4.84");
        assert_eq!(k.revenue_window, "38.18");
    }

    #[test]
    fn revenue_total_falls_back_on_the_window_without_a_balance() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(10), None, Some("2.5")).unwrap();
        let k = kpis(&conn, "2026-08-11", "2026-07-12", "2026-08-11",PlatformFilter::default()).unwrap();
        assert_eq!(k.revenue_total, "2.5");
    }

    #[test]
    fn the_curseforge_balance_adds_up_with_the_modrinth_payout() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(10), None, Some("1.00")).unwrap();
        crate::store::metrics::set_meta(
            &conn,
            "modrinth_payout",
            r#"{"available":"12.63","pending":"4.84","withdrawn_lifetime":"70.42",
                "withdrawn_ytd":"15.36","dates":{}}"#,
        )
        .unwrap();
        // 423 points × 0,05 $ = 21,15 $
        crate::store::metrics::record_cf_points(&conn, "2026-08-11", 423, "x").unwrap();

        let k = kpis(&conn, "2026-08-11", "2026-07-12", "2026-08-11",PlatformFilter::default()).unwrap();
        assert_eq!(k.revenue_modrinth, "87.89");
        assert_eq!(k.revenue_curseforge, "21.15");
        assert_eq!(k.revenue_total, "109.04");
        // Les points CurseForge se retirent sans maturation : 12,63 + 21,15.
        assert_eq!(k.revenue_available, "33.78");
    }

    #[test]
    fn hiding_curseforge_removes_its_balance_from_the_total() {
        let (conn, _, _) = seed();
        crate::store::metrics::record_cf_points(&conn, "2026-08-11", 423, "x").unwrap();
        let only_modrinth = PlatformFilter {
            modrinth: true,
            curseforge: false,
        };
        let k = kpis(&conn, "2026-08-11", "2026-07-12", "2026-08-11",only_modrinth).unwrap();
        assert_eq!(k.revenue_curseforge, "0");
        assert_eq!(k.revenue_total, "0");
    }

    #[test]
    fn thirty_day_downloads_follow_the_platform_filter() {
        let (conn, m, c) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();
        upsert_daily(&conn, c, "2026-08-10", Some(30), None, None).unwrap();

        let both = kpis(&conn, "2026-08-11", "2026-07-12", "2026-08-11",PlatformFilter::default()).unwrap();
        assert_eq!(both.downloads_30d, 130);

        let only_modrinth = PlatformFilter {
            modrinth: true,
            curseforge: false,
        };
        assert_eq!(
            kpis(&conn, "2026-08-11", "2026-07-12", "2026-08-11",only_modrinth)
                .unwrap()
                .downloads_30d,
            100
        );
    }

    /// Chaque carte de tête annonce d'où vient son total : la somme des deux
    /// parts doit toujours retomber sur le total affiché.
    #[test]
    fn every_headline_figure_is_split_between_the_two_platforms() {
        let (conn, m, c) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();
        upsert_daily(&conn, c, "2026-08-10", Some(30), None, None).unwrap();
        crate::store::metrics::record_cf_points(&conn, "2026-08-10", 400, "2026-08-10T00:00:00Z")
            .unwrap();

        let k = kpis(&conn, "2026-08-11", "2026-07-12", "2026-08-11",PlatformFilter::default()).unwrap();
        assert_eq!(k.downloads_30d_modrinth, 100);
        assert_eq!(k.downloads_30d_curseforge, 30);
        assert_eq!(
            k.downloads_30d_modrinth + k.downloads_30d_curseforge,
            k.downloads_30d
        );
        assert_eq!(k.projects_modrinth, 1);
        assert_eq!(k.projects_curseforge, 1);
        assert_eq!(
            k.projects_modrinth + k.projects_curseforge,
            k.projects_active
        );
        // 400 points à 0,05 $ : la part CurseForge du solde retirable.
        assert_eq!(k.revenue_available_curseforge, "20");
        let parts = Decimal::from_str(&k.revenue_available_modrinth).unwrap()
            + Decimal::from_str(&k.revenue_available_curseforge).unwrap();
        assert_eq!(parts.normalize().to_string(), k.revenue_available);
    }

    /// Les mesures de période suivent les bornes, là où les mesures d'état
    /// n'écoutent que le jour : c'est toute la différence entre les deux
    /// lectures que l'interface propose.
    #[test]
    fn range_figures_follow_the_chosen_bounds() {
        let (conn, m, c) = seed();
        upsert_daily(&conn, m, "2026-08-05", Some(50), None, Some("2.00")).unwrap();
        upsert_daily(&conn, c, "2026-08-05", Some(20), None, None).unwrap();
        // Hors de la fenêtre demandée, mais dans celle qui la précède.
        upsert_daily(&conn, m, "2026-08-01", Some(9), None, Some("9.00")).unwrap();

        // Du 4 au 6 août inclus : la borne haute est exclue.
        let k = kpis(
            &conn,
            "2026-08-11",
            "2026-08-04",
            "2026-08-07",
            PlatformFilter::default(),
        )
        .unwrap();
        assert_eq!(k.range_downloads, 70);
        assert_eq!(k.range_downloads_modrinth, 50);
        assert_eq!(k.range_downloads_curseforge, 20);
        // Les trois jours précédents, du 1er au 3 août.
        assert_eq!(k.range_downloads_prev, 9);
        assert_eq!(k.range_revenue, "2");
        // Les mesures d'état ignorent ces bornes.
        assert_eq!(k.downloads_30d, 79);
    }

    /// Les revenus CurseForge de la période viennent de l'écart entre les
    /// soldes de points relevés, seule trace que la plateforme en laisse.
    #[test]
    fn curseforge_range_revenue_comes_from_the_points_gap() {
        let (conn, _, _) = seed();
        crate::store::metrics::record_cf_points(&conn, "2026-08-04", 100, "x").unwrap();
        crate::store::metrics::record_cf_points(&conn, "2026-08-05", 160, "x").unwrap();
        crate::store::metrics::record_cf_points(&conn, "2026-08-06", 200, "x").unwrap();

        let k = kpis(
            &conn,
            "2026-08-11",
            "2026-08-05",
            "2026-08-07",
            PlatformFilter::default(),
        )
        .unwrap();
        // 100 points gagnés sur la fenêtre, à 0,05 $ le point.
        assert_eq!(k.range_revenue_curseforge, "5");
        assert_eq!(k.range_revenue, "5");
    }

    #[test]
    fn kpis_compare_the_two_last_thirty_day_windows() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, Some("1.5")).unwrap();
        upsert_daily(&conn, m, "2026-06-20", Some(40), None, Some("0.5")).unwrap();
        let k = kpis(&conn, "2026-08-11", "2026-07-12", "2026-08-11",PlatformFilter::default()).unwrap();
        assert_eq!(k.downloads_30d, 100);
        assert_eq!(k.downloads_prev_30d, 40);
        assert_eq!(k.revenue_total, "2");
        assert_eq!(k.downloads_modrinth, 23_225);
        assert_eq!(k.downloads_curseforge, 86_753);
    }
}
