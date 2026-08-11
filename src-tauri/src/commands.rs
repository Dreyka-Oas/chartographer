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
    /// Auteur CurseForge retenu, réglé à la main ou détecté au dernier cycle.
    pub curseforge_username: Option<String>,
    /// Nombre de projets relevés par plateforme, pour l'état de connexion.
    pub modrinth_projects: i64,
    pub curseforge_projects: i64,
}

fn status_of(state: &AppState) -> AuthStatus {
    let session = config::load_session(&state.data_dir);
    let settings = config::load_settings(&state.data_dir);
    // L'état des plateformes se lit dans la base : elle survit au redémarrage,
    // contrairement à ce que le front garde en mémoire.
    let (author, modrinth, curseforge) = state
        .store
        .with(|conn| {
            let count = |platform: Platform| -> Result<i64> {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM projects WHERE platform = ?1",
                    [platform.as_str()],
                    |r| r.get(0),
                )?)
            };
            Ok((
                crate::store::metrics::get_meta(conn, "curseforge_author")?,
                count(Platform::Modrinth)?,
                count(Platform::CurseForge)?,
            ))
        })
        .unwrap_or((None, 0, 0));

    AuthStatus {
        connected: session.is_some(),
        username: session.as_ref().map(|s| s.username.clone()),
        connected_since: session.as_ref().map(|s| s.obtained_at.clone()),
        curseforge_username: settings.curseforge_username.or(author),
        modrinth_projects: modrinth,
        curseforge_projects: curseforge,
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

/// `from` et `to` sont des dates incluses `YYYY-MM-DD`. Absentes, on retombe sur
/// la fenêtre glissante de `range_days` jours qui se termine aujourd'hui.
#[tauri::command]
pub fn overview(
    state: State<'_, AppState>,
    range_days: i64,
    from: Option<String>,
    to: Option<String>,
    platforms: Option<Vec<String>>,
) -> Result<Overview> {
    let today = sync::today_utc();
    let range = range_days.clamp(7, 730);
    let (from, to) = queries::resolve_range(&today, range, from.as_deref(), to.as_deref());
    let filter = queries::PlatformFilter::from_names(platforms.as_deref());
    state
        .store
        .with(|conn| queries::overview(conn, &today, &from, &to, filter))
}

#[tauri::command]
pub fn project_detail(
    state: State<'_, AppState>,
    modrinth_id: Option<i64>,
    curseforge_id: Option<i64>,
    range_days: i64,
    from: Option<String>,
    to: Option<String>,
) -> Result<crate::models::ProjectDetail> {
    let today = sync::today_utc();
    let range = range_days.clamp(7, 730);
    let (from, to) = queries::resolve_range(&today, range, from.as_deref(), to.as_deref());
    state
        .store
        .with(|conn| queries::project_detail(conn, &from, &to, modrinth_id, curseforge_id))
}

/// Un projet vu par l'écran d'appariement : son état vis-à-vis de l'autre
/// plateforme, qu'il soit apparié, en attente, ou déclaré sans équivalent.
#[derive(Debug, Serialize)]
pub struct PairingEntry {
    pub id: i64,
    pub platform: String,
    pub title: String,
    /// Identifiant et titre du jumeau, si le projet est apparié.
    pub linked_id: Option<i64>,
    pub linked_to: Option<String>,
    /// Vrai si l'appariement a été posé à la main.
    pub manual: bool,
    /// Vrai si le projet a été déclaré sans équivalent sur l'autre plateforme.
    pub solo: bool,
}

/// Tous les projets des deux plateformes avec leur état d'appariement.
///
/// L'écran a besoin de la liste complète, pas seulement des orphelins : sans
/// elle, impossible de corriger un rapprochement erroné ni de rattacher un mod
/// à un jumeau déjà pris.
#[tauri::command]
pub fn pairing_state(state: State<'_, AppState>) -> Result<Vec<PairingEntry>> {
    state.store.with(|conn| {
        let link_rows = p::links(conn)?;
        let solo = p::solo_ids(conn)?;
        let projects = p::list(conn)?;
        let title_of = |id: i64| projects.iter().find(|p| p.id == id).map(|p| p.title.clone());

        Ok(projects
            .iter()
            .map(|project| {
                let link = link_rows.iter().find(|l| match project.platform {
                    Platform::Modrinth => l.modrinth_project_id == project.id,
                    Platform::CurseForge => l.cf_project_id == project.id,
                });
                let linked_id = link.map(|l| match project.platform {
                    Platform::Modrinth => l.cf_project_id,
                    Platform::CurseForge => l.modrinth_project_id,
                });
                PairingEntry {
                    id: project.id,
                    platform: project.platform.as_str().to_string(),
                    title: project.title.clone(),
                    linked_to: linked_id.and_then(title_of),
                    linked_id,
                    manual: link.map(|l| l.manual).unwrap_or(false),
                    solo: solo.contains(&project.id),
                }
            })
            .collect())
    })
}

#[tauri::command]
pub fn link_manual(state: State<'_, AppState>, modrinth_id: i64, curseforge_id: i64) -> Result<()> {
    state.store.with(|conn| {
        // Un projet rattaché à la main n'est plus « sans équivalent ».
        p::set_solo(conn, modrinth_id, false)?;
        p::set_solo(conn, curseforge_id, false)?;
        p::link_exclusive(conn, modrinth_id, curseforge_id)
    })
}

#[tauri::command]
pub fn unlink(state: State<'_, AppState>, modrinth_id: i64, curseforge_id: i64) -> Result<()> {
    state
        .store
        .with(|conn| p::delete_link(conn, modrinth_id, curseforge_id).map(|_| ()))
}

#[tauri::command]
pub fn set_solo(state: State<'_, AppState>, project_id: i64, solo: bool) -> Result<()> {
    state.store.with(|conn| p::set_solo(conn, project_id, solo))
}

/// Enregistre le solde de points CurseForge relevé à la main.
///
/// Leur programme de rémunération n'expose aucune interface : ni l'API publique,
/// ni le jeton de dépôt ne donnent accès au solde. Il ne se lit que sur le
/// tableau de bord auteur, d'où cette saisie.
#[tauri::command]
pub fn record_curseforge_points(state: State<'_, AppState>, points: i64) -> Result<()> {
    let today = sync::today_utc();
    let now = Utc::now().to_rfc3339();
    state
        .store
        .with(|conn| crate::store::metrics::record_cf_points(conn, &today, points, &now))
}

#[tauri::command]
pub fn forget_curseforge_points(state: State<'_, AppState>, day: String) -> Result<()> {
    state
        .store
        .with(|conn| crate::store::metrics::delete_cf_points(conn, &day).map(|_| ()))
}

#[tauri::command]
pub fn curseforge_points(
    state: State<'_, AppState>,
) -> Result<Vec<crate::store::metrics::CfPointEntry>> {
    state.store.with(crate::store::metrics::cf_points)
}

/// Fenêtre où l'utilisateur se connecte à CurseForge. Ses identifiants sont
/// saisis dans la page officielle, jamais dans notre interface.
pub const CF_WINDOW: &str = "curseforge";
pub const CF_AUTHOR_PAGE: &str = "https://authors.curseforge.com/";

#[tauri::command]
pub fn open_curseforge_window(app: tauri::AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(CF_WINDOW) {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    let url = tauri::Url::parse(CF_AUTHOR_PAGE)
        .map_err(|e| AppError::Config(format!("adresse CurseForge invalide : {e}")))?;
    tauri::WebviewWindowBuilder::new(&app, CF_WINDOW, tauri::WebviewUrl::External(url))
        .title("CurseForge — connexion et solde de points")
        .inner_size(1100.0, 820.0)
        .build()
        .map_err(|e| AppError::Config(format!("ouverture de la fenêtre CurseForge : {e}")))?;
    Ok(())
}

/// Ce que la lecture de la page a retenu, soumis à l'utilisateur avant
/// enregistrement : la valeur n'est jamais écrite dans son dos.
#[derive(Debug, Serialize)]
pub struct CfScrape {
    pub url: String,
    pub title: String,
    pub points: Option<i64>,
    pub excerpt: String,
}

/// Relève le texte affiché dans la fenêtre CurseForge. On ne lit que ce que
/// l'utilisateur voit déjà à l'écran, sur son propre compte.
const READ_SCRIPT: &str = r#"(function () {
  return {
    url: location.href,
    title: document.title,
    text: (document.body ? document.body.innerText : '').slice(0, 20000)
  };
})()"#;

