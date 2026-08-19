//! Abonnés CurseForge, tels que la fiche publique du compte les montre.
//!
//! La plateforme les classe du plus récent au plus ancien mais ne date aucun
//! abonnement. Ce que l'application peut en tirer, elle le tire du temps :
//! d'un relevé à l'autre, un nom qui apparaît est un nouvel abonné, un nom qui
//! disparaît est un départ. La date d'abonnement reste inconnue pour ceux qui
//! étaient déjà là au premier relevé — c'est une limite du site, pas un oubli,
//! et l'interface le dit plutôt que de la masquer.

use crate::error::Result;
use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Follower {
    pub name: String,
    pub avatar_url: Option<String>,
    /// Ancienneté du compte telle que le site l'écrit (« Member for 2 years »).
    pub seniority: Option<String>,
    /// Jour du premier relevé où ce nom est apparu.
    pub first_seen: String,
    pub last_seen: String,
    /// Jour où il a cessé de figurer, s'il est parti.
    pub lost_on: Option<String>,
    /// Rang dans le classement du site, le plus récent en tête.
    pub rank: i64,
    /// Faux tant qu'on ne l'a vu qu'au tout premier relevé : son abonnement est
    /// alors antérieur, sans qu'on sache de combien.
    pub arrival_known: bool,
}

/// Ce qu'un relevé rapporte d'un abonné, avant confrontation à la base.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Seen {
    pub name: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub seniority: Option<String>,
}

/// Jour du tout premier relevé, s'il y en a eu un.
///
/// Il sert de repère : tout abonné dont la première apparition tombe ce jour-là
/// était déjà présent avant que l'application ne regarde.
pub fn first_survey(conn: &Connection) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT MIN(first_seen) FROM cf_followers", [], |r| {
            r.get::<_, Option<String>>(0)
        })
        .unwrap_or(None))
}

/// Confronte un relevé à ce qui est connu, et rend le nombre d'arrivées et de
/// départs constatés ce jour-là.
///
/// `owner` est le compte dont on lit la fiche : son propre lien figure en tête
/// de page, et une première version du relevé le comptait parmi ses abonnés.
/// On l'écarte ici aussi, pour effacer ce qu'elle a pu laisser.
pub fn record(conn: &Connection, day: &str, owner: &str, seen: &[Seen]) -> Result<(usize, usize)> {
    conn.execute(
        "DELETE FROM cf_followers WHERE LOWER(name) = LOWER(?1)",
        params![owner],
    )?;
    let known_before = first_survey(conn)?.is_some();
    let mut arrived = 0usize;

    for (index, entry) in seen.iter().enumerate() {
        if entry.name.eq_ignore_ascii_case(owner) {
            continue;
        }
        let existed: bool = conn
            .query_row(
                "SELECT 1 FROM cf_followers WHERE name = ?1",
                params![entry.name],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !existed && known_before {
            arrived += 1;
        }
        conn.execute(
            "INSERT INTO cf_followers (name, avatar_url, seniority, first_seen, last_seen, lost_on, rank)
             VALUES (?1, ?2, ?3, ?4, ?4, NULL, ?5)
             ON CONFLICT(name) DO UPDATE SET
               avatar_url = COALESCE(excluded.avatar_url, cf_followers.avatar_url),
               seniority = COALESCE(excluded.seniority, cf_followers.seniority),
               last_seen = excluded.last_seen,
               -- Un retour efface le départ : il est de nouveau là.
               lost_on = NULL,
               rank = excluded.rank",
            params![
                entry.name,
                entry.avatar_url,
                entry.seniority,
                day,
                index as i64
            ],
        )?;
    }

    // Ce qui ne figure plus dans le relevé du jour a été perdu. Un relevé vide
    // ne prouve rien — une page mal chargée en donnerait autant : on ne marque
    // aucun départ dans ce cas.
    let lost = if seen.is_empty() {
        0
    } else {
        conn.execute(
            "UPDATE cf_followers SET lost_on = ?1 WHERE last_seen < ?1 AND lost_on IS NULL",
            params![day],
        )?
    };

    Ok((arrived, lost))
}

/// Un jour de la courbe des abonnés, plateforme par plateforme.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FollowerDay {
    pub day: String,
    pub modrinth: i64,
    pub curseforge: i64,
}

/// Note le compte du jour pour une plateforme.
///
/// Un second relevé le même jour remplace le premier : c'est le même compte,
/// mieux mesuré, pas une seconde mesure à additionner.
pub fn record_count(conn: &Connection, day: &str, platform: &str, count: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO followers_daily (day, platform, count) VALUES (?1, ?2, ?3)
         ON CONFLICT(day, platform) DO UPDATE SET count = excluded.count",
        params![day, platform, count.max(0)],
    )?;
    Ok(())
}

