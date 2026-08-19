//! Jetons rangés là où le système les garde, pas dans un fichier.
//!
//! Un jeton Modrinth ouvre le compte de l'auteur, et celui de CurseForge y
//! publie des fichiers. Ecrits en clair dans le dossier de données, ils sont
//! lisibles par n'importe quel programme lancé sous la même session — les
//! droits du fichier n'y changent rien, puisque ce programme est cet
//! utilisateur. Le trousseau du système, lui, les chiffre avec les
//! identifiants du compte : le gestionnaire d'identifiants sous Windows, le
//! service de secrets sous Linux.
//!
//! Quand le trousseau ne répond pas, rien n'est écrit ailleurs en repli. Un
//! secours qui reposerait le fichier en clair rendrait tout ceci décoratif :
//! il suffirait d'empêcher le trousseau de répondre pour retrouver le jeton
//! sur le disque. L'application dit alors ce qui manque et redemande le jeton.

use crate::error::Result;

/// Nom sous lequel l'application se présente au trousseau. Le même que
/// l'identifiant de l'application : ce qui apparaît dans le gestionnaire
/// d'identifiants doit se laisser reconnaître.
pub const SERVICE: &str = "fr.dreykaoas.chartographer";

/// Jeton personnel Modrinth.
pub const MODRINTH_TOKEN: &str = "modrinth-token";
/// Jeton d'envoi CurseForge, relevé sur le tableau de bord de l'auteur.
pub const CURSEFORGE_UPLOAD_TOKEN: &str = "curseforge-upload-token";

/// Range un jeton. Ecrase celui qui s'y trouvait sous le même nom.
pub fn store(name: &str, value: &str) -> Result<()> {
    backend::store(name, value)
}

/// Relit un jeton. `None` quand il n'y en a pas — ce qui n'est pas une erreur :
/// c'est l'état d'une application qu'on n'a pas encore reliée.
pub fn load(name: &str) -> Result<Option<String>> {
    backend::load(name)
}

/// Oublie un jeton. Sans effet s'il n'y en avait pas : se déconnecter deux fois
/// de suite ne doit pas échouer la seconde.
pub fn clear(name: &str) -> Result<()> {
    backend::clear(name)
}

/// Vrai quand un jeton est rangé sous ce nom. Sert à annoncer une présence
/// sans sortir la valeur, que l'interface n'a jamais à connaître.
pub fn present(name: &str) -> bool {
    matches!(load(name), Ok(Some(_)))
}

/// Sous test, le trousseau de papier ne peut pas refuser : ce message n'a
/// alors personne pour l'employer.
#[cfg(not(test))]
fn unavailable(name: &str, detail: impl std::fmt::Display) -> crate::error::AppError {
    crate::error::AppError::Config(format!(
        "le trousseau du système n'a pas répondu pour « {name} » ({detail}). \
         Les jetons n'y sont pas conservés en clair ailleurs : il faudra les \
         redonner une fois le trousseau accessible."
    ))
}

/// Le trousseau du système, tel que l'application s'en sert vraiment.
#[cfg(not(test))]
mod backend {
    use super::{unavailable, SERVICE};
    use crate::error::Result;
    use keyring::Entry;

    fn entry(name: &str) -> Result<Entry> {
        Entry::new(SERVICE, name).map_err(|e| unavailable(name, e))
    }

    pub fn store(name: &str, value: &str) -> Result<()> {
        entry(name)?
            .set_password(value)
            .map_err(|e| unavailable(name, e))
    }

    pub fn load(name: &str) -> Result<Option<String>> {
        match entry(name)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(unavailable(name, e)),
        }
    }

    pub fn clear(name: &str) -> Result<()> {
        match entry(name)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(unavailable(name, e)),
        }
    }
}

/// Trousseau de papier, pour les tests unitaires seuls.
///
/// Deux raisons de ne pas éprouver le vrai trousseau ici. Il appartient à la
/// machine et non au dossier de données : des tests qui y écriraient sous le
/// nom de production effaceraient les jetons de l'application installée sur le
/// poste — c'est arrivé une fois, et un jeton perdu se recrée à la main. Et le
/// gestionnaire d'identifiants de Windows ne rend pas toujours une suppression
/// visible aussitôt : les tests devenaient instables sans rien dire du code.
///
/// Le vrai trousseau est éprouvé par `tests/keyring_roundtrip.rs`, qui sort de
/// ce fonctionnement de papier et se lance à la demande.
#[cfg(test)]
mod backend {
    use crate::error::Result;
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    fn shelf() -> &'static Mutex<HashMap<String, String>> {
        static SHELF: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
        SHELF.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn locked() -> std::sync::MutexGuard<'static, HashMap<String, String>> {
        shelf().lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn store(name: &str, value: &str) -> Result<()> {
        locked().insert(name.to_string(), value.to_string());
        Ok(())
    }

    pub fn load(name: &str) -> Result<Option<String>> {
        Ok(locked().get(name).cloned())
    }

    pub fn clear(name: &str) -> Result<()> {
        locked().remove(name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les entrées sont communes au module : deux tests qui se partageraient
    /// un nom se marcheraient dessus.
    fn scratch(suffix: &str) -> String {
        format!("test-{suffix}")
    }

    #[test]
    fn a_token_survives_a_round_trip() {
        let name = scratch("aller-retour");
        assert_eq!(load(&name).unwrap(), None, "rien avant d'avoir rangé");
        store(&name, "mrp_secret").unwrap();
        assert_eq!(load(&name).unwrap().as_deref(), Some("mrp_secret"));
        assert!(present(&name));
        clear(&name).unwrap();
        assert_eq!(load(&name).unwrap(), None, "plus rien après avoir oublié");
        assert!(!present(&name));
    }

    #[test]
    fn forgetting_twice_is_not_an_error() {
        let name = scratch("double-oubli");
        clear(&name).unwrap();
        clear(&name).unwrap();
    }

    #[test]
    fn storing_again_replaces_the_previous_one() {
        let name = scratch("remplacement");
        store(&name, "premier").unwrap();
        store(&name, "second").unwrap();
        assert_eq!(load(&name).unwrap().as_deref(), Some("second"));
        clear(&name).unwrap();
    }

    /// Deux noms ne se mélangent pas : c'est ce qui sépare le jeton Modrinth
    /// de celui de CurseForge.
    #[test]
    fn two_names_stay_apart() {
        store(&scratch("un"), "premier").unwrap();
        store(&scratch("deux"), "second").unwrap();
        assert_eq!(load(&scratch("un")).unwrap().as_deref(), Some("premier"));
        clear(&scratch("un")).unwrap();
        assert_eq!(load(&scratch("deux")).unwrap().as_deref(), Some("second"));
        clear(&scratch("deux")).unwrap();
    }
}
