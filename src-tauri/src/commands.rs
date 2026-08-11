use crate::config::{self, Session, Settings};
use crate::error::{AppError, Result};
use crate::models::{Overview, Platform};
use crate::providers::modrinth::ModrinthClient;
use crate::store::{projects as p, queries, Store};
use crate::sync::{self, SyncContext, SyncReport};
use chrono::Utc;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{Manager, State};
use tauri_plugin_opener::OpenerExt;

/// Page où l'utilisateur génère son token personnel. Ouverte pour lui depuis
/// l'écran de connexion, pour qu'il n'ait pas à la chercher.
pub const PAT_PAGE: &str = "https://modrinth.com/settings/pats";

pub struct AppState {
    pub store: Store,
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn context(&self) -> Result<SyncContext> {
        Ok(SyncContext {
            session: config::require_token(&self.data_dir)?,
            settings: config::load_settings(&self.data_dir),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct AuthStatus {
    pub connected: bool,
    pub username: Option<String>,
    pub connected_since: Option<String>,
}

fn status_of(state: &AppState) -> AuthStatus {
    let session = config::load_session(&state.data_dir);
    AuthStatus {
        connected: session.is_some(),
        username: session.as_ref().map(|s| s.username.clone()),
        connected_since: session.as_ref().map(|s| s.obtained_at.clone()),
    }
}

#[tauri::command]
pub fn auth_status(state: State<'_, AppState>) -> AuthStatus {
    status_of(&state)
}

/// Valide le token auprès de Modrinth avant de l'écrire sur le disque :
/// une saisie erronée est rejetée tout de suite, avec le message de l'API.
#[tauri::command]
pub async fn connect(state: State<'_, AppState>, token: String) -> Result<AuthStatus> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err(AppError::Config("colle ton token Modrinth".into()));
    }

    let user = ModrinthClient::new(&token)?.me().await?;
    config::save_session(
        &state.data_dir,
        &Session {
            token,
            user_id: user.id,
            username: user.username,
            obtained_at: Utc::now().to_rfc3339(),
        },
    )?;
    Ok(status_of(&state))
}

#[tauri::command]
pub fn logout(state: State<'_, AppState>) -> Result<AuthStatus> {
    config::clear_session(&state.data_dir)?;
    Ok(status_of(&state))
}

#[tauri::command]
pub fn open_token_page(app: tauri::AppHandle) -> Result<()> {
    app.opener()
        .open_url(PAT_PAGE, None::<&str>)
        .map_err(|e| AppError::Config(format!("ouverture du navigateur : {e}")))
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    config::load_settings(&state.data_dir)
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    curseforge_username: Option<String>,
    range_days: i64,
) -> Result<()> {
    let settings = Settings {
        curseforge_username: curseforge_username
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),
        range_days: range_days.clamp(7, 730),
    };
    config::save_settings(&state.data_dir, &settings)
}

#[tauri::command]
pub async fn sync_now(state: State<'_, AppState>) -> Result<Vec<SyncReport>> {
    let ctx = state.context()?;
    Ok(sync::full(&state.store, &ctx).await)
}

#[tauri::command]
pub fn overview(state: State<'_, AppState>, range_days: i64) -> Result<Overview> {
    let today = sync::today_utc();
    let range = range_days.clamp(7, 730);
    state
        .store
        .with(|conn| queries::overview(conn, &today, range))
}

#[tauri::command]
pub fn link_manual(state: State<'_, AppState>, modrinth_id: i64, curseforge_id: i64) -> Result<()> {
    state
        .store
        .with(|conn| p::upsert_link(conn, modrinth_id, curseforge_id, 1.0, true))
}

#[tauri::command]
pub fn unlink(state: State<'_, AppState>, modrinth_id: i64, curseforge_id: i64) -> Result<()> {
    state
        .store
        .with(|conn| p::delete_link(conn, modrinth_id, curseforge_id).map(|_| ()))
}

#[tauri::command]
pub fn unlinked_projects(state: State<'_, AppState>) -> Result<Vec<(i64, String, String)>> {
    state.store.with(|conn| {
        let link_rows = p::links(conn)?;
        Ok(p::list(conn)?
            .into_iter()
            .filter(|project| match project.platform {
                Platform::Modrinth => !link_rows
                    .iter()
                    .any(|l| l.modrinth_project_id == project.id),
                Platform::CurseForge => !link_rows.iter().any(|l| l.cf_project_id == project.id),
            })
            .map(|project| {
                (
                    project.id,
                    project.platform.as_str().to_string(),
                    project.title,
                )
            })
            .collect())
    })
}

pub fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
}
