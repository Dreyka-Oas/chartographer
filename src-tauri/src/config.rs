use crate::error::{AppError, Result};
use crate::secrets;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Token personnel Modrinth collé par l'utilisateur, validé puis conservé.
/// Ne repart jamais vers la webview : les commandes ne renvoient qu'un pseudo.
///
/// Le jeton ne s'écrit pas dans `session.json` : il va au trousseau du système,
/// et ce fichier ne garde que ce qui l'accompagne. Le champ se lit encore, en
/// revanche, c'est ainsi qu'une session écrite par une version antérieure se
/// laisse reprendre et déplacer, une fois, vers le trousseau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    #[serde(default, skip_serializing)]
    pub token: String,
    pub user_id: String,
    pub username: String,
    pub obtained_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub curseforge_username: Option<String>,
    #[serde(default = "default_range_days")]
    pub range_days: i64,
    /// Devise d'affichage, en code ISO à trois lettres. Les deux plateformes
    /// paient en dollars : tout autre choix passe par un taux de change relevé
    /// automatiquement.
    #[serde(default = "default_currency")]
    pub currency: String,
    /// Minutes entre deux relevés automatiques. Le plancher n'est pas
    /// cosmétique : CurseForge n'a pas d'interface publique et ne se lit qu'à
    /// travers une session de navigateur, qu'un martèlement régulier ferait
    /// remarquer. Voir `clamp_auto_sync`.
    #[serde(default = "default_auto_sync_minutes")]
    pub auto_sync_minutes: i64,
    /// Jeton d'envoi CurseForge, relevé sur le compte de l'auteur. Il ne repart
    /// jamais vers l'interface : les commandes ne disent que sa présence.
    ///
    /// Comme le jeton Modrinth, il vit dans le trousseau et non dans ce
    /// fichier. Le champ se lit encore pour reprendre, une fois, ce qu'une
    /// version antérieure y avait écrit.
    #[serde(default, skip_serializing)]
    pub curseforge_upload_token: Option<String>,
    /// Recherche d'une nouvelle version au démarrage. Vrai par défaut, y
    /// compris pour les fichiers écrits avant que ce réglage existe : une
    /// application qui ne se met jamais à jour d'elle-même reste sur un défaut
    /// corrigé ailleurs sans que personne ne le sache.
    #[serde(default = "default_auto_update")]
    pub auto_update: bool,
}

fn default_range_days() -> i64 {
    30
}

fn default_currency() -> String {
    "USD".into()
}

fn default_auto_sync_minutes() -> i64 {
    10
}

fn default_auto_update() -> bool {
    true
}

/// Cadence retenue pour les relevés automatiques.
///
/// Le plancher de dix minutes tient à CurseForge : ses chiffres ne se lisent
/// que dans une session de navigateur, et une horloge plus rapide se verrait.
/// Le plafond d'une journée évite qu'un réglage extrême ne fige les données.
pub fn clamp_auto_sync(minutes: i64) -> i64 {
    minutes.clamp(10, 1440)
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            curseforge_username: None,
            range_days: default_range_days(),
            currency: default_currency(),
            auto_sync_minutes: default_auto_sync_minutes(),
            curseforge_upload_token: None,
            auto_update: default_auto_update(),
        }
    }
}

pub fn db_path(app_data: &Path) -> PathBuf {
    app_data.join("chartographer.db")
}

fn session_path(app_data: &Path) -> PathBuf {
    app_data.join("session.json")
}

