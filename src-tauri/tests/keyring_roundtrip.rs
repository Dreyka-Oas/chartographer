//! Le vrai trousseau du système, éprouvé pour lui-même.
//!
//! Ignoré par défaut : il écrit dans le trousseau de la machine.
//! `cargo test --test keyring_roundtrip -- --ignored --nocapture`.
//!
//! Les tests unitaires de `secrets` passent par un trousseau de papier — ils
//! éprouvent les règles, pas le système. Celui-ci fait l'inverse : il ne dit
//! rien des règles, et répond à la seule question que le papier ne peut pas
//! trancher, à savoir si la machine garde et rend vraiment ce qu'on lui confie.
//!
//! Il n'emploie jamais les noms de l'application. Un test qui écrirait sous
//! `modrinth-token` effacerait le jeton de l'installation présente sur le
//! poste — ce n'est pas une hypothèse, c'est arrivé.

use keyring::Entry;

/// Service distinct de celui de l'application, pour la même raison.
const SERVICE: &str = "fr.dreykaoas.chartographer.essai-trousseau";

fn entry(name: &str) -> Entry {
    Entry::new(SERVICE, name).expect("le trousseau doit s'ouvrir")
}

#[test]
#[ignore = "ecrit dans le trousseau de la machine"]
fn the_system_keeps_and_returns_what_it_is_given() {
    let name = format!("essai-{}", std::process::id());
    let slot = entry(&name);

    // Table rase : une exécution précédente a pu laisser quelque chose.
    let _ = slot.delete_credential();

    slot.set_password("mrp_valeur_d_essai")
        .expect("le trousseau doit accepter un secret");
    let relu = entry(&name)
        .get_password()
        .expect("le trousseau doit rendre ce qu'il a pris");
    assert_eq!(relu, "mrp_valeur_d_essai");
    println!("le trousseau garde et rend : bon");

    entry(&name)
        .delete_credential()
        .expect("le trousseau doit oublier sur demande");
    match entry(&name).get_password() {
        Err(keyring::Error::NoEntry) => println!("le trousseau oublie : bon"),
        Ok(_) => panic!("le secret est encore là après avoir été oublié"),
        Err(e) => panic!("réponse inattendue après suppression : {e}"),
    }
}
