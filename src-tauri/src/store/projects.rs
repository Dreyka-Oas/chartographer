use crate::error::Result;
use crate::models::Platform;
use rusqlite::{params, Connection};

#[derive(Debug, Clone)]
pub struct ProjectUpsert {
    pub platform: Platform,
    pub ext_id: String,
    pub slug: Option<String>,
    pub title: String,
    pub project_type: Option<String>,
    pub url: Option<String>,
    pub icon_url: Option<String>,
    pub created_at: Option<String>,
    pub total_downloads: i64,
    /// Abonnés, quand la source les compte. `None` veut dire "je ne sais
    /// pas" : le nombre déjà connu est alors conservé.
    pub followers: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ProjectRow {
    pub id: i64,
    pub platform: Platform,
    pub ext_id: String,
    pub slug: Option<String>,
    pub title: String,
    pub project_type: Option<String>,
    pub url: Option<String>,
    pub icon_url: Option<String>,
    pub created_at: Option<String>,
    pub total_downloads: i64,
    pub followers: i64,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LinkRow {
    pub modrinth_project_id: i64,
    pub cf_project_id: i64,
    pub confidence: f64,
    pub manual: bool,
}

pub fn upsert(conn: &Connection, p: &ProjectUpsert) -> Result<i64> {
    conn.execute(
        "INSERT INTO projects
           (platform, ext_id, slug, title, project_type, url, icon_url, created_at, total_downloads, followers, archived_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, COALESCE(?10, 0), NULL)
         ON CONFLICT(platform, ext_id) DO UPDATE SET
           slug = excluded.slug,
           title = excluded.title,
           project_type = excluded.project_type,
           url = COALESCE(excluded.url, projects.url),
           icon_url = COALESCE(excluded.icon_url, projects.icon_url),
           created_at = COALESCE(excluded.created_at, projects.created_at),
           total_downloads = excluded.total_downloads,
           -- Une source qui ne compte pas les abonnés envoie `None` : elle ne
           -- doit pas effacer ce qu'une autre a relevé. On relit le paramètre
           -- plutôt que `excluded`, que la valeur insérée a déjà ramené à zéro.
           followers = COALESCE(?10, projects.followers),
           archived_at = NULL",
        params![
            p.platform.as_str(),
            p.ext_id,
            p.slug,
            p.title,
            p.project_type,
            p.url,
            p.icon_url,
            p.created_at,
            p.total_downloads,
            p.followers
        ],
    )?;
    Ok(conn.query_row(
        "SELECT id FROM projects WHERE platform = ?1 AND ext_id = ?2",
        params![p.platform.as_str(), p.ext_id],
        |r| r.get(0),
    )?)
}

fn row_to_project(r: &rusqlite::Row) -> rusqlite::Result<ProjectRow> {
    Ok(ProjectRow {
        id: r.get(0)?,
        platform: Platform::from_str_lossy(&r.get::<_, String>(1)?),
        ext_id: r.get(2)?,
        slug: r.get(3)?,
        title: r.get(4)?,
        project_type: r.get(5)?,
        url: r.get(6)?,
        icon_url: r.get(7)?,
        created_at: r.get(8)?,
        total_downloads: r.get(9)?,
        followers: r.get(10)?,
        archived_at: r.get(11)?,
    })
}

const SELECT_PROJECT: &str = "SELECT id, platform, ext_id, slug, title, project_type, url, icon_url, created_at, total_downloads, followers, archived_at FROM projects";

pub fn list(conn: &Connection) -> Result<Vec<ProjectRow>> {
    let mut stmt = conn.prepare(&format!("{SELECT_PROJECT} ORDER BY title COLLATE NOCASE"))?;
    let rows = stmt
        .query_map([], row_to_project)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn list_by_platform(conn: &Connection, platform: Platform) -> Result<Vec<ProjectRow>> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT_PROJECT} WHERE platform = ?1 ORDER BY title COLLATE NOCASE"
    ))?;
    let rows = stmt
        .query_map(params![platform.as_str()], row_to_project)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn archive_missing(
    conn: &Connection,
    platform: Platform,
    seen_ext_ids: &[String],
    now: &str,
) -> Result<usize> {
    let placeholders = if seen_ext_ids.is_empty() {
        "''".to_string()
    } else {
        seen_ext_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",")
    };
    let sql = format!(
        "UPDATE projects SET archived_at = ?1
         WHERE platform = ?2 AND archived_at IS NULL AND ext_id NOT IN ({placeholders})"
    );
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(now.to_string()),
        Box::new(platform.as_str().to_string()),
    ];
    for id in seen_ext_ids {
        values.push(Box::new(id.clone()));
    }
    let refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    Ok(conn.execute(&sql, refs.as_slice())?)
}

