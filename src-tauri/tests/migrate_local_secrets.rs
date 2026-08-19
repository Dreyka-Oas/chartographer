//! Migration des jetons vers le trousseau, sur les fichiers de l'utilisateur.
//!
//! Ignoré par défaut : il touche au dossier de données réel et au trousseau du
//! système. `cargo test --test migrate_local_secrets -- --ignored --nocapture`.
//!
//! Relire la session et les réglages suffit à déclencher la migration : c'est
//! ce que fait l'application à chaque démarrage. Le test se contente donc de
//! lire, puis de constater que les fichiers ne portent plus de jeton et que le
//! trousseau, lui, les a.

use chartographer_lib::{config, secrets};

fn data_dir() -> std::path::PathBuf {
    std::path::Path::new(&std::env::var("APPDATA").expect("APPDATA"))
        .join("fr.dreykaoas.chartographer")
}

#[test]
#[ignore = "touche au dossier de donnees reel et au trousseau du systeme"]
fn local_files_hand_their_tokens_to_the_keyring() {
    let dir = data_dir();
    if !dir.join("session.json").exists() {
        eprintln!("aucune session locale : rien a migrer");
        return;
    }

    // Une lecture, exactement comme au demarrage de l'application.
    let session = config::load_session(&dir).expect("session lisible");
    assert!(
        !session.token.is_empty(),
        "la session doit rendre un jeton, qu'il vienne du fichier ou du trousseau"
    );
    let settings = config::load_settings(&dir);

    let raw_session = std::fs::read_to_string(dir.join("session.json")).unwrap();
    assert!(
        !raw_session.contains(&session.token),
        "session.json porte encore le jeton en clair"
    );
    assert!(
        !raw_session.contains("\"token\""),
        "session.json porte encore un champ token"
    );
    println!("session.json : plus de jeton sur le disque");

    assert_eq!(
        secrets::load(secrets::MODRINTH_TOKEN).unwrap().as_deref(),
        Some(session.token.as_str()),
        "le jeton Modrinth doit etre dans le trousseau"
    );
    println!("trousseau : jeton Modrinth present");

    if let Some(token) = settings.curseforge_upload_token.as_deref() {
        let raw_settings = std::fs::read_to_string(dir.join("settings.json")).unwrap();
        assert!(
            !raw_settings.contains(token),
            "settings.json porte encore le jeton d'envoi en clair"
        );
        assert_eq!(
            secrets::load(secrets::CURSEFORGE_UPLOAD_TOKEN)
                .unwrap()
                .as_deref(),
            Some(token),
            "le jeton d'envoi CurseForge doit etre dans le trousseau"
        );
        println!("settings.json : plus de jeton d'envoi sur le disque, trousseau garni");
    } else {
        println!("aucun jeton d'envoi CurseForge enregistre");
    }
}
