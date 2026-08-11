//! Inspection ponctuelle de la base réelle de l'application.
//! Ignoré par défaut : il ne s'exécute que sur demande explicite, via
//! `cargo test --test inspect_local_db -- --ignored --nocapture`.

use rusqlite::Connection;

fn db_path() -> Option<std::path::PathBuf> {
    let roaming = std::env::var("APPDATA").ok()?;
    let path = std::path::Path::new(&roaming)
        .join("fr.dreykaoas.chartographer")
        .join("chartographer.db");
    path.exists().then_some(path)
}

#[test]
#[ignore = "lit la base locale de l'utilisateur, pas un test de regression"]
fn resume_local_database() {
    let Some(path) = db_path() else {
        println!("aucune base locale");
        return;
    };
    let conn = Connection::open(path).expect("ouverture de la base");

    let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap_or(-1) };

    println!(
        "projets            : {}",
        count("SELECT COUNT(*) FROM projects")
    );
    println!(
        "  dont modrinth    : {}",
        count("SELECT COUNT(*) FROM projects WHERE platform = 'modrinth'")
    );
    println!(
        "  dont curseforge  : {}",
        count("SELECT COUNT(*) FROM projects WHERE platform = 'curseforge'")
    );
    println!(
        "liens              : {}",
        count("SELECT COUNT(*) FROM links")
    );
    println!(
        "jours de metriques : {}",
        count("SELECT COUNT(*) FROM metrics_daily")
    );
    println!(
        "lignes pays        : {}",
        count("SELECT COUNT(*) FROM countries_daily")
    );
    println!(
        "snapshots CF       : {}",
        count("SELECT COUNT(*) FROM cf_snapshots")
    );
    println!(
        "versions           : {}",
        count("SELECT COUNT(*) FROM versions")
    );
    println!(
        "evenements         : {}",
        count("SELECT COUNT(*) FROM events")
    );

    let payout: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'modrinth_payout'",
            [],
            |r| r.get(0),
        )
        .ok();
    match payout {
        Some(raw) => println!("\npayout brut       : {}", &raw[..raw.len().min(220)]),
        None => println!("\npayout            : jamais releve"),
    }

    let mut orphan_stmt = conn
        .prepare(
            "SELECT p.id, p.platform, p.title, p.slug FROM projects p
             WHERE (p.platform = 'modrinth'
                    AND p.id NOT IN (SELECT modrinth_project_id FROM links))
                OR (p.platform = 'curseforge'
                    AND p.id NOT IN (SELECT cf_project_id FROM links))
             ORDER BY p.platform, p.title",
        )
        .unwrap();
    println!("\nprojets sans jumeau :");
    for row in orphan_stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })
        .unwrap()
    {
        let (id, platform, title, slug) = row.unwrap();
        println!("  {id:4} {platform:12} {title:24} {}", slug.unwrap_or_default());
    }

    // Reproduit exactement ce que la commande `unlinked_projects` renvoie au
    // front, pour vérifier que la page des réglages reçoit bien la liste.
    {
        use chartographer_lib::models::Platform;
        use chartographer_lib::store::projects as p;

        let link_rows = p::links(&conn).unwrap();
        let payload: Vec<(i64, String, String)> = p::list(&conn)
            .unwrap()
            .into_iter()
            .filter(|project| match project.platform {
                Platform::Modrinth => !link_rows
                    .iter()
                    .any(|l| l.modrinth_project_id == project.id),
                Platform::CurseForge => !link_rows.iter().any(|l| l.cf_project_id == project.id),
            })
            .map(|project| {
                (
                    project.id,
                    project.platform.as_str().to_string(),
                    project.title,
                )
            })
            .collect();
        println!("\nunlinked_projects renvoie : {payload:?}");
    }

    let mut stmt = conn
        .prepare("SELECT provider, status, detail FROM sync_runs ORDER BY id DESC LIMIT 6")
        .unwrap();
    println!("\nderniers cycles :");
    for row in stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .unwrap()
    {
        let (provider, status, detail) = row.unwrap();
        println!("  {provider:22} {status:8} {detail}");
    }
}