pub fn upsert_link(
    conn: &Connection,
    modrinth_id: i64,
    cf_id: i64,
    confidence: f64,
    manual: bool,
) -> Result<()> {
    conn.execute(
        "INSERT INTO links (modrinth_project_id, cf_project_id, confidence, manual)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(modrinth_project_id, cf_project_id) DO UPDATE SET
           confidence = CASE WHEN links.manual = 1 THEN links.confidence ELSE excluded.confidence END,
           manual = CASE WHEN links.manual = 1 THEN 1 ELSE excluded.manual END",
        params![modrinth_id, cf_id, confidence, manual as i64],
    )?;
    Ok(())
}

/// Appariement manuel exclusif : un projet n'a qu'un seul jumeau. Les liens
/// existants des deux côtés sont donc effacés avant d'écrire le nouveau, sinon
/// un mod mal apparié resterait rattaché à son ancien jumeau.
pub fn link_exclusive(conn: &Connection, modrinth_id: i64, cf_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM links WHERE modrinth_project_id = ?1 OR cf_project_id = ?2",
        params![modrinth_id, cf_id],
    )?;
    upsert_link(conn, modrinth_id, cf_id, 1.0, true)
}

/// Déclare un projet sans équivalent sur l'autre plateforme, ou annule cette
/// déclaration. Un projet marqué ainsi sort de la liste des appariements à faire.
#[cfg(test)]
mod follower_tests {
    use super::*;
    use crate::store::schema::migrate;

    fn base() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn fiche(followers: Option<i64>) -> ProjectUpsert {
        ProjectUpsert {
            platform: Platform::CurseForge,
            ext_id: "1002185".into(),
            slug: Some("mobs-blocker".into()),
            title: "Mobs Blocker".into(),
            project_type: None,
            url: None,
            icon_url: None,
            created_at: None,
            total_downloads: 86_904,
            followers,
        }
    }

    /// Une source qui ne compte pas les abonnés ne doit pas effacer ce qu'une
    /// autre a relevé : elle envoie `None`, pas zéro.
    #[test]
    fn a_source_that_ignores_followers_keeps_the_known_count() {
        let conn = base();
        upsert(&conn, &fiche(Some(42))).unwrap();
        upsert(&conn, &fiche(None)).unwrap();

        let row = list(&conn).unwrap().into_iter().next().unwrap();
        assert_eq!(row.followers, 42);
        assert_eq!(row.total_downloads, 86_904);
    }

    #[test]
    fn a_source_that_counts_them_writes_its_own_figure() {
        let conn = base();
        upsert(&conn, &fiche(Some(7))).unwrap();
        assert_eq!(list(&conn).unwrap()[0].followers, 7);
        upsert(&conn, &fiche(Some(3))).unwrap();
        assert_eq!(list(&conn).unwrap()[0].followers, 3);
    }
}

pub fn set_solo(conn: &Connection, id: i64, solo: bool) -> Result<()> {
    conn.execute(
        "UPDATE projects SET solo = ?2 WHERE id = ?1",
        params![id, solo as i64],
    )?;
    Ok(())
}

