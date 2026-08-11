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

/// Tableau de bord auteur, ouvert dans le navigateur de l'utilisateur.
///
/// Une fenêtre intégrée restait blanche : le site refuse de s'afficher hors d'un
/// navigateur complet, et sa page de connexion passe par un tiers. On l'ouvre
/// donc là où l'utilisateur est déjà connecté, et il rapporte ce qu'il y voit.
pub const CF_AUTHOR_PAGE: &str = "https://authors.curseforge.com/";

pub const CF_WINDOW: &str = "curseforge";

#[tauri::command]
pub fn open_curseforge_site(app: tauri::AppHandle) -> Result<()> {
    app.opener()
        .open_url(CF_AUTHOR_PAGE, None::<&str>)
        .map_err(|e| AppError::Config(format!("ouverture du navigateur : {e}")))
}

/// Ouvre le tableau de bord dans une fenêtre de l'application.
///
/// Rien n'est injecté au chargement : le site est une application web protégée
/// par un filtre anti-robot, et modifier `fetch` avant son démarrage l'empêchait
/// de s'afficher. L'écoute des requêtes est posée après coup, à la demande.
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
        .title("CurseForge — connexion, solde et statistiques")
        .inner_size(1180.0, 860.0)
        .build()
        .map_err(|e| AppError::Config(format!("ouverture de la fenêtre CurseForge : {e}")))?;
    Ok(())
}

/// Écoute posée dans la page déjà chargée : elle conserve les réponses que la
/// page recevra ensuite, en naviguant vers les statistiques. Rien n'est demandé
/// au serveur qui ne l'aurait été de toute façon.
const CAPTURE_SCRIPT: &str = r#"(function () {
  if (window.__chartographer) { return 'deja en place'; }
  var store = { captures: [] };
  window.__chartographer = store;

  function keep(url, body) {
    if (typeof body !== 'string') return;
    var trimmed = body.trim();
    if (trimmed.charAt(0) !== '{' && trimmed.charAt(0) !== '[') return;
    store.captures.unshift({ url: String(url), body: trimmed.slice(0, 400000) });
    if (store.captures.length > 40) store.captures.length = 40;
  }

  try {
    var nativeFetch = window.fetch;
    if (nativeFetch) {
      window.fetch = function () {
        var first = arguments[0];
        var url = typeof first === 'string' ? first : (first && first.url) || '';
        return nativeFetch.apply(this, arguments).then(function (response) {
          try { response.clone().text().then(function (b) { keep(url, b); }); } catch (e) {}
          return response;
        });
      };
    }
    var open = XMLHttpRequest.prototype.open;
    var send = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function (method, url) {
      this.__cgUrl = url;
      return open.apply(this, arguments);
    };
    XMLHttpRequest.prototype.send = function () {
      var xhr = this;
      xhr.addEventListener('load', function () {
        try { keep(xhr.__cgUrl || '', xhr.responseText); } catch (e) {}
      });
      return send.apply(this, arguments);
    };
    return 'ecoute posee';
  } catch (e) {
    return 'echec : ' + e;
  }
})()"#;

/// Relève le texte affiché et les réponses interceptées depuis l'armement.
const READ_SCRIPT: &str = r#"(function () {
  var store = window.__chartographer || { captures: [] };
  return {
    url: location.href,
    title: document.title,
    text: (document.body ? document.body.innerText : '').slice(0, 20000),
    captures: store.captures
  };
})()"#;

/// Exécute un script dans la fenêtre CurseForge et rend son résultat.
async fn eval_in_window(app: &tauri::AppHandle, script: &str) -> Result<String> {
    let window = app.get_webview_window(CF_WINDOW).ok_or_else(|| {
        AppError::Config("ouvre d'abord la fenêtre CurseForge et connecte-toi".into())
    })?;

    let (sender, receiver) = tokio::sync::oneshot::channel::<String>();
    let slot = std::sync::Mutex::new(Some(sender));
    window
        .eval_with_callback(script, move |raw| {
            if let Ok(mut guard) = slot.lock() {
                if let Some(sender) = guard.take() {
                    let _ = sender.send(raw);
                }
            }
        })
        .map_err(|e| AppError::Config(format!("exécution dans la page : {e}")))?;

    tokio::time::timeout(std::time::Duration::from_secs(8), receiver)
        .await
        .map_err(|_| AppError::Remote {
            provider: "CurseForge".into(),
            detail: "la page n'a pas répondu en huit secondes".into(),
        })?
        .map_err(|_| AppError::Remote {
            provider: "CurseForge".into(),
            detail: "lecture de la page interrompue".into(),
        })
}

#[tauri::command]
pub async fn arm_curseforge_capture(app: tauri::AppHandle) -> Result<String> {
    let raw = eval_in_window(&app, CAPTURE_SCRIPT).await?;
    Ok(raw.trim_matches('"').to_string())
}

