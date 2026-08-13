//! Confrontation de la base locale aux chiffres que Modrinth annonce.
//!
//! Ignoré par défaut : il interroge le compte de l'utilisateur avec son propre
//! token, en lecture seule. Il ne s'exécute que sur demande explicite, avec
//! `cargo test --test verify_against_api -- --ignored --nocapture`.
//!
//! Ce qu'il éprouve : chaque journée enregistrée dans `metrics_daily` doit
//! valoir celle que l'API rend pour la même journée. Un écart signalerait une
//! collecte fautive — un décalage de fuseau, un jour recopié au mauvais
//! endroit, un total pris pour un quotidien.

use chartographer_lib::config;
use chartographer_lib::providers::modrinth::ModrinthClient;
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use rusqlite::Connection;
use std::collections::BTreeMap;

fn data_dir() -> std::path::PathBuf {
    std::path::Path::new(&std::env::var("APPDATA").expect("APPDATA")).join("fr.dreykaoas.chartographer")
}

/// D'où vient le nombre d'abonnés Modrinth, et vaut-il ce que l'API annonce.
///
/// Modrinth ne totalise rien : il donne un compte d'abonnés par projet, et
/// c'est leur somme qui fait le chiffre affiché. Deux pièges s'y cachent — les
/// projets archivés, qu'il ne faut pas compter, et ceux qui ne sont pas
/// publics, que l'API ne rend qu'avec un token.
#[tokio::test]
#[ignore = "interroge le compte Modrinth de l'utilisateur"]
async fn followers_are_the_sum_of_every_project() {
    let dir = data_dir();
    let session = config::load_session(&dir).expect("aucune session Modrinth");
    let client = ModrinthClient::new(&session.token).expect("client");

    let projects = client.projects(&session.user_id).await.expect("projets");
    println!("projet                     abonnés (API)");
    let mut api_total = 0i64;
    for project in &projects {
        let count = project.followers;
        api_total += count;
        println!("  {:26} {count:5}", project.title);
    }
    println!("\ntotal API : {api_total} sur {} projets", projects.len());

    let conn = Connection::open(dir.join("chartographer.db")).expect("base");
    let base_total: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(followers), 0) FROM projects
             WHERE archived_at IS NULL AND platform = 'modrinth'",
            [],
            |r| r.get(0),
        )
        .expect("somme");
    let counted: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE archived_at IS NULL AND platform = 'modrinth'",
            [],
            |r| r.get(0),
        )
        .expect("compte");
    println!("total base : {base_total} sur {counted} projets");

    assert_eq!(
        api_total, base_total,
        "le chiffre affiché doit être la somme de ce que l'API annonce"
    );
}

#[tokio::test]
#[ignore = "interroge le compte Modrinth de l'utilisateur"]
async fn local_days_match_what_modrinth_answers() {
    let dir = data_dir();
    let session = config::load_session(&dir).expect("aucune session Modrinth");
    let client = ModrinthClient::new(&session.token).expect("client");

    let projects = client.projects(&session.user_id).await.expect("projets");
    let ids: Vec<String> = projects.iter().map(|p| p.id.clone()).collect();
    println!("{} projets Modrinth interrogés", ids.len());

    // Quinze jours suffisent à voir un décalage : au-delà, l'API impose des
    // lots et l'appel s'allonge sans rien apprendre de plus.
    //
    // La borne basse tombe à minuit, jamais à l'heure courante : demander
    // « depuis il y a quinze jours » ramenait une première journée amputée de
    // ses premières heures, qu'aucune base ne pouvait égaler.
    let end = Utc::now();
    let first: NaiveDate = end.date_naive() - Duration::days(14);
    let start = Utc.from_utc_datetime(&first.and_hms_opt(0, 0, 0).expect("minuit"));
    let series = client
        .analytics_downloads(&ids, start, end)
        .await
        .expect("analytics");

    // L'API rend une série par projet, indexée par horodatage. On la replie sur
    // des journées, comme la collecte le fait.
    let mut remote: BTreeMap<String, i64> = BTreeMap::new();
    for points in series.values() {
        for (stamp, value) in points {
            let day = Utc
                .timestamp_opt(*stamp, 0)
                .single()
                .map(|d| d.format("%Y-%m-%d").to_string());
            if let Some(day) = day {
                *remote.entry(day).or_default() += *value;
            }
        }
    }

    let conn = Connection::open(dir.join("chartographer.db")).expect("base");
    let mut stmt = conn
        .prepare(
            "SELECT m.day, COALESCE(SUM(m.downloads), 0) FROM metrics_daily m
             JOIN projects p ON p.id = m.project_id
             WHERE p.platform = 'modrinth' AND m.day >= ?1
             GROUP BY m.day ORDER BY m.day",
        )
        .expect("requete");
    let local: BTreeMap<String, i64> = stmt
        .query_map([start.format("%Y-%m-%d").to_string()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .expect("lecture")
        .map(|row| row.expect("ligne"))
        .collect();

    println!("\njour         base    API    écart");
    let mut worst = 0i64;
    let today = Utc::now().format("%Y-%m-%d").to_string();
    for (day, api) in &remote {
        let base = local.get(day).copied().unwrap_or(0);
        let gap = base - api;
        // La journée en cours n'est comparable à rien : les deux côtés la
        // relèvent à des instants différents, et elle grandit encore.
        let mark = if *day == today {
            "  (jour en cours)"
        } else {
            if gap.abs() > worst {
                worst = gap.abs();
            }
            ""
        };
        println!("{day} {base:7} {api:6} {gap:+7}{mark}");
    }

    let dates: Vec<&String> = remote.keys().collect();
    println!(
        "\nfenêtre comparée : {} → {}",
        dates.first().map(|d| d.as_str()).unwrap_or("—"),
        dates.last().map(|d| d.as_str()).unwrap_or("—")
    );
    println!("écart maximal hors jour en cours : {worst}");
    assert_eq!(worst, 0, "la base s'écarte de ce que Modrinth annonce");
}