/// La courbe, du plus ancien relevé au plus récent.
///
/// Les jours sans relevé ne sont pas inventés : ils manquent, tout simplement.
/// Une plateforme absente d'un jour donné y vaut zéro, faute de mieux — c'est
/// le seul endroit où l'on écrit un chiffre qu'on n'a pas mesuré, et il ne
/// concerne que l'affichage.
pub fn history(conn: &Connection) -> Result<Vec<FollowerDay>> {
    let mut stmt = conn.prepare(
        "SELECT day,
                COALESCE(SUM(CASE WHEN platform = 'modrinth' THEN count END), 0),
                COALESCE(SUM(CASE WHEN platform = 'curseforge' THEN count END), 0)
         FROM followers_daily GROUP BY day ORDER BY day",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(FollowerDay {
                day: r.get(0)?,
                modrinth: r.get(1)?,
                curseforge: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Les abonnés connus, les plus récents d'abord, ceux qui sont partis à la fin.
pub fn list(conn: &Connection) -> Result<Vec<Follower>> {
    let first = first_survey(conn)?;
    let mut stmt = conn.prepare(
        "SELECT name, avatar_url, seniority, first_seen, last_seen, lost_on, rank
         FROM cf_followers
         ORDER BY lost_on IS NOT NULL, rank",
    )?;
    let rows = stmt
        .query_map([], |r| {
            let first_seen: String = r.get(3)?;
            Ok(Follower {
                name: r.get(0)?,
                avatar_url: r.get(1)?,
                seniority: r.get(2)?,
                arrival_known: first.as_deref() != Some(first_seen.as_str()),
                first_seen,
                last_seen: r.get(4)?,
                lost_on: r.get(5)?,
                rank: r.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::store::schema::migrate(&conn).unwrap();
        conn
    }

    fn seen(names: &[&str]) -> Vec<Seen> {
        names
            .iter()
            .map(|name| Seen {
                name: (*name).to_string(),
                avatar_url: None,
                seniority: None,
            })
            .collect()
    }

    /// Au premier relevé, personne n'est un nouvel abonné : on découvre l'état
    /// des lieux, on n'assiste pas à des arrivées.
    #[test]
    fn the_first_survey_counts_nobody_as_new() {
        let conn = base();
        let (arrived, lost) = record(&conn, "2026-08-13", "moi", &seen(&["ana", "bob"])).unwrap();
        assert_eq!((arrived, lost), (0, 0));
        assert!(list(&conn).unwrap().iter().all(|f| !f.arrival_known));
    }

    #[test]
    fn a_name_that_appears_later_is_a_new_follower() {
        let conn = base();
        record(&conn, "2026-08-13", "moi", &seen(&["ana"])).unwrap();
        let (arrived, lost) = record(&conn, "2026-08-14", "moi", &seen(&["cyd", "ana"])).unwrap();
        assert_eq!((arrived, lost), (1, 0));

        let all = list(&conn).unwrap();
        let cyd = all.iter().find(|f| f.name == "cyd").unwrap();
        assert_eq!(cyd.first_seen, "2026-08-14");
        assert!(cyd.arrival_known, "son arrivée a été constatée");
    }

    #[test]
    fn a_name_that_stops_appearing_is_marked_lost() {
        let conn = base();
        record(&conn, "2026-08-13", "moi", &seen(&["ana", "bob"])).unwrap();
        let (_, lost) = record(&conn, "2026-08-14", "moi", &seen(&["ana"])).unwrap();
        assert_eq!(lost, 1);

        let all = list(&conn).unwrap();
        let bob = all.iter().find(|f| f.name == "bob").unwrap();
        assert_eq!(bob.lost_on.as_deref(), Some("2026-08-14"));
        // Les partis passent en fin de liste.
        assert_eq!(all.last().unwrap().name, "bob");
    }

    /// Une page qui n'a rien rendu ne dit pas que tout le monde est parti.
    #[test]
    fn an_empty_survey_never_declares_a_departure() {
        let conn = base();
        record(&conn, "2026-08-13", "moi", &seen(&["ana", "bob"])).unwrap();
        let (arrived, lost) = record(&conn, "2026-08-14", "moi", &[]).unwrap();
        assert_eq!((arrived, lost), (0, 0));
        assert!(list(&conn).unwrap().iter().all(|f| f.lost_on.is_none()));
    }

    #[test]
    fn a_returning_follower_loses_its_departure() {
        let conn = base();
        record(&conn, "2026-08-13", "moi", &seen(&["ana", "bob"])).unwrap();
        record(&conn, "2026-08-14", "moi", &seen(&["ana"])).unwrap();
        record(&conn, "2026-08-15", "moi", &seen(&["ana", "bob"])).unwrap();

        let all = list(&conn).unwrap();
        let bob = all.iter().find(|f| f.name == "bob").unwrap();
        assert!(bob.lost_on.is_none());
        assert_eq!(
            bob.first_seen, "2026-08-13",
            "sa venue d'origine est gardée"
        );
    }

    /// Le compte lui-même n'est pas son propre abonné : son lien figure en tête
    /// de sa fiche, et une première version du relevé l'y comptait.
    #[test]
    fn the_account_is_never_its_own_follower() {
        let conn = base();
        record(&conn, "2026-08-13", "moi", &seen(&["ana", "MOI"])).unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "ana");
    }

    #[test]
    fn the_curve_keeps_one_figure_per_day_and_platform() {
        let conn = base();
        record_count(&conn, "2026-08-12", "modrinth", 120).unwrap();
        record_count(&conn, "2026-08-12", "curseforge", 6).unwrap();
        // Second passage le même jour : il corrige, il n'ajoute pas.
        record_count(&conn, "2026-08-12", "modrinth", 121).unwrap();
        record_count(&conn, "2026-08-13", "modrinth", 123).unwrap();

        let days = history(&conn).unwrap();
        assert_eq!(days.len(), 2);
        assert_eq!(
            days[0],
            FollowerDay {
                day: "2026-08-12".into(),
                modrinth: 121,
                curseforge: 6,
            }
        );
        // Une plateforme non relevée ce jour-là vaut zéro, faute de mesure.
        assert_eq!(days[1].curseforge, 0);
    }
}