/// Une réponse interceptée, résumée pour l'interface.
#[derive(Debug, Serialize)]
pub struct CfCapture {
    pub url: String,
    pub days: usize,
    pub from: Option<String>,
    pub to: Option<String>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct CfScrape {
    pub url: String,
    pub title: String,
    pub points: Option<i64>,
    pub excerpt: String,
    pub captures: Vec<CfCapture>,
}

#[tauri::command]
pub async fn read_curseforge_page(app: tauri::AppHandle) -> Result<CfScrape> {
    let raw = eval_in_window(&app, READ_SCRIPT).await?;

    #[derive(serde::Deserialize)]
    struct RawCapture {
        url: String,
        body: String,
    }
    #[derive(serde::Deserialize)]
    struct Snapshot {
        url: String,
        title: String,
        text: String,
        #[serde(default)]
        captures: Vec<RawCapture>,
    }
    let snapshot: Snapshot = serde_json::from_str(&raw)
        .map_err(|e| AppError::Data(format!("réponse de la page illisible : {e}")))?;

    let mut captures: Vec<CfCapture> = snapshot
        .captures
        .iter()
        .filter_map(|capture| {
            let parsed: serde_json::Value = serde_json::from_str(&capture.body).ok()?;
            let series = crate::scrape::find_daily_series(&parsed);
            if series.is_empty() {
                return None;
            }
            Some(CfCapture {
                url: capture.url.clone(),
                days: series.len(),
                from: series.first().map(|p| p.day.clone()),
                to: series.last().map(|p| p.day.clone()),
                total: series.iter().map(|p| p.value).sum(),
            })
        })
        .collect();
    captures.sort_by_key(|c| std::cmp::Reverse(c.days));

    Ok(CfScrape {
        points: crate::scrape::extract_points(&snapshot.text),
        excerpt: crate::scrape::excerpt_around(&snapshot.text, "point", 90),
        url: snapshot.url,
        title: snapshot.title,
        captures,
    })
}

/// Enregistre le solde lu dans la page, après validation de l'utilisateur.
#[tauri::command]
pub async fn import_curseforge_capture(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    curseforge_id: i64,
    url: String,
) -> Result<usize> {
    let raw = eval_in_window(&app, READ_SCRIPT).await?;

    #[derive(serde::Deserialize)]
    struct RawCapture {
        url: String,
        body: String,
    }
    #[derive(serde::Deserialize)]
    struct Snapshot {
        #[serde(default)]
        captures: Vec<RawCapture>,
    }
    let snapshot: Snapshot = serde_json::from_str(&raw)
        .map_err(|e| AppError::Data(format!("réponse de la page illisible : {e}")))?;

    let capture = snapshot
        .captures
        .iter()
        .find(|c| c.url == url)
        .ok_or_else(|| AppError::Data("cette source n'est plus en mémoire".into()))?;
    let parsed: serde_json::Value = serde_json::from_str(&capture.body)
        .map_err(|e| AppError::Data(format!("contenu illisible : {e}")))?;
    let series = crate::scrape::find_daily_series(&parsed);
    if series.is_empty() {
        return Err(AppError::Data("aucune série datée dans cette source".into()));
    }

    state.store.with(|conn| {
        for point in &series {
            crate::store::metrics::upsert_daily(
                conn,
                curseforge_id,
                &point.day,
                Some(point.value),
                None,
                None,
            )?;
        }
        Ok(series.len())
    })
}

/// Ce que l'application a reconnu dans ce que l'utilisateur a rapporté de son
/// tableau de bord : rien n'est enregistré sans sa validation.
#[derive(Debug, Serialize)]
pub struct CfAnalysis {
    /// Solde de points repéré dans du texte copié.
    pub points: Option<i64>,
    /// Nombre de jours de la série trouvée, s'il y en a une.
    pub days: usize,
    pub from: Option<String>,
    pub to: Option<String>,
    pub total: i64,
    /// Extrait de contrôle, pour que l'utilisateur voie ce qui a été lu.
    pub excerpt: String,
}

/// Analyse un contenu rapporté du tableau de bord CurseForge : soit le texte de
/// la page, soit la réponse d'une de ses requêtes internes.
///
/// Le site refusant de s'afficher dans une fenêtre intégrée, c'est l'utilisateur
/// qui apporte la donnée depuis son navigateur. L'analyse reste ici, testable.
#[tauri::command]
pub fn analyze_curseforge_text(text: String) -> CfAnalysis {
    let series = serde_json::from_str::<serde_json::Value>(text.trim())
        .map(|parsed| crate::scrape::find_daily_series(&parsed))
        .unwrap_or_default();

    CfAnalysis {
        points: crate::scrape::extract_points(&text),
        days: series.len(),
        from: series.first().map(|p| p.day.clone()),
        to: series.last().map(|p| p.day.clone()),
        total: series.iter().map(|p| p.value).sum(),
        excerpt: crate::scrape::excerpt_around(&text, "point", 90),
    }
}

/// Enregistre une série datée rapportée du tableau de bord, sur un projet
/// CurseForge donné. Les journées déjà connues sont remplacées.
#[tauri::command]
pub fn import_curseforge_series(
    state: State<'_, AppState>,
    curseforge_id: i64,
    text: String,
) -> Result<usize> {
    let parsed: serde_json::Value = serde_json::from_str(text.trim())
        .map_err(|e| AppError::Data(format!("ce contenu n'est pas du JSON : {e}")))?;
    let series = crate::scrape::find_daily_series(&parsed);
    if series.is_empty() {
        return Err(AppError::Data(
            "aucune série datée reconnue dans ce contenu".into(),
        ));
    }

    state.store.with(|conn| {
        for point in &series {
            crate::store::metrics::upsert_daily(
                conn,
                curseforge_id,
                &point.day,
                Some(point.value),
                None,
                None,
            )?;
        }
        Ok(series.len())
    })
}

pub fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
}
