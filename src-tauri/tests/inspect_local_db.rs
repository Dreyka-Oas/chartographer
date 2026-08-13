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

    let mut days_stmt = conn
        .prepare(
            "SELECT substr(taken_at, 1, 10) AS day, COUNT(*) FROM cf_snapshots
             GROUP BY day ORDER BY day",
        )
        .unwrap();
    println!("\njours de snapshots CurseForge :");
    for row in days_stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .unwrap()
    {
        let (day, count) = row.unwrap();
        println!("  {day} : {count} relevés");
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
        println!(
            "  {id:4} {platform:12} {title:24} {}",
            slug.unwrap_or_default()
        );
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

    // Ce que la collecte du tableau de bord CurseForge a réellement déposé.
    println!("\nCurseForge collecté :");
    println!(
        "  jours mesures    : {}",
        count(
            "SELECT COUNT(DISTINCT m.day) FROM metrics_daily m
             JOIN projects p ON p.id = m.project_id WHERE p.platform = 'curseforge'"
        )
    );
    println!(
        "  mods couverts    : {}",
        count(
            "SELECT COUNT(DISTINCT m.project_id) FROM metrics_daily m
             JOIN projects p ON p.id = m.project_id WHERE p.platform = 'curseforge'"
        )
    );
    println!(
        "  soldes de points : {} · dernier {} points",
        count("SELECT COUNT(*) FROM cf_points"),
        count("SELECT points FROM cf_points ORDER BY day DESC LIMIT 1")
    );
    println!(
        "  mois de revenus  : {}",
        count("SELECT COUNT(*) FROM cf_revenue")
    );

    let mut cf_stmt = conn
        .prepare(
            "SELECT p.title, COUNT(*), MIN(m.day), MAX(m.day), SUM(m.downloads)
             FROM metrics_daily m JOIN projects p ON p.id = m.project_id
             WHERE p.platform = 'curseforge' GROUP BY p.id ORDER BY COUNT(*) DESC LIMIT 25",
        )
        .unwrap();
    for row in cf_stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })
        .unwrap()
    {
        let (title, days, from, to, total) = row.unwrap();
        println!("  {title:26} {days:4} j  {from} → {to}  {total} tel.");
    }

    let mut money_stmt = conn
        .prepare("SELECT month, amount_usd FROM cf_revenue ORDER BY month")
        .unwrap();
    for row in money_stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .unwrap()
    {
        let (month, amount) = row.unwrap();
        println!("  revenus {month} : {amount} $");
    }

    // D'où vient chaque journée du graphique : une mesure rapportée du tableau
    // de bord, ou l'écart entre deux snapshots. Les deux ne doivent jamais
    // compter la même journée.
    println!("\ndernières journées, source par source :");
    let mut mixed = conn
        .prepare(
            "SELECT m.day,
                    SUM(CASE WHEN p.platform = 'modrinth' THEN m.downloads ELSE 0 END),
                    SUM(CASE WHEN p.platform = 'curseforge' THEN m.downloads ELSE 0 END),
                    COUNT(DISTINCT CASE WHEN p.platform = 'curseforge' THEN p.id END)
             FROM metrics_daily m JOIN projects p ON p.id = m.project_id
             GROUP BY m.day ORDER BY m.day DESC LIMIT 12",
        )
        .unwrap();
    for row in mixed
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .unwrap()
    {
        let (day, modrinth, curseforge, mods) = row.unwrap();
        println!("  {day}  modrinth {modrinth:6}  curseforge {curseforge:6} ({mods} mods mesurés)");
    }

    println!("\nsnapshots CurseForge, cumul par mod et par jour :");
    let mut snaps = conn
        .prepare(
            "SELECT substr(s.taken_at, 1, 10) AS day, p.title, MAX(s.total_downloads)
             FROM cf_snapshots s JOIN projects p ON p.id = s.project_id
             GROUP BY day, p.id ORDER BY day, p.title LIMIT 40",
        )
        .unwrap();
    for row in snaps
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .unwrap()
    {
        let (day, title, total) = row.unwrap();
        println!("  {day}  {title:26} {total}");
    }

    println!("\nabonnés CurseForge :");
    println!(
        "  connus           : {}",
        count("SELECT COUNT(*) FROM cf_followers")
    );
    let mut who = conn
        .prepare("SELECT name, seniority, first_seen, rank FROM cf_followers ORDER BY rank")
        .unwrap();
    for row in who
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .unwrap()
    {
        let (name, seniority, first_seen, rank) = row.unwrap();
        println!(
            "  #{rank:<2} {name:28} {:24} vu le {first_seen}",
            seniority.unwrap_or_default()
        );
    }

    println!("\ncourbe des abonnés :");
    let mut curve = conn
        .prepare("SELECT day, platform, count FROM followers_daily ORDER BY day, platform")
        .unwrap();
    for row in curve
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .unwrap()
    {
        let (day, platform, count) = row.unwrap();
        println!("  {day}  {platform:12} {count}");
    }

    let mut metas = conn
        .prepare(
            "SELECT key, value FROM meta
             WHERE key LIKE 'curseforge_%' ORDER BY key",
        )
        .unwrap();
    for row in metas
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .unwrap()
    {
        let (key, value) = row.unwrap();
        println!("  {key:32} {}", &value[..value.len().min(60)]);
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
