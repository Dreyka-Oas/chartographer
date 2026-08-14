//! Rang d'une journée parmi celles qui l'ont précédée.
//!
//! Le classement vit à part des autres requêtes parce qu'il répond à une autre
//! question : non pas « combien », mais « était-ce un bon jour quand il s'est
//! produit ». Il ne regarde donc jamais en avant.

use crate::error::Result;
use crate::models::{DayRankRow, DayRankings, Platform, RankBy};
use crate::store::queries::{shift_day, timeline, PlatformFilter};
use rusqlite::{params, Connection};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;

/// Longueur de la fenêtre de comparaison, en jours, la journée jugée comprise.
pub const RANK_WINDOW_DAYS: i64 = 90;

/// Rang de `total` parmi les totaux de sa fenêtre, 1 étant le meilleur.
///
/// Les journées à zéro sont écartées par l'appelant : ce sont des jours sans
/// relevé, pas des jours creux, et elles flatteraient le classement. Le rang
/// compte les journées strictement meilleures, si bien que deux journées
/// égales portent le même rang.
pub fn rank_within(window: &[i64], total: i64) -> Option<i64> {
    (total > 0).then(|| window.iter().filter(|other| **other > total).count() as i64 + 1)
}

/// Contre-valeur en dollars d'un point CurseForge, telle que la base la connaît.
fn point_value() -> Decimal {
    Decimal::from_str(&crate::store::metrics::CF_POINT_VALUE_USD.to_string()).unwrap_or_default()
}