fn settings_path(app_data: &Path) -> PathBuf {
    app_data.join("settings.json")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Option<T> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_json<T: Serialize>(app_data: &Path, path: PathBuf, value: &T) -> Result<()> {
    std::fs::create_dir_all(app_data)
        .map_err(|e| AppError::Config(format!("dossier de configuration : {e}")))?;
    let raw = serde_json::to_string_pretty(value)?;
    std::fs::write(path, raw).map_err(|e| AppError::Config(format!("écriture : {e}")))
}

/// Relit la session, jeton compris, celui-ci venant du trousseau.
///
/// Un fichier écrit par une version antérieure porte encore le jeton en clair :
/// il est alors déplacé vers le trousseau et le fichier réécrit sans lui, une
/// bonne fois. La migration est silencieuse à dessein ; elle ne demande rien à
/// l'utilisateur et ne peut que retirer un secret du disque.
///
/// Sans jeton, trousseau muet, ou entrée effacée par ailleurs, il n'y a pas
/// de session : mieux vaut redemander le jeton qu'ouvrir l'application sur des
/// appels qui échoueront tous.
pub fn load_session(app_data: &Path) -> Option<Session> {
    let mut session: Session = read_json(session_path(app_data))?;
    if !session.token.is_empty() {
        // Fichier d'avant le trousseau : on l'y range, puis on le réécrit sans.
        if secrets::store(secrets::MODRINTH_TOKEN, &session.token).is_ok() {
            let _ = write_json(app_data, session_path(app_data), &session);
        }
        return Some(session);
    }
    session.token = secrets::load(secrets::MODRINTH_TOKEN).ok()??;
    Some(session)
}

pub fn save_session(app_data: &Path, session: &Session) -> Result<()> {
    // Le jeton d'abord : si le trousseau refuse, rien n'est écrit sur le
    // disque, et l'application dira que la connexion n'a pas pu être gardée.
    secrets::store(secrets::MODRINTH_TOKEN, &session.token)?;
    write_json(app_data, session_path(app_data), session)
}

pub fn clear_session(app_data: &Path) -> Result<()> {
    // Le jeton part même si le fichier a déjà disparu : c'est lui qui compte.
    secrets::clear(secrets::MODRINTH_TOKEN)?;
    match std::fs::remove_file(session_path(app_data)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::Config(format!("suppression de la session : {e}"))),
    }
}

pub fn require_token(app_data: &Path) -> Result<Session> {
    load_session(app_data)
        .ok_or_else(|| AppError::Config("aucune session Modrinth, connecte-toi d'abord".into()))
}

/// Relit les réglages, jeton d'envoi compris. Même migration que pour la
/// session : un jeton encore inscrit dans le fichier passe au trousseau, et le
/// fichier est réécrit sans lui.
pub fn load_settings(app_data: &Path) -> Settings {
    let mut settings: Settings = read_json(settings_path(app_data)).unwrap_or_default();
    match settings.curseforge_upload_token.take() {
        Some(token) if !token.is_empty() => {
            if secrets::store(secrets::CURSEFORGE_UPLOAD_TOKEN, &token).is_ok() {
                let _ = write_json(app_data, settings_path(app_data), &settings);
            }
            settings.curseforge_upload_token = Some(token);
        }
        _ => {
            settings.curseforge_upload_token = secrets::load(secrets::CURSEFORGE_UPLOAD_TOKEN)
                .ok()
                .flatten();
        }
    }
    settings
}

