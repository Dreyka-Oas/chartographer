//! Rang d'une journée parmi celles qu'on lui choisit pour comparaison.
//!
//! Le classement vit à part des autres requêtes parce qu'il répond à une autre
//! question : non pas « combien », mais « était-ce un bon jour ». Deux façons
//! d'y répondre ne regardent jamais en avant — « était-ce un bon jour quand il
//! s'est produit » — deux autres comparent un groupe de journées à lui-même,
//! sans égard à leur ordre. `RankScope` documente les quatre.

use crate::error::Result;
use crate::models::{DayRankRow, DayRankings, Platform, RankBy, RankScope, RankSource};
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
        // Même principe que `cf_points_gained`, mais pour toute la plage en une
        // seule requête : les relevés sont rares (quatre dans la base réelle),
        // les jours qui séparent deux relevés ne le sont pas. Boucler sur les
        // jours faisait dépendre le coût de la largeur de la plage plutôt que
        // du nombre de relevés — un gel assuré dès que la plage remonte loin.
        let mut stmt = conn.prepare(
            "SELECT day, points FROM cf_points
             WHERE day < ?2
               AND (day >= ?1 OR day = (SELECT MAX(day) FROM cf_points WHERE day < ?1))
             ORDER BY day",
        )?;
        let readings = stmt
            .query_map(params![from, to], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for pair in readings.windows(2) {
            // Une baisse de solde est un retrait, jamais un gain négatif : la
            // même règle qu'à côté, dans `cf_points_gained`.
            let gained = (pair[1].1 - pair[0].1).max(0);
            if gained != 0 {
                out.entry(pair[1].0.clone()).or_default().1 += Decimal::from(gained) * point_value();
            }
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

/// Montant en centièmes : deux journées à un centime près restent distinctes,
/// et rien ne se perd à l'échelle où `rank_within` compare des entiers.
fn cents(amount: Decimal) -> i64 {
    (amount * Decimal::from(100)).round().to_string().parse().unwrap_or(0)
}

/// Valeur classée d'une journée, selon le critère et la source demandés.
///
/// `by` choisit la métrique — téléchargements ou revenus — et `source` la
/// plateforme qui l'alimente. Les deux sont des axes indépendants : croiser
/// leurs deux valeurs suffit à couvrir les six classements possibles, sans
/// qu'il faille écrire une variante par combinaison.
fn ranked_value(
    point: &crate::models::TimelinePoint,
    revenue: &(Decimal, Decimal),
    by: RankBy,
    source: RankSource,
) -> i64 {
    match (by, source) {
        (RankBy::Downloads, RankSource::Both) => point.modrinth + point.curseforge,
        (RankBy::Downloads, RankSource::Modrinth) => point.modrinth,
        (RankBy::Downloads, RankSource::CurseForge) => point.curseforge,
        (RankBy::Revenue, RankSource::Both) => cents(revenue.0 + revenue.1),
        (RankBy::Revenue, RankSource::Modrinth) => cents(revenue.0),
        (RankBy::Revenue, RankSource::CurseForge) => cents(revenue.1),
    }
}

/// `DayRankings` vide : aucune journée relevée, donc aucun classement à faire.
fn empty_rankings() -> DayRankings {
    DayRankings {
        rows: Vec::new(),
        first_modrinth_day: None,
        first_curseforge_day: None,
    }
}

/// Classement absolu : toutes les journées d'une même plage se comparent
/// entre elles, sans égard à leur ordre, contre une seule et même référence.
/// C'est le cœur commun à `Period`, dont la plage de comparaison est la
/// période affichée, et à `All`, dont la plage de comparaison est tout
/// l'historique — les deux ne diffèrent que par `pool_start`, jamais par la
/// règle de classement elle-même.
///
/// Ce que la comparaison regarde (`pool_start..to`) et ce que la page liste
/// (`rows_from..to`) sont deux choses distinctes : les boutons de période de
/// la barre du haut décident toujours ce qui s'affiche, la portée ne décide
/// que ce à quoi ça se compare. Pour `Period`, les deux plages sont les
/// mêmes ; pour `All`, la comparaison déborde largement ce qui est listé.
fn rank_against_pool(
    conn: &Connection,
    pool_start: &str,
    rows_from: &str,
    to: &str,
    filter: PlatformFilter,
    by: RankBy,
    source: RankSource,
) -> Result<DayRankings> {
    let history = timeline(conn, pool_start, to, filter)?;
    let revenue = revenue_by_day(conn, pool_start, to, filter)?;
    let values: Vec<i64> = history
        .iter()
        .map(|p| {
            let r = revenue.get(&p.day).copied().unwrap_or_default();
            ranked_value(p, &r, by, source)
        })
        .collect();

    let pool: Vec<i64> = values.iter().copied().filter(|v| *v > 0).collect();
    let mut rows = Vec::new();
    for (i, point) in history.iter().enumerate() {
        if point.day.as_str() < rows_from {
            continue;
        }
        let (modrinth_revenue, curseforge_revenue) =
            revenue.get(&point.day).copied().unwrap_or_default();
        rows.push(DayRankRow {
            day: point.day.clone(),
            modrinth: point.modrinth,
            curseforge: point.curseforge,
            total: point.modrinth + point.curseforge,
            revenue: (modrinth_revenue + curseforge_revenue).normalize().to_string(),
            rank: rank_within(&pool, values[i]),
            compared_days: pool.len() as i64,
        });
    }
    Ok(DayRankings {
        rows,
        first_modrinth_day: first_day(conn, Platform::Modrinth)?,
        first_curseforge_day: first_day(conn, Platform::CurseForge)?,
    })
}

/// Classement des journées d'une période, `to` exclu. Les lignes rendues
/// sont toujours celles de `[from, to)` — la période affichée en haut de
/// page — quelle que soit la portée demandée ; seule la comparaison qui
/// détermine leur rang change d'étendue.
///
/// Une seule question se pose à chaque journée : quel rang les réglages
/// demandés lui donnent-ils. `by` choisit la métrique — téléchargements ou
/// revenus — et `source` la plateforme qui l'alimente, indépendamment l'une
/// de l'autre. Classer sur une plateforme masquée par `filter` ne rend rien :
/// la colonne serait vide, un rang là-dessus ne voudrait rien dire. `scope`
/// choisit à quoi une journée se compare, et se répartit en deux familles :
///
/// - `Period` et `All` sont absolues : elles comparent un groupe de journées
///   à lui-même, sans égard à leur ordre, si bien qu'une journée peut y être
///   dépassée par une autre qui la suit. `Period` prend pour groupe la
///   période affichée ; `All`, tout l'historique — la même règle, sans autre
///   différence que la plage.
/// - `Sliding` et `Retrospective` sont rétrospectives : une journée n'y est
///   jugée que sur celles qui la précèdent, elle comprise, si bien qu'un rang
///   une fois acquis ne bouge plus jamais. `Sliding` regarde les
///   `window_days` journées qui précèdent ; `Retrospective`, sans borne
///   basse — c'est la même fenêtre, simplement non bornée.
pub fn day_rankings(
    conn: &Connection,
    from: &str,
    to: &str,
    filter: PlatformFilter,
    by: RankBy,
    source: RankSource,
    scope: RankScope,
    window_days: Option<i64>,
) -> Result<DayRankings> {
    // Classer sur une plateforme que le filtre du haut a masquée n'a pas de
    // sens : la colonne serait vide et le rang porterait sur des zéros.
    // Plutôt que d'inventer un classement sur rien, on ne rend rien — le
    // réglage choisit sur quoi classer *parmi ce qui est visible*, il ne
    // remplace pas le filtre de plateformes.
    let source_hidden = match source {
        RankSource::Modrinth => !filter.modrinth,
        RankSource::CurseForge => !filter.curseforge,
        RankSource::Both => false,
    };
    if source_hidden {
        return Ok(empty_rankings());
    }

    if scope == RankScope::Period {
        return rank_against_pool(conn, from, from, to, filter, by, source);
    }
    if scope == RankScope::All {
        // « Toutes les journées relevées » veut dire ce que la base contient
        // réellement, pas l'origine du calendrier. Ce que la page liste reste
        // la période affichée (`from`) ; seule la comparaison porte sur tout
        // l'historique (`pool_start`).
        return match crate::store::metrics::first_metrics_day(conn)? {
            Some(pool_start) => rank_against_pool(conn, &pool_start, from, to, filter, by, source),
            None => Ok(empty_rankings()),
        };
    }

    // `Sliding` et `Retrospective` : une fenêtre qui ne regarde jamais en
    // avant, la seconde n'étant qu'une fenêtre sans borne basse.
    let history_start = match scope {
        RankScope::Sliding => match window_days {
            Some(n) => Some(shift_day(from, -(n - 1))),
            None => crate::store::metrics::first_metrics_day(conn)?,
        },
        RankScope::Retrospective => crate::store::metrics::first_metrics_day(conn)?,
        RankScope::Period | RankScope::All => unreachable!("traitées plus haut"),
    };
    let Some(history_start) = history_start else {
        return Ok(empty_rankings());
    };
    let history = timeline(conn, &history_start, to, filter)?;
    let revenue = revenue_by_day(conn, &history_start, to, filter)?;

    let values: Vec<i64> = history
        .iter()
        .map(|p| {
            let r = revenue.get(&p.day).copied().unwrap_or_default();
            ranked_value(p, &r, by, source)
        })
        .collect();

    let bound = if scope == RankScope::Sliding { window_days } else { None };

    // Borne basse glissante de la fenêtre : elle n'avance jamais en arrière, ce
    // qui laisse le parcours linéaire au lieu de rouvrir la fenêtre à chaque jour.
    let mut rows = Vec::new();
    let mut start = 0usize;
    for (i, point) in history.iter().enumerate() {
        if point.day.as_str() < from {
            continue;
        }
        if let Some(n) = bound {
            let window_start = shift_day(&point.day, -(n - 1));
            while history[start].day < window_start {
                start += 1;
            }
        }
        let window: Vec<i64> = values[start..=i].iter().copied().filter(|v| *v > 0).collect();
        let compared_days = window.len() as i64;
        let rank = rank_within(&window, values[i]);
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

    /// Une plage large comme celle que produit `All` — de l'an zéro à
    /// aujourd'hui — doit rendre le même résultat qu'une plage étroite qui
    /// contient les mêmes relevés, et le rendre sans boucler sur les jours :
    /// c'est exactement le gel que la version précédente provoquait.
    #[test]
    fn a_wide_range_matches_a_narrow_one_and_does_not_loop_over_days() {
        let (conn, _, c) = seed();
        crate::store::metrics::record_cf_points(&conn, "2024-03-19", 1_000, "x").unwrap();
        crate::store::metrics::record_cf_points(&conn, "2025-01-10", 1_400, "x").unwrap();
        crate::store::metrics::record_cf_points(&conn, "2026-08-10", 1_900, "x").unwrap();
        crate::store::metrics::record_cf_points(&conn, "2026-08-14", 2_100, "x").unwrap();
        upsert_daily(&conn, c, "2024-03-19", Some(1), None, None).unwrap();

        let narrow =
            revenue_by_day(&conn, "2024-03-19", "2026-08-15", PlatformFilter::default()).unwrap();
        let wide =
            revenue_by_day(&conn, "0000-01-01", "2026-08-15", PlatformFilter::default()).unwrap();
        assert_eq!(wide, narrow, "même relevés, même résultat, quelle que soit la borne basse");
        assert_eq!(
            wide.get("2026-08-14").unwrap().1,
            Decimal::from(200) * point_value(),
            "200 points gagnés entre le 10 et le 14 aout"
        );
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
            RankSource::Both,
            RankScope::Sliding,
            Some(90),
        )
        .unwrap();
        let first = out.rows.iter().find(|r| r.day == "2026-08-10").unwrap();
        assert_eq!(first.rank, Some(1), "elle était première le jour même");
    }

    /// Sans borne basse, la comparaison porte sur tout l'historique antérieur —
    /// mais jamais sur ce qui suit : la règle « jamais en avant » vaut aussi
    /// pour cette fenêtre-là.
    #[test]
    fn scope_retrospective_compares_to_everything_before_never_after() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2024-01-01", Some(900), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();

        let out = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-11",
            PlatformFilter::default(),
            RankBy::Downloads,
            RankSource::Both,
            RankScope::Retrospective,
            None,
        )
        .unwrap();
        let day = &out.rows[0];
        assert_eq!(day.rank, Some(2), "la journée de 2024 la précède et la dépasse");
        assert_eq!(day.compared_days, 2);
    }

    /// Portées absolues et rétrospectives ne répondent pas à la même
    /// question. Deux journées, la seconde dépassant la première : en absolu,
    /// elles se comparent l'une à l'autre sans égard à l'ordre, donc la
    /// première recule quand la seconde arrive — rangs 2 puis 1. En
    /// rétrospectif, chacune ne juge que ce qui la précède : la première n'a
    /// personne devant elle, la seconde dépasse tout ce qu'elle voit — rangs
    /// 1 puis 1, et ni l'une ni l'autre ne bouge plus jamais après coup.
    #[test]
    fn absolute_scope_lets_a_later_day_demote_an_earlier_one() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-11", Some(500), None, None).unwrap();

        let out = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-12",
            PlatformFilter::default(),
            RankBy::Downloads,
            RankSource::Both,
            RankScope::All,
            None,
        )
        .unwrap();
        let rank_of = |day: &str| out.rows.iter().find(|r| r.day == day).unwrap().rank;
        assert_eq!(rank_of("2026-08-10"), Some(2), "dépassée par la journée qui la suit");
        assert_eq!(rank_of("2026-08-11"), Some(1));
    }

    #[test]
    fn retrospective_scope_never_lets_a_later_day_change_an_earlier_rank() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-11", Some(500), None, None).unwrap();

        let out = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-12",
            PlatformFilter::default(),
            RankBy::Downloads,
            RankSource::Both,
            RankScope::Retrospective,
            None,
        )
        .unwrap();
        let rank_of = |day: &str| out.rows.iter().find(|r| r.day == day).unwrap().rank;
        assert_eq!(rank_of("2026-08-10"), Some(1), "personne ne la précède le jour même");
        assert_eq!(rank_of("2026-08-11"), Some(1), "elle dépasse tout ce qu'elle voit derrière elle");
    }

    /// Ce que la page liste et ce à quoi elle compare sont deux choses
    /// distinctes. Un historique large, une période étroite au milieu : la
    /// portée `All` ne doit rendre que les journées de la période affichée —
    /// pas tout l'historique — mais chacune de ces lignes doit porter son
    /// rang parmi la totalité des journées relevées, pas seulement celles de
    /// la période. C'est le défaut que les boutons de date muets aurait
    /// laissé passer : la liste devait bouger avec eux, le dénominateur non.
    #[test]
    fn absolute_scope_lists_the_displayed_period_but_ranks_against_the_whole_history() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2024-01-01", Some(900), None, None).unwrap();
        upsert_daily(&conn, m, "2025-06-01", Some(50), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-11", Some(500), None, None).unwrap();

        let out = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-12",
            PlatformFilter::default(),
            RankBy::Downloads,
            RankSource::Both,
            RankScope::All,
            None,
        )
        .unwrap();
        assert_eq!(
            out.rows.len(),
            2,
            "seules les journées de la période affichée sont listées"
        );
        assert!(
            out.rows.iter().all(|r| r.compared_days == 4),
            "le dénominateur porte sur les quatre journées de tout l'historique"
        );
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
            RankSource::Both,
            RankScope::Sliding,
            Some(30),
        )
        .unwrap();
        assert_eq!(out.rows[0].rank, Some(1), "le pic de mai est hors fenêtre");
    }

    /// Classer sur les revenus classe sur les revenus, pas sur les
    /// téléchargements — les deux appels partagent le même jeu de données,
    /// seul `by` change.
    #[test]
    fn ranking_on_revenue_ignores_downloads() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(1000), None, Some("0.10")).unwrap();
        upsert_daily(&conn, m, "2026-08-11", Some(10), None, Some("5.00")).unwrap();
        upsert_daily(&conn, m, "2026-08-12", Some(2000), None, Some("0.50")).unwrap();

        let by_revenue = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-13",
            PlatformFilter::default(),
            RankBy::Revenue,
            RankSource::Both,
            RankScope::Retrospective,
            None,
        )
        .unwrap();
        let rank_of = |out: &DayRankings, day: &str| {
            out.rows.iter().find(|r| r.day == day).unwrap().rank
        };
        assert_eq!(rank_of(&by_revenue, "2026-08-10"), Some(1), "première journée relevée");
        assert_eq!(rank_of(&by_revenue, "2026-08-11"), Some(1), "dépasse le 10 en revenus");
        assert_eq!(
            rank_of(&by_revenue, "2026-08-12"),
            Some(2),
            "moins payée que le 11, malgré plus de téléchargements"
        );

        let by_downloads = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-13",
            PlatformFilter::default(),
            RankBy::Downloads,
            RankSource::Both,
            RankScope::Retrospective,
            None,
        )
        .unwrap();
        assert_eq!(
            rank_of(&by_downloads, "2026-08-12"),
            Some(1),
            "sur les téléchargements, le même jour passe première"
        );
    }

    /// La source décide vraiment, indépendamment de la métrique : une journée
    /// forte sur Modrinth et faible sur CurseForge doit changer de rang selon
    /// la plateforme choisie — l'inverse d'une journée forte sur CurseForge.
    #[test]
    fn source_decides_which_platform_the_ranking_reads() {
        let (conn, m, c) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(900), None, None).unwrap();
        upsert_daily(&conn, c, "2026-08-10", Some(10), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-11", Some(10), None, None).unwrap();
        upsert_daily(&conn, c, "2026-08-11", Some(900), None, None).unwrap();

        let rank_of = |source: RankSource, day: &str| {
            day_rankings(
                &conn,
                "2026-08-10",
                "2026-08-12",
                PlatformFilter::default(),
                RankBy::Downloads,
                source,
                RankScope::All,
                None,
            )
            .unwrap()
            .rows
            .into_iter()
            .find(|r| r.day == day)
            .unwrap()
            .rank
        };

        assert_eq!(rank_of(RankSource::Modrinth, "2026-08-10"), Some(1), "en tête sur Modrinth");
        assert_eq!(rank_of(RankSource::Modrinth, "2026-08-11"), Some(2));
        assert_eq!(
            rank_of(RankSource::CurseForge, "2026-08-11"),
            Some(1),
            "en tête sur CurseForge, l'inverse de Modrinth"
        );
        assert_eq!(rank_of(RankSource::CurseForge, "2026-08-10"), Some(2));
    }

    /// Classer sur une plateforme masquée par le filtre du haut n'a pas de
    /// sens : la colonne serait vide. Plutôt qu'un rang bâti sur des zéros,
    /// la commande ne rend rien.
    #[test]
    fn ranking_on_a_platform_hidden_by_the_filter_returns_nothing() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();
        let filter = PlatformFilter { modrinth: true, curseforge: false };

        let out = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-11",
            filter,
            RankBy::Downloads,
            RankSource::CurseForge,
            RankScope::All,
            None,
        )
        .unwrap();
        assert!(out.rows.is_empty());
    }

    /// Le classement sur la période compare les journées entre elles, sans
    /// égard à leur ordre : c'est la seule des trois fenêtres où une journée
    /// peut être dépassée par une qui la suit.
    #[test]
    fn scope_period_orders_the_whole_range_regardless_of_order() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-11", Some(500), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-12", Some(300), None, None).unwrap();

        let out = day_rankings(
            &conn,
            "2026-08-10",
            "2026-08-13",
            PlatformFilter::default(),
            RankBy::Downloads,
            RankSource::Both,
            RankScope::Period,
            None,
        )
        .unwrap();
        let rank_of = |day: &str| out.rows.iter().find(|r| r.day == day).unwrap().rank;
        assert_eq!(rank_of("2026-08-11"), Some(1));
        assert_eq!(rank_of("2026-08-12"), Some(2));
        assert_eq!(
            rank_of("2026-08-10"),
            Some(3),
            "dépassée par des journées qui la suivent, ce que Sliding et Retrospective interdisent"
        );
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
            RankSource::Both,
            RankScope::Sliding,
            Some(RANK_WINDOW_DAYS),
        )
        .unwrap();
        let days: Vec<&str> = out.rows.iter().map(|r| r.day.as_str()).collect();
        assert_eq!(days, vec!["2026-08-10", "2026-08-12"]);
    }

    /// Mesure ponctuelle sur la base réelle de l'utilisateur : la portée par
    /// défaut de la page, `All`, compare chaque journée listée à tout
    /// l'historique — les lignes rendues restent celles de la période
    /// affichée, mais leur rang va chercher sa place dans les 383 journées de
    /// la base. Ça doit répondre en un temps raisonnable, pas geler
    /// l'interface. Ouverte en lecture seule, jamais modifiée. Ignoré par
    /// défaut : le fichier n'existe pas sur les machines qui n'ont pas cette
    /// base.
    #[test]
    #[ignore]
    fn day_rankings_all_scope_answers_quickly_on_the_real_database() {
        let path = std::path::PathBuf::from(std::env::var("APPDATA").unwrap())
            .join("fr.dreykaoas.chartographer")
            .join("chartographer.db");
        if !path.exists() {
            eprintln!("base reelle absente, mesure ignoree");
            return;
        }
        let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY;
        let conn = Connection::open_with_flags(&path, flags).unwrap();

        let started = std::time::Instant::now();
        let out = day_rankings(
            &conn,
            "2026-08-01",
            "2026-08-15",
            PlatformFilter::default(),
            RankBy::Revenue,
            RankSource::Both,
            RankScope::All,
            None,
        )
        .unwrap();
        let elapsed = started.elapsed();

        eprintln!(
            "day_rankings(All) sur la base reelle : {} lignes en {:?}",
            out.rows.len(),
            elapsed
        );
        assert!(
            elapsed.as_secs() < 5,
            "la portee All doit repondre en quelques secondes, pas geler la page"
        );
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
            RankSource::Both,
            RankScope::Sliding,
            Some(RANK_WINDOW_DAYS),
        )
        .unwrap();
        assert_eq!(out.first_curseforge_day.as_deref(), Some("2024-03-19"));
        assert_eq!(out.first_modrinth_day.as_deref(), Some("2025-08-11"));
    }
}