pub fn solo_ids(conn: &Connection) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT id FROM projects WHERE solo = 1")?;
    let rows = stmt
        .query_map([], |r| r.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn links(conn: &Connection) -> Result<Vec<LinkRow>> {
    let mut stmt =
        conn.prepare("SELECT modrinth_project_id, cf_project_id, confidence, manual FROM links")?;
    let rows = stmt
        .query_map([], |r| {
            Ok(LinkRow {
                modrinth_project_id: r.get(0)?,
                cf_project_id: r.get(1)?,
                confidence: r.get(2)?,
                manual: r.get::<_, i64>(3)? == 1,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn clear_automatic_links(conn: &Connection) -> Result<usize> {
    Ok(conn.execute("DELETE FROM links WHERE manual = 0", [])?)
}

pub fn delete_link(conn: &Connection, modrinth_id: i64, cf_id: i64) -> Result<usize> {
    Ok(conn.execute(
        "DELETE FROM links WHERE modrinth_project_id = ?1 AND cf_project_id = ?2",
        params![modrinth_id, cf_id],
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::schema::migrate;
    use rusqlite::Connection;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn sample(platform: Platform, ext: &str, title: &str) -> ProjectUpsert {
        ProjectUpsert {
            platform,
            ext_id: ext.into(),
            slug: Some(title.to_lowercase().replace(' ', "-")),
            title: title.into(),
            project_type: Some("mod".into()),
            url: None,
            icon_url: None,
            created_at: None,
            total_downloads: 100,
            followers: Some(2),
        }
    }

    #[test]
    fn upsert_inserts_then_updates_same_row() {
        let conn = db();
        let id = upsert(&conn, &sample(Platform::Modrinth, "abc", "Vein Vantage")).unwrap();
        let mut second = sample(Platform::Modrinth, "abc", "Vein Vantage");
        second.total_downloads = 999;
        let id2 = upsert(&conn, &second).unwrap();
        assert_eq!(id, id2);
        assert_eq!(list(&conn).unwrap().len(), 1);
        assert_eq!(list(&conn).unwrap()[0].total_downloads, 999);
    }

    #[test]
    fn same_ext_id_on_two_platforms_are_distinct_rows() {
        let conn = db();
        upsert(&conn, &sample(Platform::Modrinth, "abc", "A")).unwrap();
        upsert(&conn, &sample(Platform::CurseForge, "abc", "A")).unwrap();
        assert_eq!(list(&conn).unwrap().len(), 2);
    }

    #[test]
    fn archive_missing_flags_absent_projects_only() {
        let conn = db();
        upsert(&conn, &sample(Platform::Modrinth, "keep", "Keep")).unwrap();
        upsert(&conn, &sample(Platform::Modrinth, "gone", "Gone")).unwrap();
        let n = archive_missing(
            &conn,
            Platform::Modrinth,
            &["keep".into()],
            "2026-08-11T00:00:00Z",
        )
        .unwrap();
        assert_eq!(n, 1);
        let rows = list(&conn).unwrap();
        let gone = rows.iter().find(|r| r.ext_id == "gone").unwrap();
        let keep = rows.iter().find(|r| r.ext_id == "keep").unwrap();
        assert!(gone.archived_at.is_some());
        assert!(keep.archived_at.is_none());
    }

    #[test]
    fn upsert_link_replaces_automatic_but_never_manual() {
        let conn = db();
        let m = upsert(&conn, &sample(Platform::Modrinth, "m1", "A")).unwrap();
        let c = upsert(&conn, &sample(Platform::CurseForge, "c1", "A")).unwrap();
        upsert_link(&conn, m, c, 0.9, false).unwrap();
        upsert_link(&conn, m, c, 1.0, false).unwrap();
        assert_eq!(links(&conn).unwrap()[0].confidence, 1.0);
        upsert_link(&conn, m, c, 1.0, true).unwrap();
        upsert_link(&conn, m, c, 0.5, false).unwrap();
        let l = &links(&conn).unwrap()[0];
        assert!(l.manual);
        assert_eq!(l.confidence, 1.0);
    }

    #[test]
    fn clear_automatic_links_keeps_manual_ones() {
        let conn = db();
        let m = upsert(&conn, &sample(Platform::Modrinth, "m1", "A")).unwrap();
        let c1 = upsert(&conn, &sample(Platform::CurseForge, "c1", "A")).unwrap();
        let m2 = upsert(&conn, &sample(Platform::Modrinth, "m2", "B")).unwrap();
        let c2 = upsert(&conn, &sample(Platform::CurseForge, "c2", "B")).unwrap();
        upsert_link(&conn, m, c1, 0.9, false).unwrap();
        upsert_link(&conn, m2, c2, 1.0, true).unwrap();
        clear_automatic_links(&conn).unwrap();
        let rows = links(&conn).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].manual);
    }
}