#[tauri::command]
pub async fn read_curseforge_page(app: tauri::AppHandle) -> Result<CfScrape> {
    let window = app.get_webview_window(CF_WINDOW).ok_or_else(|| {
        AppError::Config("ouvre d'abord la fenêtre CurseForge et connecte-toi".into())
    })?;

    let (sender, receiver) = tokio::sync::oneshot::channel::<String>();
    let slot = std::sync::Mutex::new(Some(sender));
    window
        .eval_with_callback(READ_SCRIPT, move |raw| {
            if let Ok(mut guard) = slot.lock() {
                if let Some(sender) = guard.take() {
                    let _ = sender.send(raw);
                }
            }
        })
        .map_err(|e| AppError::Config(format!("lecture de la page : {e}")))?;

    let raw = tokio::time::timeout(std::time::Duration::from_secs(8), receiver)
        .await
        .map_err(|_| AppError::Remote {
            provider: "CurseForge".into(),
            detail: "la page n'a pas répondu en huit secondes".into(),
        })?
        .map_err(|_| AppError::Remote {
            provider: "CurseForge".into(),
            detail: "lecture de la page interrompue".into(),
        })?;

    #[derive(serde::Deserialize)]
    struct Snapshot {
        url: String,
        title: String,
        text: String,
    }
    let snapshot: Snapshot = serde_json::from_str(&raw)
        .map_err(|e| AppError::Data(format!("réponse de la page illisible : {e}")))?;

    Ok(CfScrape {
        points: crate::scrape::extract_points(&snapshot.text),
        excerpt: crate::scrape::excerpt_around(&snapshot.text, "point", 90),
        url: snapshot.url,
        title: snapshot.title,
    })
}

pub fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
}
