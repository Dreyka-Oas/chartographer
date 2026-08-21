//! Ce que la dernière release publiée promet aux postes déjà installés.
//!
//! Ignoré par défaut : il sort sur le réseau et télécharge un installeur.
//! `cargo test --test verify_release -- --ignored --nocapture`.
//!
//! Le test refait, à la main, ce que l'application fait toute seule au
//! démarrage : lire `latest.json` à l'adresse inscrite dans la configuration,
//! y trouver la plateforme courante, ramener l'archive, et vérifier sa
//! signature avec la clé publique du projet. C'est la seule manière de savoir
//! qu'une release est installable sans installer une version périmée pour
//! voir. Trois choses peuvent casser sans que rien d'autre ne le dise : une
//! adresse que le socle ne sait pas suivre, une plateforme absente du
//! manifeste, une archive signée avec une autre clé.

use minisign_verify::{PublicKey, Signature};
use serde_json::Value;

/// Configuration de l'application, lue là où elle vit vraiment : le test doit
/// éprouver ce que le binaire publié embarque, pas une copie qui aurait dérivé.
fn config() -> Value {
    let raw = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tauri.conf.json"))
        .expect("tauri.conf.json");
    serde_json::from_str(&raw).expect("tauri.conf.json lisible")
}

/// Clé de plateforme telle que l'updater la cherche dans `latest.json`.
fn platform_key() -> &'static str {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "windows-x86_64"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "linux-x86_64"
    } else {
        panic!("plateforme non couverte par ce test")
    }
}

#[tokio::test]
#[ignore = "sort sur le reseau et telecharge un installeur"]
async fn the_published_release_is_installable() {
    let conf = config();
    let updater = &conf["plugins"]["updater"];
    let pubkey_b64 = updater["pubkey"].as_str().expect("pubkey");
    let endpoint = updater["endpoints"][0].as_str().expect("endpoint");
    assert!(
        endpoint.starts_with("https://"),
        "l'adresse des mises a jour doit etre en HTTPS : {endpoint}"
    );

    // La clé est rangée en base64 dans la configuration, et minisign l'attend
    // sous sa forme texte.
    let pubkey_text = String::from_utf8(
        base64_decode(pubkey_b64).expect("la cle publique doit etre du base64 valide"),
    )
    .expect("cle publique lisible");
    let pubkey = PublicKey::decode(&pubkey_text).expect("cle publique minisign valide");

    let http = reqwest::Client::builder()
        .user_agent("chartographer-release-test")
        .build()
        .unwrap();

    let manifest: Value = http
        .get(endpoint)
        .send()
        .await
        .expect("latest.json joignable")
        .error_for_status()
        .expect("latest.json doit repondre 200 : une release en brouillon rend 404")
        .json()
        .await
        .expect("latest.json est du JSON");

    let version = manifest["version"].as_str().expect("version annoncee");
    println!("derniere version publiee : {version}");

    let platform = &manifest["platforms"][platform_key()];
    assert!(
        !platform.is_null(),
        "aucune entree {} dans latest.json : les postes de cette plateforme ne verraient jamais de mise a jour",
        platform_key()
    );

    let url = platform["url"].as_str().expect("adresse de l'archive");
    assert!(
        !url.starts_with("https://api.github.com/"),
        "l'archive est annoncee par l'adresse de l'API ({url}) : sans en-tete \
         `Accept: application/octet-stream`, elle rend du JSON et non le binaire"
    );

    let archive = http
        .get(url)
        .send()
        .await
        .expect("archive joignable")
        .error_for_status()
        .expect("l'archive doit repondre 200")
        .bytes()
        .await
        .expect("archive lisible");
    println!("archive : {} octets depuis {url}", archive.len());
    assert!(
        archive.len() > 1_000_000,
        "archive suspecte, {} octets seulement, c'est la taille d'une page \
         d'erreur, pas d'un installeur",
        archive.len()
    );

    // La signature voyage en base64 dans `latest.json`, comme la clé publique
    // dans la configuration : c'est le texte minisign qui est encodé, pas les
    // octets bruts de la signature.
    let signature_text = String::from_utf8(
        base64_decode(platform["signature"].as_str().expect("signature"))
            .expect("la signature doit etre du base64 valide"),
    )
    .expect("signature lisible");
    let signature = Signature::decode(&signature_text).expect("signature minisign valide");
    pubkey
        .verify(&archive, &signature, false)
        .expect("l'archive publiee doit etre signee par la cle du projet");
    println!("signature verifiee avec la cle publique de tauri.conf.json");

    // Contre-epreuve : un test de signature qui accepte tout passerait aussi.
    // On abime un octet et on exige que la verification le voie, sans quoi
    // rien de ce qui precede ne prouve quoi que ce soit.
    let mut altered = archive.to_vec();
    altered[archive.len() / 2] ^= 0xff;
    assert!(
        pubkey.verify(&altered, &signature, false).is_err(),
        "une archive modifiee d'un seul octet doit etre refusee"
    );
    println!("archive modifiee : refusee, comme attendu");
}

/// Décodage base64 sans dépendance : la configuration n'en contient qu'une
/// valeur, et le reste du programme n'en a jamais eu besoin.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in input.bytes().filter(|b| !b.is_ascii_whitespace()) {
        if byte == b'=' {
            break;
        }
        let value = TABLE.iter().position(|c| *c == byte)? as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}