/// Revenus jour par jour, plateforme par plateforme, `to` exclu.
///
/// Modrinth relève ses revenus quotidiennement. CurseForge n'en publie aucun :
/// ce qui apparaît ici est l'écart entre deux soldes de points, rapporté au
/// jour où il a été constaté. Les journées sans le moindre revenu sont absentes
/// de la table plutôt que portées à zéro : rien n'a été relevé, ce qui n'est
/// pas la même chose que rien gagné.
pub fn revenue_by_day(
    conn: &Connection,
    from: &str,
    to: &str,
    filter: PlatformFilter,
) -> Result<BTreeMap<String, (Decimal, Decimal)>> {
    let mut out: BTreeMap<String, (Decimal, Decimal)> = BTreeMap::new();

    if filter.modrinth {
        let mut stmt = conn.prepare(
            "SELECT m.day, m.revenue FROM metrics_daily m
             JOIN projects p ON p.id = m.project_id
             WHERE m.revenue IS NOT NULL AND m.day >= ?1 AND m.day < ?2
               AND p.platform = ?3",
        )?;
        let rows = stmt.query_map(params![from, to, Platform::Modrinth.as_str()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (day, raw) = row?;
            out.entry(day).or_default().0 += Decimal::from_str(&raw).unwrap_or_default();
        }
    }

    if filter.curseforge {
        let mut day = from.to_string();
        while day.as_str() < to {
            let next = shift_day(&day, 1);
            let gained = crate::store::metrics::cf_points_gained(conn, &day, &next)?;
            if gained != 0 {
                out.entry(day.clone()).or_default().1 += Decimal::from(gained) * point_value();
            }
            day = next;
        }
    }

    Ok(out)
}

/// Première journée relevée pour une plateforme, toutes périodes confondues.
fn first_day(conn: &Connection, platform: Platform) -> Result<Option<String>> {
    Ok(conn.query_row(
        "SELECT MIN(m.day) FROM metrics_daily m JOIN projects p ON p.id = m.project_id
         WHERE p.platform = ?1 AND COALESCE(m.downloads, 0) > 0",
        params![platform.as_str()],
        |r| r.get::<_, Option<String>>(0),
    )?)
}

/// Valeur classée d'une journée, selon le critère demandé.
///
/// Les revenus sont des `Decimal` ; `rank_within` travaille sur des entiers.
/// On les ramène en centièmes : deux journées à un centime près restent
/// distinctes, et rien ne se perd à l'échelle où l'on compare.
fn ranked_value(point: &crate::models::TimelinePoint, revenue: &(Decimal, Decimal), by: RankBy) -> i64 {
    match by {
        RankBy::Downloads => point.modrinth + point.curseforge,
        RankBy::Revenue => {
            let cents = (revenue.0 + revenue.1) * Decimal::from(100);
            cents.round().to_string().parse().unwrap_or(0)
        }
    }
}

/// Classement des journées d'une période, `to` exclu.
///
/// Une seule question se pose à chaque journée : quel rang les réglages
/// demandés lui donnent-ils. `by` choisit ce qui est classé — téléchargements
/// ou revenus. `window` choisit à quoi une journée se compare :
///
/// - `Some(n)` : aux `n` journées glissantes qui la précèdent, elle comprise —
///   le rang qu'elle avait le jour où elle s'est produite. Le classement ne
///   regarde jamais en avant : rien de ce qui est arrivé ensuite ne peut plus
///   le modifier. C'est aussi le cas particulier de « la période affichée »,
///   quand l'appelant passe la longueur de cette période comme fenêtre.
/// - `None` : à tout ce qui a été relevé jusqu'à aujourd'hui, une seule et
///   même référence pour toutes les journées listées, ce qui suppose de
///   charger l'historique depuis l'origine plutôt que depuis `from - 89`.
pub fn day_rankings(
    conn: &Connection,
    from: &str,
    to: &str,
    filter: PlatformFilter,
    by: RankBy,
    window: Option<i64>,
) -> Result<DayRankings> {
    let history_start = match window {
        Some(n) => shift_day(from, -(n - 1)),
        None => "0000-01-01".to_string(),
    };
    let history = timeline(conn, &history_start, to, filter)?;
    let revenue = revenue_by_day(conn, &history_start, to, filter)?;

    let values: Vec<i64> = history
        .iter()
        .map(|p| {
            let r = revenue.get(&p.day).copied().unwrap_or_default();
            ranked_value(p, &r, by)
        })
        .collect();

    // Sans fenêtre, toutes les journées listées se comparent à la même
    // référence : tout ce qui a été relevé jusqu'à `to`. Un tri unique suffit
    // alors, là où la fenêtre glissante recalcule sa propre référence à
    // chaque journée.
    let whole_history: Vec<i64> = values.iter().copied().filter(|v| *v > 0).collect();

    let mut rows = Vec::new();
    // Borne basse glissante de la fenêtre : elle n'avance jamais en arrière, ce
    // qui laisse le parcours linéaire au lieu de rouvrir la fenêtre à chaque jour.
    let mut start = 0usize;
    for (i, point) in history.iter().enumerate() {
        if point.day.as_str() < from {
            continue;
        }
        let value = values[i];
        let (rank, compared_days) = match window {
            Some(n) => {
                let window_start = shift_day(&point.day, -(n - 1));
                while history[start].day < window_start {
                    start += 1;
                }
                let sliding: Vec<i64> =
                    values[start..=i].iter().copied().filter(|v| *v > 0).collect();
                let compared = sliding.len() as i64;
                (rank_within(&sliding, value), compared)
            }
            None => (rank_within(&whole_history, value), whole_history.len() as i64),
        };
        let (modrinth_revenue, curseforge_revenue) =
            revenue.get(&point.day).copied().unwrap_or_default();
        rows.push(DayRankRow {
            day: point.day.clone(),
            modrinth: point.modrinth,
            curseforge: point.curseforge,
            total: point.modrinth + point.curseforge,
            revenue: (modrinth_revenue + curseforge_revenue).normalize().to_string(),
            rank,
            compared_days,
        });
    }

    Ok(DayRankings {
        rows,
        first_modrinth_day: first_day(conn, Platform::Modrinth)?,
        first_curseforge_day: first_day(conn, Platform::CurseForge)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::metrics::upsert_daily;
    use crate::store::projects::{upsert, upsert_link, ProjectUpsert};
    use crate::store::schema::migrate;

    #[test]
    fn rank_counts_only_better_days() {
        assert_eq!(rank_within(&[10, 30, 20], 30), Some(1));
        assert_eq!(rank_within(&[10, 30, 20], 20), Some(2));
        assert_eq!(rank_within(&[10, 30, 20], 10), Some(3));
    }

    /// Une journée sans le moindre téléchargement n'est pas un jour creux :
    /// c'est un jour sans relevé, et il n'a pas de rang.
    #[test]
    fn a_day_without_downloads_has_no_rank() {
        assert_eq!(rank_within(&[10, 30], 0), None);
    }

    /// Deux journées identiques partagent le même rang : la seconde ne perd
    /// pas une place pour être arrivée après.
    #[test]
    fn equal_days_share_their_rank() {
        assert_eq!(rank_within(&[50, 50, 10], 50), Some(1));
    }

    /// Base de test : un mod des deux côtés, reliés, comme dans `queries`.
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
    fn revenue_is_returned_day_by_day() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, Some("1.25")).unwrap();
        upsert_daily(&conn, m, "2026-08-11", Some(300), None, Some("3.50")).unwrap();

        let by_day = revenue_by_day(&conn, "2026-08-10", "2026-08-12", PlatformFilter::default())
            .unwrap();
        assert_eq!(by_day.get("2026-08-10").unwrap().0, Decimal::from_str("1.25").unwrap());
        assert_eq!(by_day.get("2026-08-11").unwrap().0, Decimal::from_str("3.50").unwrap());
        assert_eq!(by_day.len(), 2, "les jours sans revenu ne sont pas inventés");
    }

    /// Masquer Modrinth doit vider ses revenus, pas les laisser passer sous un
    /// autre nom.
    #[test]
    fn hiding_modrinth_hides_its_revenue() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, Some("1.25")).unwrap();
        let filter = PlatformFilter { modrinth: false, curseforge: true };
        let by_day = revenue_by_day(&conn, "2026-08-10", "2026-08-11", filter).unwrap();
        assert!(by_day.get("2026-08-10").map(|v| v.0).unwrap_or_default().is_zero());
    }

    /// Fenêtre glissante : le rang ne regarde que les journées qui précèdent.
    #[test]
    fn a_sliding_window_never_looks_ahead() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-11", Some(500), None, None).unwrap();

        let out = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-12",
            PlatformFilter::default(),
            RankBy::Downloads,
            Some(90),
        )
        .unwrap();
        let first = out.rows.iter().find(|r| r.day == "2026-08-10").unwrap();
        assert_eq!(first.rank, Some(1), "elle était première le jour même");
    }

    /// Sans fenêtre, la comparaison porte sur tout l'historique antérieur.
    #[test]
    fn without_a_window_the_whole_past_counts() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2024-01-01", Some(900), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();

        let out = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-11",
            PlatformFilter::default(),
            RankBy::Downloads,
            None,
        )
        .unwrap();
        let day = &out.rows[0];
        assert_eq!(day.rank, Some(2), "la journée de 2024 la précède et la dépasse");
        assert_eq!(day.compared_days, 2);
    }

    /// La fenêtre courte ne voit pas ce que la longue voit.
    #[test]
    fn a_short_window_forgets_the_old_peak() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-05-01", Some(900), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();

        let out = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-11",
            PlatformFilter::default(),
            RankBy::Downloads,
            Some(30),
        )
        .unwrap();
        assert_eq!(out.rows[0].rank, Some(1), "le pic de mai est hors fenêtre");
    }

    /// Classer sur les revenus classe sur les revenus, pas sur les téléchargements.
    #[test]
    fn ranking_on_revenue_ignores_downloads() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(1000), None, Some("0.10")).unwrap();
        upsert_daily(&conn, m, "2026-08-11", Some(10), None, Some("5.00")).unwrap();

        let out = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-12",
            PlatformFilter::default(),
            RankBy::Revenue,
            None,
        )
        .unwrap();
        let rank_of = |day: &str| out.rows.iter().find(|r| r.day == day).unwrap().rank;
        assert_eq!(rank_of("2026-08-11"), Some(1), "la journée la mieux payée est première");
        assert_eq!(rank_of("2026-08-10"), Some(2));
    }

    /// Les journées sont rendues dans l'ordre du temps : c'est ce que le
    /// graphique attend, et le tableau les retrie comme il veut.
    #[test]
    fn rows_are_returned_in_chronological_order() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-12", Some(300), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();

        let out = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-13",
            PlatformFilter::default(),
            RankBy::Downloads,
            Some(RANK_WINDOW_DAYS),
        )
        .unwrap();
        let days: Vec<&str> = out.rows.iter().map(|r| r.day.as_str()).collect();
        assert_eq!(days, vec!["2026-08-10", "2026-08-12"]);
    }

    /// Les deux plateformes n'ont pas commencé le même jour : la page doit
    /// pouvoir le dire, sinon les vieux totaux paraissent effondrés.
    #[test]
    fn the_first_day_of_each_platform_is_reported() {
        let (conn, m, c) = seed();
        upsert_daily(&conn, c, "2024-03-19", Some(10), None, None).unwrap();
        upsert_daily(&conn, m, "2025-08-11", Some(20), None, None).unwrap();

        let out = day_rankings(
            &conn,
            "2024-03-19",
            "2026-08-13",
            PlatformFilter::default(),
            RankBy::Downloads,
            Some(RANK_WINDOW_DAYS),
        )
        .unwrap();
        assert_eq!(out.first_curseforge_day.as_deref(), Some("2024-03-19"));
        assert_eq!(out.first_modrinth_day.as_deref(), Some("2025-08-11"));
    }
}
