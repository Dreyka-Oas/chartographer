use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Token Modrinth obtenu par OAuth. Ne franchit jamais la frontière vers la webview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
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
}

fn default_range_days() -> i64 {
    90
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            curseforge_username: None,
            range_days: default_range_days(),
        }
    }
}

/// Identifiants de l'application OAuth enregistrée sur modrinth.com/settings/applications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OauthApp {
    pub client_id: String,
    pub client_secret: String,
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

fn oauth_path(app_data: &Path) -> PathBuf {
    app_data.join("oauth.json")
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

pub fn load_session(app_data: &Path) -> Option<Session> {
    read_json(session_path(app_data))
}

pub fn save_session(app_data: &Path, session: &Session) -> Result<()> {
    write_json(app_data, session_path(app_data), session)
}

pub fn clear_session(app_data: &Path) -> Result<()> {
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

pub fn load_settings(app_data: &Path) -> Settings {
    read_json(settings_path(app_data)).unwrap_or_default()
}

pub fn save_settings(app_data: &Path, settings: &Settings) -> Result<()> {
    write_json(app_data, settings_path(app_data), settings)
}

/// Les valeurs injectées à la compilation priment sur le fichier.
/// Un couple incomplet est ignoré : mieux vaut aucune application qu'une application cassée.
pub fn load_oauth_app(
    app_data: &Path,
    compiled_id: Option<&str>,
    compiled_secret: Option<&str>,
) -> Option<OauthApp> {
    if let (Some(id), Some(secret)) = (compiled_id, compiled_secret) {
        if !id.is_empty() && !secret.is_empty() {
            return Some(OauthApp {
                client_id: id.into(),
                client_secret: secret.into(),
            });
        }
    }
    read_json(oauth_path(app_data))
}

pub fn save_oauth_app(app_data: &Path, app: &OauthApp) -> Result<()> {
    write_json(app_data, oauth_path(app_data), app)
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

    #[test]
    fn session_roundtrips_on_disk() {
        let dir = tmp("session");
        assert!(load_session(&dir).is_none());
        let session = Session {
            token: "mrp_abc".into(),
            user_id: "VgD9obZq".into(),
            username: "DreykaOas".into(),
            obtained_at: "2026-08-11T10:00:00Z".into(),
        };
        save_session(&dir, &session).unwrap();
        assert_eq!(load_session(&dir).unwrap(), session);
        clear_session(&dir).unwrap();
        assert!(load_session(&dir).is_none());
    }

    #[test]
    fn settings_default_when_absent_then_persist() {
        let dir = tmp("settings");
        let defaults = load_settings(&dir);
        assert_eq!(defaults.range_days, 90);
        assert!(defaults.curseforge_username.is_none());

        let updated = Settings {
            curseforge_username: Some("DreykaOas_official".into()),
            range_days: 180,
        };
        save_settings(&dir, &updated).unwrap();
        assert_eq!(load_settings(&dir), updated);
    }

    #[test]
    fn oauth_app_prefers_compiled_values_then_disk() {
        let dir = tmp("oauth");
        assert!(load_oauth_app(&dir, None, None).is_none());

        save_oauth_app(
            &dir,
            &OauthApp {
                client_id: "disk".into(),
                client_secret: "s1".into(),
            },
        )
        .unwrap();
        assert_eq!(load_oauth_app(&dir, None, None).unwrap().client_id, "disk");

        let compiled = load_oauth_app(&dir, Some("compiled"), Some("s2")).unwrap();
        assert_eq!(compiled.client_id, "compiled");
        assert_eq!(compiled.client_secret, "s2");
    }

    #[test]
    fn oauth_app_ignores_half_filled_compiled_values() {
        let dir = tmp("oauth-partial");
        assert!(load_oauth_app(&dir, Some("only-id"), None).is_none());
        assert!(load_oauth_app(&dir, None, Some("only-secret")).is_none());
    }
}
