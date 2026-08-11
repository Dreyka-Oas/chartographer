//! Épreuve de la publication sur les vraies plateformes.
//!
//! Ignoré par défaut : il crée puis efface des objets sur les comptes de
//! l'utilisateur. Il ne s'exécute que sur demande explicite, avec
//! `cargo test --test publish_live -- --ignored --nocapture`.
//!
//! Ce qu'il vérifie, dans l'ordre : créer un projet, y déposer une version,
//! supprimer la version, supprimer le projet. Rien ne doit rester après lui.

use chartographer_lib::config;
use chartographer_lib::providers::curseforge_upload::UploadClient;
use chartographer_lib::providers::modrinth::ModrinthClient;
use chartographer_lib::publish::{self, Draft};

fn data_dir() -> std::path::PathBuf {
    std::path::Path::new(&std::env::var("APPDATA").expect("APPDATA"))
        .join("fr.dreykaoas.chartographer")
}

/// Archive minimale mais valide : l'enregistrement de fin d'un ZIP vide.
/// Les deux plateformes acceptent une archive, pas un fichier quelconque.
fn empty_jar() -> Vec<u8> {
    let mut bytes = vec![0x50, 0x4b, 0x05, 0x06];
    bytes.extend(std::iter::repeat_n(0u8, 18));
    bytes
}

#[tokio::test]
#[ignore = "publie réellement sur les comptes de l'utilisateur"]
async fn modrinth_full_cycle() {
    let session = config::load_session(&data_dir()).expect("aucune session Modrinth");
    let client = ModrinthClient::new(&session.token).expect("client");

    // Nom unique : un essai précédent ne doit jamais bloquer celui-ci.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let slug = format!("chartographer-essai-{stamp}");

    let data = serde_json::json!({
        "slug": slug,
        "title": "Essai Chartographer",
        "description": "Projet d'essai créé par Chartographer, supprimé aussitôt.",
        "body": "Projet d'essai. Il ne doit pas subsister.",
        "project_type": "mod",
        "categories": ["utility"],
        "client_side": "required",
        "server_side": "required",
        "license_id": "MIT",
        "is_draft": true,
        "initial_versions": [],
    })
    .to_string();

    // Le même jeton en lecture : sépare un défaut d'authentification d'un
    // manque de portée, que Modrinth signale du même code.
    match client.me().await {
        Ok(user) => println!("lecture : session valide pour {}", user.username),
        Err(error) => println!("lecture : refusée aussi · {error}"),
    }

    let (status, body) = client.create_project(&data).await.expect("appel");
    println!("création du projet : HTTP {status}");
    if !(200..300).contains(&status) {
        println!(
            "refus : {}",
            publish::refusal_reason(&body).unwrap_or_else(|| body.chars().take(300).collect())
        );
        panic!("le jeton n'a pas la portée PROJECT_CREATE, ou la demande est refusée");
    }
    let project_id = publish::modrinth_version_id(&body).expect("identifiant du projet");
    println!("projet créé : {project_id}");

    let draft = Draft {
        modrinth_project_id: Some(project_id.clone()),
        curseforge_project_id: None,
        name: "Essai 0.0.1".into(),
        version_number: "0.0.1".into(),
        changelog: "Version d'essai, supprimée aussitôt.".into(),
        game_versions: vec!["1.21.1".into()],
        loaders: vec!["fabric".into()],
        release_type: "alpha".into(),
        manual_release: false,
    };
    let (status, body) = client
        .create_version(&draft.modrinth_data(), "essai-0.0.1.jar", &empty_jar())
        .await
        .expect("appel");
    println!("dépôt de la version : HTTP {status}");
    let version_id = if (200..300).contains(&status) {
        let id = publish::modrinth_version_id(&body).expect("identifiant de version");
        println!("version déposée : {id}");
        Some(id)
    } else {
        println!(
            "refus : {}",
            publish::refusal_reason(&body).unwrap_or_else(|| body.chars().take(300).collect())
        );
        None
    };

    if let Some(id) = &version_id {
        let (status, body) = client.delete_version(id).await.expect("appel");
        println!("suppression de la version : HTTP {status}");
        assert!(
            (200..300).contains(&status),
            "la version doit disparaître : {body}"
        );
    }

    let (status, body) = client.delete_project(&project_id).await.expect("appel");
    println!("suppression du projet : HTTP {status}");
    assert!(
        (200..300).contains(&status),
        "le projet d'essai doit disparaître : {body}"
    );

    assert!(
        version_id.is_some(),
        "le dépôt de version a échoué ; le projet a bien été nettoyé"
    );
}

#[tokio::test]
#[ignore = "interroge CurseForge avec le jeton d'envoi de l'utilisateur"]
async fn curseforge_authenticates_and_validates() {
    let token = match config::load_settings(&data_dir()).curseforge_upload_token {
        Some(token) => token,
        None => {
            println!("aucun jeton d'envoi enregistré : relève-le depuis les réglages");
            return;
        }
    };
    let client = UploadClient::new(&token).expect("client");

    // Le catalogue prouve à la fois que le jeton vaut et que l'interface répond.
    let catalogue = client.game_versions().await.expect("catalogue");
    println!("catalogue : {} entrées", catalogue.len());
    assert!(catalogue.len() > 50, "le catalogue paraît trop court");

    let (ids, missing) =
        publish::resolve_game_versions(&["1.21.1".to_string(), "Fabric".to_string()], &catalogue);
    println!("1.21.1 + Fabric → {ids:?} · non reconnues : {missing:?}");
    assert!(!ids.is_empty(), "aucune version reconnue dans le catalogue");

    // Envoi volontairement incomplet : il doit être refusé, sans rien déposer.
    // C'est la seule façon d'éprouver la porte sans publier un fichier qui ne
    // pourrait plus être retiré par cette interface.
    let (status, body) = client
        .upload(1002185, "{}", "essai.jar", &empty_jar())
        .await
        .expect("appel");
    println!(
        "envoi incomplet : HTTP {status} · {}",
        body.chars().take(200).collect::<String>()
    );
    assert!(
        (400..500).contains(&status),
        "un envoi sans métadonnées doit être refusé, pas accepté"
    );
}