pub fn save_settings(app_data: &Path, settings: &Settings) -> Result<()> {
    match settings.curseforge_upload_token.as_deref() {
        Some(token) if !token.is_empty() => {
            secrets::store(secrets::CURSEFORGE_UPLOAD_TOKEN, token)?
        }
        // Réglages enregistrés sans jeton : celui du trousseau reste en place.
        // Seule une déconnexion l'efface, et elle passe par `clear_session`.
        _ => {}
    }
    write_json(app_data, settings_path(app_data), settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "chartographer-test-{}-{suffix}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// Les jetons passent par `secrets`, qui sous test range dans un trousseau
    /// de papier propre au processus : ces tests n'atteignent donc jamais le
    /// trousseau de la machine, et ne peuvent pas effacer les jetons d'une
    /// application installee sur le poste. Ils se partagent en revanche ce
    /// meme trousseau de papier, d'ou le verrou.
    ///
    /// Le verrou se reprend meme empoisonne : un test qui echoue le laisse
    /// dans cet etat, et les suivants n'ont pas a echouer pour cela.
    static KEYRING: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
        KEYRING.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn session_roundtrips_on_disk() {
        let _serial = one_at_a_time();
        let dir = tmp("session");
        let _ = secrets::clear(secrets::MODRINTH_TOKEN);
        assert!(load_session(&dir).is_none());
        let session = Session {
            token: "mrp_abc".into(),
            user_id: "VgD9obZq".into(),
            username: "DreykaOas".into(),
            obtained_at: "2026-08-11T10:00:00Z".into(),
        };
        save_session(&dir, &session).unwrap();
        assert_eq!(load_session(&dir).unwrap(), session);

        // Ce qui compte : le jeton n'est plus sur le disque.
        let raw = std::fs::read_to_string(session_path(&dir)).unwrap();
        assert!(
            !raw.contains("mrp_abc"),
            "le jeton ne doit pas figurer dans session.json : {raw}"
        );

        clear_session(&dir).unwrap();
        assert!(load_session(&dir).is_none());
    }

    #[test]
    fn settings_default_when_absent_then_persist() {
        let _serial = one_at_a_time();
        let dir = tmp("settings");
        let _ = secrets::clear(secrets::CURSEFORGE_UPLOAD_TOKEN);
        let defaults = load_settings(&dir);
        assert_eq!(defaults.range_days, 30);
        assert!(defaults.curseforge_username.is_none());

        assert_eq!(defaults.currency, "USD");
        assert_eq!(defaults.auto_sync_minutes, 10);
        assert!(defaults.auto_update);

        let updated = Settings {
            curseforge_username: Some("DreykaOas_official".into()),
            range_days: 180,
            currency: "EUR".into(),
            auto_sync_minutes: 30,
            curseforge_upload_token: Some("jeton".into()),
            auto_update: false,
        };
        save_settings(&dir, &updated).unwrap();
        assert_eq!(load_settings(&dir), updated);

        let raw = std::fs::read_to_string(settings_path(&dir)).unwrap();
        assert!(
            !raw.contains("jeton"),
            "le jeton d'envoi ne doit pas figurer dans settings.json : {raw}"
        );
        let _ = secrets::clear(secrets::CURSEFORGE_UPLOAD_TOKEN);
    }

    /// Un fichier ecrit avant le trousseau porte le jeton en clair. Le relire
    /// doit le deplacer et nettoyer le disque, sans rien demander.
    #[test]
    fn an_old_session_file_moves_its_token_to_the_keyring() {
        let _serial = one_at_a_time();
        let dir = tmp("migration");
        std::fs::create_dir_all(&dir).unwrap();
        let _ = secrets::clear(secrets::MODRINTH_TOKEN);
        std::fs::write(
            session_path(&dir),
            r#"{"token":"mrp_ancien","user_id":"VgD9obZq","username":"DreykaOas","obtained_at":"2026-08-11T10:00:00Z"}"#,
        )
        .unwrap();

        let session = load_session(&dir).expect("la session ancienne se reprend");
        assert_eq!(session.token, "mrp_ancien");
        assert_eq!(
            secrets::load(secrets::MODRINTH_TOKEN).unwrap().as_deref(),
            Some("mrp_ancien"),
            "le jeton doit avoir rejoint le trousseau"
        );
        let raw = std::fs::read_to_string(session_path(&dir)).unwrap();
        assert!(
            !raw.contains("mrp_ancien"),
            "le fichier doit avoir ete reecrit sans le jeton : {raw}"
        );

        // Et il se relit encore, depuis le trousseau cette fois.
        assert_eq!(load_session(&dir).unwrap().token, "mrp_ancien");
        clear_session(&dir).unwrap();
    }

    #[test]
    fn auto_sync_never_goes_below_ten_minutes() {
        assert_eq!(clamp_auto_sync(1), 10);
        assert_eq!(clamp_auto_sync(0), 10);
        assert_eq!(clamp_auto_sync(-5), 10);
        assert_eq!(clamp_auto_sync(45), 45);
        assert_eq!(clamp_auto_sync(99_999), 1440);
    }

    #[test]
    fn an_old_settings_file_keeps_working() {
        let _serial = one_at_a_time();
        // Fichier écrit par une version antérieure, sans devise ni jeton.
        let dir = tmp("legacy");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            settings_path(&dir),
            r#"{"curseforge_username":"DreykaOas_official","range_days":30}"#,
        )
        .unwrap();

        // Le trousseau n'est pas propre a ce dossier : un jeton qui y traine
        // serait rendu a ce faux reglage, et le test verifie justement qu'il
        // n'y en a pas.
        let _ = secrets::clear(secrets::CURSEFORGE_UPLOAD_TOKEN);

        let loaded = load_settings(&dir);

        assert_eq!(loaded.range_days, 30);
        assert_eq!(loaded.currency, "USD");
        assert_eq!(loaded.auto_sync_minutes, 10);
        assert!(loaded.curseforge_upload_token.is_none());
        // Le réglage n'existait pas : l'application se met quand même à jour.
        assert!(loaded.auto_update);
    }

    #[test]
    fn require_token_explains_what_is_missing() {
        let dir = tmp("require");
        let message = require_token(&dir).unwrap_err().to_string();
        assert!(message.contains("connecte-toi"));
    }
}
