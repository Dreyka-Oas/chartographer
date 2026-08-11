use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Token personnel Modrinth collé par l'utilisateur, validé puis conservé ici.
/// Ne repart jamais vers la webview : les commandes ne renvoient qu'un pseudo.
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
    fn require_token_explains_what_is_missing() {
        let dir = tmp("require");
        let message = require_token(&dir).unwrap_err().to_string();
        assert!(message.contains("connecte-toi"));
    }
}
