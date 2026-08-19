//! Sondage des adresses Modrinth qui pourraient nommer les abonnés.
//!
//! Ignoré par défaut : il interroge le compte de l'utilisateur avec son propre
//! token, en lecture seule. `cargo test --test probe_modrinth -- --ignored
//! --nocapture`.
//!
//! Une adresse absente rend 404, une adresse existante mais fermée rend 401 ou
//! 403 : la distinction compte, c'est elle qui dit si la donnée existe et nous
//! est seulement refusée, ou si elle n'existe pas du tout.

use chartographer_lib::config;

fn data_dir() -> std::path::PathBuf {
    std::path::Path::new(&std::env::var("APPDATA").expect("APPDATA"))
        .join("fr.dreykaoas.chartographer")
}

#[tokio::test]
#[ignore = "interroge le compte Modrinth de l'utilisateur"]
async fn look_for_a_followers_listing() {
    let session = config::load_session(&data_dir()).expect("aucune session Modrinth");
    let http = reqwest::Client::builder()
        .user_agent("chartographer/0.1 (sondage)")
        .build()
        .expect("client");

    // Un projet du compte sert de cobaye : les adresses par projet sont les
    // plus plausibles, un abonnement se prenant sur un projet.
    let projects: serde_json::Value = http
        .get(format!(
            "https://api.modrinth.com/v2/user/{}/projects",
            session.user_id
        ))
        .header("Authorization", &session.token)
        .send()
        .await
        .expect("projets")
        .json()
        .await
        .expect("json");
    let project = projects[0]["id"].as_str().expect("un projet").to_string();
    let slug = projects[0]["slug"].as_str().unwrap_or("").to_string();
    println!("projet sondé : {project} ({slug})\n");

    let candidates = vec![
        format!("https://api.modrinth.com/v2/project/{project}/followers"),
        format!("https://api.modrinth.com/v3/project/{project}/followers"),
        format!("https://api.modrinth.com/v3/project/{project}/follows"),
        format!("https://api.modrinth.com/v3/project/{project}/follower"),
        format!("https://api.modrinth.com/v2/project/{project}/follow"),
        format!(
            "https://api.modrinth.com/v3/user/{}/followers",
            session.user_id
        ),
        format!(
            "https://api.modrinth.com/v2/user/{}/followers",
            session.user_id
        ),
        format!(
            "https://api.modrinth.com/v3/user/{}/follows",
            session.user_id
        ),
        // Les analytics couvrent téléchargements, vues et revenus : on demande
        // la même forme pour les abonnés, au cas où elle existerait.
        format!(
            "https://api.modrinth.com/v3/analytics/followers?project_ids=%5B%22{project}%22%5D"
        ),
    ];

    for url in candidates {
        let response = http
            .get(&url)
            .header("Authorization", &session.token)
            .send()
            .await;
        match response {
            Ok(r) => {
                let status = r.status().as_u16();
                let body = r.text().await.unwrap_or_default();
                let head: String = body.chars().take(120).collect();
                println!("{status}  {url}");
                if status != 404 {
                    println!("      {head}");
                }
            }
            Err(e) => println!("---  {url}  ({e})"),
        }
    }
}
