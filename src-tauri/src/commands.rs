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
    let (author, account, modrinth, curseforge) = state
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
                crate::store::metrics::get_meta(conn, "curseforge_account")?,
                count(Platform::Modrinth)?,
                count(Platform::CurseForge)?,
            ))
        })
        .unwrap_or((None, None, 0, 0));

    AuthStatus {
        connected: session.is_some(),
        username: session.as_ref().map(|s| s.username.clone()),
        connected_since: session.as_ref().map(|s| s.obtained_at.clone()),
        // Le compte relevé sur le tableau de bord fait foi : il nomme la session
        // réellement ouverte, là où l'auteur déduit des mods peut être un autre.
        curseforge_username: settings.curseforge_username.or(account).or(author),
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

/// Adresse publique d'un compte, plateforme par plateforme.
///
/// L'adresse est bâtie ici et non côté fenêtre : le nom relevé est la seule
/// chose qui vienne de l'extérieur, et il ne sert qu'à compléter un gabarit
/// connu. Aucune adresse arbitraire ne peut donc être ouverte.
pub(crate) fn account_page(platform: &str, username: &str) -> Option<String> {
    let name = username.trim();
    if name.is_empty() {
        return None;
    }
    let encoded: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        .collect();
    if encoded.is_empty() {
        return None;
    }
    match platform {
        "modrinth" => Some(format!("https://modrinth.com/user/{encoded}")),
        "curseforge" => Some(format!(
            "https://www.curseforge.com/members/{encoded}/projects"
        )),
        _ => None,
    }
}

#[tauri::command]
pub fn open_account_page(app: tauri::AppHandle, platform: String, username: String) -> Result<()> {
    let url = account_page(&platform, &username)
        .ok_or_else(|| AppError::Config("aucune page de compte pour ce pseudo".into()))?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::Config(format!("ouverture du navigateur : {e}")))
}

/// Réglages tels que l'interface les voit : le jeton d'envoi n'en sort jamais,
/// seule sa présence est annoncée.
#[derive(Debug, Serialize)]
pub struct SettingsView {
    pub curseforge_username: Option<String>,
    pub range_days: i64,
    pub currency: String,
    pub curseforge_token_ready: bool,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> SettingsView {
    let settings = config::load_settings(&state.data_dir);
    SettingsView {
        curseforge_username: settings.curseforge_username,
        range_days: settings.range_days,
        currency: settings.currency,
        curseforge_token_ready: settings.curseforge_upload_token.is_some(),
    }
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    curseforge_username: Option<String>,
    range_days: i64,
    currency: Option<String>,
) -> Result<()> {
    // Le jeton d'envoi est relevé par l'application, jamais saisi : on garde
    // celui qui existe déjà plutôt que de l'écraser au premier enregistrement.
    let previous = config::load_settings(&state.data_dir);
    let settings = Settings {
        curseforge_username: curseforge_username
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),
        range_days: range_days.clamp(7, 730),
        currency: currency
            .map(|v| v.trim().to_uppercase())
            .filter(|v| v.len() == 3)
            .unwrap_or(previous.currency),
        curseforge_upload_token: previous.curseforge_upload_token,
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
    let mut overview = state
        .store
        .with(|conn| queries::overview(conn, &today, &from, &to, filter))?;
    overview.currency = currency_view(&state);
    Ok(overview)
}

/// Devise choisie et dernier taux relevé pour elle.
///
/// Un taux qui manque ou qui vise une autre devise vaut mieux ignoré qu'appliqué
/// de travers : on retombe alors sur le dollar, la monnaie des deux plateformes.
fn currency_view(state: &AppState) -> crate::models::CurrencyView {
    let code = config::load_settings(&state.data_dir).currency;
    if code == "USD" {
        return crate::models::CurrencyView::default();
    }
    let stored = state
        .store
        .with(|conn| crate::store::metrics::get_meta(conn, "fx_rate"))
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<crate::providers::rates::Rate>(&raw).ok())
        .filter(|rate| rate.currency == code);
    match stored {
        Some(rate) => crate::models::CurrencyView {
            code,
            rate: rate.rate,
            day: rate.day,
        },
        None => crate::models::CurrencyView::default(),
    }
}

/// Relève le taux du dollar vers la devise choisie et le conserve.
///
/// Appelée au démarrage et après chaque changement de devise : sans elle,
/// l'interface afficherait des euros au cours du dollar.
#[tauri::command]
pub async fn refresh_exchange_rate(
    state: State<'_, AppState>,
) -> Result<crate::models::CurrencyView> {
    let code = config::load_settings(&state.data_dir).currency;
    let rate = crate::providers::rates::usd_to(&code).await?;
    let raw = serde_json::to_string(&rate)?;
    state
        .store
        .with(|conn| crate::store::metrics::set_meta(conn, "fx_rate", &raw))?;
    Ok(crate::models::CurrencyView {
        code: rate.currency,
        rate: rate.rate,
        day: rate.day,
    })
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
        let title_of = |id: i64| {
            projects
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.title.clone())
        };

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
/// Crée la fenêtre si besoin. Cachée, elle sert à collecter sans déranger ;
/// visible, elle sert à la connexion.
fn ensure_curseforge_window(app: &tauri::AppHandle, visible: bool) -> Result<()> {
    if let Some(window) = app.get_webview_window(CF_WINDOW) {
        if visible {
            let _ = window.show();
            let _ = window.set_focus();
        }
        return Ok(());
    }
    let url = tauri::Url::parse(CF_AUTHOR_PAGE)
        .map_err(|e| AppError::Config(format!("adresse CurseForge invalide : {e}")))?;
    tauri::WebviewWindowBuilder::new(app, CF_WINDOW, tauri::WebviewUrl::External(url))
        .title("CurseForge — connexion à ton compte")
        .inner_size(1180.0, 860.0)
        .visible(visible)
        .build()
        .map_err(|e| AppError::Config(format!("ouverture de la fenêtre CurseForge : {e}")))?;
    Ok(())
}

#[tauri::command]
pub fn open_curseforge_window(app: tauri::AppHandle) -> Result<()> {
    ensure_curseforge_window(&app, true)
}

/// Note les appels que la page émet : méthode, adresse, état, et le début du
/// corps envoyé. Sert à découvrir comment le tableau de bord s'y prend pour
/// gérer projets et fichiers, là où aucune documentation ne le dit.
pub(crate) const WATCH_SCRIPT: &str = r#"(function () {
  if (window.__cgWatchOn) { return 'deja en place'; }
  window.__cgWatchOn = true;
  window.__cgWatch = window.__cgWatch || [];
  var out = window.__cgWatch;

  function note(method, url, status, sent) {
    if (out.length > 120) return;
    out.push({
      m: String(method || 'GET').toUpperCase(),
      u: String(url).slice(0, 300),
      s: status,
      envoi: typeof sent === 'string' ? sent.slice(0, 300) : ''
    });
  }

  var nativeFetch = window.fetch;
  if (nativeFetch) {
    window.fetch = function (input, init) {
      var url = typeof input === 'string' ? input : (input && input.url) || '';
      var method = (init && init.method) || (input && input.method) || 'GET';
      var sent = init && typeof init.body === 'string' ? init.body : '';
      return nativeFetch.apply(this, arguments).then(function (response) {
        note(method, url, response.status, sent);
        return response;
      }).catch(function (error) {
        note(method, url, -1, sent);
        throw error;
      });
    };
  }
  var open = XMLHttpRequest.prototype.open;
  var send = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.open = function (method, url) {
    this.__cgM = method;
    this.__cgU = url;
    return open.apply(this, arguments);
  };
  XMLHttpRequest.prototype.send = function (body) {
    var xhr = this;
    xhr.addEventListener('load', function () {
      note(xhr.__cgM, xhr.__cgU || '', xhr.status, typeof body === 'string' ? body : '');
    });
    return send.apply(this, arguments);
  };
  return 'ecoute posee';
})()"#;

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

/// Rend la fenêtre du tableau de bord, en la créant cachée si elle manque.
///
/// La collecte doit se faire seule : une fenêtre absente — jamais ouverte, ou
/// refermée entre deux relevés — se rouvre au lieu d'interrompre le travail.
pub(crate) async fn cf_window(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow> {
    if let Some(window) = app.get_webview_window(CF_WINDOW) {
        return Ok(window);
    }
    ensure_curseforge_window(app, false)?;
    let window = app
        .get_webview_window(CF_WINDOW)
        .ok_or_else(|| AppError::Config("la fenêtre CurseForge n'a pas pu s'ouvrir".into()))?;
    wait_until_loaded(&window).await;
    Ok(window)
}

/// Attend que l'application web du tableau de bord se soit dressée.
///
/// Son temps de chargement varie du simple au triple selon le réseau : sonder
/// la page vaut mieux qu'une attente fixe, trop courte un jour et perdue l'autre.
pub(crate) async fn wait_until_loaded(window: &tauri::WebviewWindow) {
    const READY: &str = r#"(function () {
      var body = document.body ? document.body.innerText : '';
      return document.readyState + '|' + body.length;
    })()"#;
    let mut mute = 0;
    for _ in 0..24 {
        match eval_raw(window, READY).await {
            Ok(raw) => {
                mute = 0;
                let state = raw.trim_matches('"').to_string();
                let filled = state
                    .split_once('|')
                    .and_then(|(_, len)| len.parse::<usize>().ok())
                    .unwrap_or(0);
                if state.starts_with("complete") && filled > 200 {
                    return;
                }
            }
            // Une fenêtre qui ne répond plus ne répondra pas davantage en
            // insistant : mieux vaut rendre la main que bloquer la collecte.
            Err(_) => {
                mute += 1;
                if mute >= 3 {
                    return;
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

/// Emmène la fenêtre sur une adresse et attend d'y être vraiment.
///
/// Demander la navigation ne suffit pas : le document d'avant reste en place
/// quelques instants, déjà chargé, et qui le lirait aussitôt lirait la page
/// qu'on vient de quitter. On attend donc que l'adresse ait changé, puis que la
/// nouvelle page se soit dressée.
pub(crate) async fn navigate(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    url: &str,
    expect: &str,
) -> Result<()> {
    let script = format!("(function () {{ location.href = {url:?}; return 'ok'; }})()");
    eval_in_window(app, &script).await?;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        if let Ok(raw) = eval_raw(window, "location.href").await {
            if raw.contains(expect) {
                break;
            }
        }
    }
    wait_until_loaded(window).await;
    wait_until_settled(window).await;
    Ok(())
}

/// Attend que la page cesse de grossir.
///
/// `readyState` passe à « complete » avant que tout soit là : le pied de page,
/// les tableaux et les listes arrivent après. Lire à cet instant donne une page
/// à moitié vide. On attend donc deux mesures identiques d'affilée.
pub(crate) async fn wait_until_settled(window: &tauri::WebviewWindow) {
    const SIZE: &str =
        "(function () { return (document.body ? document.body.innerText.length : 0); })()";
    let mut previous = 0usize;
    let mut stable = 0;
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        let Ok(raw) = eval_raw(window, SIZE).await else {
            continue;
        };
        let size = raw.trim_matches('"').parse::<usize>().unwrap_or(0);
        if size > 200 && size == previous {
            stable += 1;
            if stable >= 1 {
                return;
            }
        } else {
            stable = 0;
        }
        previous = size;
    }
}

/// Exécute un script dans la fenêtre CurseForge et rend son résultat.
pub(crate) async fn eval_in_window(app: &tauri::AppHandle, script: &str) -> Result<String> {
    let window = cf_window(app).await?;
    eval_raw(&window, script).await
}

async fn eval_raw(window: &tauri::WebviewWindow, script: &str) -> Result<String> {
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

/// Ouvre le tableau de bord et note ce que la page rend réellement.
///
/// Le site ne peut être observé qu'à travers un navigateur : cette sonde est le
/// seul moyen de savoir si la page s'affiche, si elle expose des liens, et si
/// l'écoute posée avant chargement l'empêche de démarrer. Le rapport est écrit
/// à côté de la base, pour être relu hors de l'application.
pub async fn probe_curseforge(app: tauri::AppHandle) {
    let armed_early = std::env::var("CF_PROBE_EARLY").is_ok();
    let url = match tauri::Url::parse(CF_AUTHOR_PAGE) {
        Ok(url) => url,
        Err(_) => return,
    };

    // La collecte ordinaire ouvre la même fenêtre au démarrage : on la reprend
    // si elle est déjà là plutôt que d'échouer sur un nom déjà pris.
    if app.get_webview_window(CF_WINDOW).is_none() {
        let mut builder =
            tauri::WebviewWindowBuilder::new(&app, CF_WINDOW, tauri::WebviewUrl::External(url))
                .title("CurseForge — sonde")
                .inner_size(1180.0, 860.0);
        if armed_early {
            builder = builder.initialization_script(CAPTURE_SCRIPT);
        }
        if let Err(error) = builder.build() {
            eprintln!("PROBE: ouverture impossible : {error}");
            return;
        }
    }

    let mut report = String::new();
    let note = |state: &PageState, label: &str, report: &mut String| {
        report.push_str(&format!(
            "--- {label}\nurl      : {}\nconnexion: {}\ntexte    : {} caractères\nliens    : {}\ncaptures : {}\ntexte    : {}\n",
            state.url,
            if asks_for_login(state) { "à faire" } else { "établie" },
            state.text.chars().count(),
            state.links.len(),
            state.captures.len(),
            state.text.chars().take(400).collect::<String>().replace('\n', " | ")
        ));
        for link in state.links.iter().take(30) {
            report.push_str(&format!("  lien    : {link}\n"));
        }
        for capture in state.captures.iter().take(20) {
            let series = serde_json::from_str::<serde_json::Value>(&capture.body)
                .map(|v| crate::scrape::find_daily_series(&v))
                .unwrap_or_default();
            report.push_str(&format!(
                "  capture : {} ({} octets, {} jours){}\n",
                capture.url,
                capture.body.len(),
                series.len(),
                if series.is_empty() {
                    format!(
                        " · début : {}",
                        capture.body.chars().take(160).collect::<String>()
                    )
                } else {
                    String::new()
                }
            ));
        }
    };

    // Exploration de l'interface interne, une fois la session établie : la page
    // l'appelle déjà pour se remplir, on lui demande simplement le reste.
    if std::env::var("CF_EXPLORE").is_ok() {
        tokio::time::sleep(std::time::Duration::from_secs(9)).await;
        let _ = eval_in_window(&app, EXPLORE_SCRIPT).await;
        tokio::time::sleep(std::time::Duration::from_secs(25)).await;
        if let Ok(raw) = eval_in_window(&app, "JSON.stringify(window.__cgExplore || [])").await {
            let unquoted: String = serde_json::from_str(&raw).unwrap_or(raw);
            let path = data_dir(&app).join("cf_explore.txt");
            let _ = std::fs::write(&path, &unquoted);
            println!("PROBE: exploration écrite dans {}", path.display());
        }
    }

    tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    let mut page = match page_state(&app).await {
        Ok(page) => page,
        Err(error) => {
            let _ = std::fs::write(
                data_dir(&app).join("cf_probe.txt"),
                format!("lecture impossible : {error}"),
            );
            return;
        }
    };
    note(&page, "état initial", &mut report);

    if !asks_for_login(&page) {
        let _ = eval_in_window(&app, CAPTURE_SCRIPT).await;
        let targets = crate::collect::worth_visiting(&page.links);
        report.push_str(&format!("\npages retenues : {}\n", targets.len()));
        for target in targets.iter().take(6) {
            let script = format!("(function () {{ location.href = {target:?}; return 'ok'; }})()");
            let _ = eval_in_window(&app, &script).await;
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let _ = eval_in_window(&app, CAPTURE_SCRIPT).await;
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            if let Ok(next) = page_state(&app).await {
                page = next;
                note(&page, &format!("après visite de {target}"), &mut report);
            }
        }
    }

    let path = data_dir(&app).join("cf_probe.txt");
    let _ = std::fs::write(&path, &report);
    println!("PROBE: rapport écrit dans {}", path.display());
    println!("{report}");
}

/// Interroge l'interface interne du tableau de bord depuis la page elle-même.
///
/// Les appels partent du site, avec sa session : c'est la seule façon de
/// franchir son filtre anti-robot. On ne demande que ce que le tableau de bord
/// demande déjà pour s'afficher.
const EXPLORE_SCRIPT: &str = r#"(function () {
  window.__cgExplore = [];
  var out = window.__cgExplore;
  function probe(path) {
    return fetch(path, { credentials: 'include' })
      .then(function (r) { return r.text().then(function (t) {
        out.push({ path: path, status: r.status, body: t.slice(0, 900) });
      }); })
      .catch(function (e) { out.push({ path: path, status: -1, body: String(e) }); });
  }
  // Le tableau de bord est bâti sur react-admin : ses adresses suivent une
  // convention, `?filter=&range=&sort=` pour lister, `/{id}` pour lire. On
  // essaie donc les ressources qu'il manipule, en lecture seule.
  var query = '?filter=%7B%7D&range=%5B0%2C3%5D&sort=%5B%22id%22%2C%22DESC%22%5D';
  var byProject = '?filter=%7B%22projectId%22%3A1002185%7D&range=%5B0%2C5%5D&sort=%5B%22id%22%2C%22DESC%22%5D';
  var jobs = [];
  ['files', 'project-files', 'projectFiles', 'uploads', 'versions',
   'categories', 'game-versions', 'gameVersions', 'issues', 'builds',
   'projects/1002185/file', 'projects/1002185/upload'].forEach(function (name) {
    jobs.push('/_api/' + name + query);
  });
  ['files', 'project-files', 'uploads', 'issues'].forEach(function (name) {
    jobs.push('/_api/' + name + byProject);
  });
  jobs.reduce(function (chain, path) {
    return chain.then(function () { return probe(path); });
  }, Promise.resolve());
  return 'exploration lancee sur ' + jobs.length + ' adresses';
})()"#;

/// Résultat d'une collecte automatique.
#[derive(Debug, Default, Serialize)]
pub struct CfCollect {
    /// Vrai tant que l'utilisateur n'est pas connecté : la fenêtre reste ouverte.
    pub needs_login: bool,
    pub visited: Vec<String>,
    /// Séries retenues, projet par projet.
    pub imported: Vec<CfImported>,
    /// Solde de points relevé au passage.
    pub points: Option<i64>,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct CfImported {
    pub title: String,
    pub days: usize,
    pub from: String,
    pub to: String,
}

/// Script qui rend l'état de la page : adresse, connexion, liens du tableau de
/// bord et réponses captées depuis l'armement.
const STATE_SCRIPT: &str = r#"(function () {
  var store = window.__chartographer || { captures: [] };
  var links = [];
  var anchors = document.querySelectorAll('a[href]');
  for (var i = 0; i < anchors.length && links.length < 400; i++) {
    links.push(anchors[i].href);
  }
  var text = document.body ? document.body.innerText : '';
  return {
    url: location.href,
    ready: document.readyState,
    text: text.slice(0, 20000),
    links: links,
    captures: store.captures
  };
})()"#;

#[derive(serde::Deserialize)]
struct RawCapture {
    url: String,
    body: String,
}

#[derive(serde::Deserialize)]
struct PageState {
    url: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    links: Vec<String>,
    #[serde(default)]
    captures: Vec<RawCapture>,
}

async fn page_state(app: &tauri::AppHandle) -> Result<PageState> {
    let raw = eval_in_window(app, STATE_SCRIPT).await?;
    serde_json::from_str(&raw).map_err(|e| AppError::Data(format!("état de page illisible : {e}")))
}

fn asks_for_login(state: &PageState) -> bool {
    crate::collect::is_login_page(&state.url, &state.text)
}

/// Interroge l'interface du tableau de bord et rend ses réponses.
///
/// Les adresses ont été relevées en observant la page se remplir. Les appels
/// partent d'elle, avec sa session : c'est la seule façon de franchir le filtre
/// qui refuse toute requête faite hors d'un navigateur.
const FETCH_SCRIPT: &str = r#"(function () {
  window.__cgData = null;
  var out = {};
  var paths = {
    projects: '/_api/projects/compact?filter=%7B%22amOwner%22%3Atrue%2C%22status%22%3A4%7D&range=%5B0%2C999%5D&sort=%5B%22totalDownloads%22%2C%22DESC%22%5D',
    downloads: '/_api/statistics/queries/downloads',
    revenue: '/_api/statistics/queries/lastMonthRevenue',
    estimation: '/_api/statistics/queries/revenueEstimation'
  };
  var names = Object.keys(paths);
  names.reduce(function (chain, name) {
    return chain.then(function () {
      return fetch(paths[name], { credentials: 'include' })
        .then(function (r) { return r.text(); })
        .then(function (t) { out[name] = t; })
        .catch(function (e) { out[name] = 'erreur ' + e; });
    });
  }, Promise.resolve()).then(function () {
    // Nom du compte connecté : aucune adresse n'est documentée, on essaie les
    // formes usuelles et on garde la première réponse qui ressemble à du JSON.
    var who = ['/_api/users/me', '/_api/user', '/_api/me', '/_api/account', '/_api/profile'];
    return who.reduce(function (chain, path) {
      return chain.then(function () {
        if (out.account) return null;
        return fetch(path, { credentials: 'include' })
          .then(function (r) { return r.ok ? r.text() : ''; })
          .then(function (t) { if (t && t.charAt(0) === '{') out.account = t.slice(0, 4000); })
          .catch(function () {});
      });
    }, Promise.resolve());
  }).then(function () {
    out.balance = (document.body ? document.body.innerText : '').slice(0, 4000);
    window.__cgData = out;
  });
  return 'appels lances';
})()"#;

/// Solde de points affiché par le tableau de bord.
///
/// Le bandeau montre « My Balance », le nombre de points puis leur contre-valeur
/// en dollars. Aucune adresse ne le sert : il est lu là où il s'affiche.
fn balance_from_text(text: &str) -> Option<i64> {
    let lower = text.to_lowercase();
    let start = lower.find("my balance")? + "my balance".len();
    let rest = &text[start..];
    let digits: String = rest
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// Réponses brutes du tableau de bord, telles que la page les a rapportées.
#[derive(serde::Deserialize, Default)]
struct Fetched {
    #[serde(default)]
    downloads: String,
    #[serde(default)]
    revenue: String,
    #[serde(default)]
    estimation: String,
    #[serde(default)]
    balance: String,
    /// Réponse qui nomme le compte connecté, quand l'une des adresses répond.
    #[serde(default)]
    account: String,
}

/// Lance les appels depuis la page et attend leur retour.
///
/// L'attente est sondée plutôt que fixée : les appels s'enchaînent, et leur
/// durée dépend du réseau du jour.
async fn fetch_dashboard(app: &tauri::AppHandle) -> Result<Fetched> {
    let _ = eval_in_window(app, FETCH_SCRIPT).await;
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
        let raw = eval_in_window(app, "JSON.stringify(window.__cgData)").await?;
        let unquoted: String = serde_json::from_str(&raw).unwrap_or(raw);
        if unquoted.trim() == "null" || unquoted.trim().is_empty() {
            continue;
        }
        return Ok(serde_json::from_str(&unquoted).unwrap_or_default());
    }
    Ok(Fetched::default())
}

/// Collecte le tableau de bord CurseForge sans rien demander à l'utilisateur.
///
/// La fenêtre reste invisible et se referme après coup ; elle ne s'affiche que
/// si la session a expiré et qu'il faut se reconnecter.
#[tauri::command]
pub async fn collect_curseforge(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<CfCollect> {
    let window = cf_window(&app).await?;
    wait_until_loaded(&window).await;
    tracing::debug!("collecte CurseForge : fenêtre prête");

    let page = page_state(&app).await?;
    if asks_for_login(&page) {
        return Ok(CfCollect {
            needs_login: true,
            detail: "session CurseForge expirée : une connexion est nécessaire".into(),
            ..Default::default()
        });
    }

    let mut fetched = fetch_dashboard(&app).await?;
    // Un relevé qui revient vide vient presque toujours de la même cause : la
    // page n'était pas encore sur le tableau de bord quand les appels sont
    // partis. On l'y ramène et on redemande une fois avant de conclure.
    if fetched.downloads.trim().is_empty() && fetched.revenue.trim().is_empty() {
        tracing::debug!("relevé vide : retour au tableau de bord puis seconde tentative");
        let _ = navigate(&app, &window, CF_AUTHOR_PAGE, "authors.curseforge.com").await;
        let second = fetch_dashboard(&app).await?;
        if !second.downloads.trim().is_empty() || !second.revenue.trim().is_empty() {
            fetched = second;
        }
    }

    // Les projets connus, avec leur identifiant CurseForge et leur titre : les
    // colonnes de la série portent l'un ou l'autre.
    let known: Vec<(i64, String, String)> = state.store.with(|conn| {
        Ok(p::list(conn)?
            .into_iter()
            .filter(|project| project.platform == Platform::CurseForge)
            .map(|project| (project.id, project.ext_id, project.title))
            .collect())
    })?;

    let series = crate::collect::parse_downloads_query(&fetched.downloads);
    let mut per_project: std::collections::HashMap<i64, Vec<(String, i64)>> =
        std::collections::HashMap::new();
    let mut orphan_columns: std::collections::BTreeSet<String> = Default::default();
    for point in &series {
        match crate::collect::match_column(&point.key, &known) {
            Some(project_id) => per_project
                .entry(project_id)
                .or_default()
                .push((point.day.clone(), point.downloads)),
            None => {
                orphan_columns.insert(point.key.clone());
            }
        }
    }

    let mut imported: Vec<CfImported> = Vec::new();
    for (project_id, days) in per_project {
        state.store.with(|conn| {
            for (day, downloads) in &days {
                crate::store::metrics::upsert_daily(
                    conn,
                    project_id,
                    day,
                    Some(*downloads),
                    None,
                    None,
                )?;
            }
            Ok(())
        })?;
        let title = known
            .iter()
            .find(|(id, _, _)| *id == project_id)
            .map(|(_, _, title)| title.clone())
            .unwrap_or_default();
        let mut sorted: Vec<String> = days.iter().map(|(day, _)| day.clone()).collect();
        sorted.sort();
        imported.push(CfImported {
            title,
            days: days.len(),
            from: sorted.first().cloned().unwrap_or_default(),
            to: sorted.last().cloned().unwrap_or_default(),
        });
    }
    imported.sort_by_key(|row| std::cmp::Reverse(row.days));

    // Solde de points et revenus estimés, conservés tels que le tableau de bord
    // les annonce.
    let points = balance_from_text(&fetched.balance);
    if let Some(value) = points {
        let today = sync::today_utc();
        let now = Utc::now().to_rfc3339();
        state
            .store
            .with(|conn| crate::store::metrics::record_cf_points(conn, &today, value, &now))?;
    }
    // Le compte CurseForge connecté n'a aucune raison de porter le même nom que
    // le compte Modrinth : on relève le sien plutôt que de le deviner.
    if let Some(name) = crate::collect::parse_account_name(&fetched.account) {
        state
            .store
            .with(|conn| crate::store::metrics::set_meta(conn, "curseforge_account", &name))?;
    }

    let months = crate::collect::parse_revenue_series(&fetched.revenue);
    let (last_month, year_to_date) = crate::collect::parse_revenue_estimation(&fetched.estimation);
    let now = Utc::now().to_rfc3339();
    state.store.with(|conn| {
        for month in &months {
            crate::store::metrics::record_cf_revenue(conn, &month.month, month.amount, &now)?;
        }
        let money = |value: Option<f64>| value.map(|v| format!("{v:.2}")).unwrap_or_default();
        crate::store::metrics::set_meta(conn, "curseforge_revenue_last_month", &money(last_month))?;
        crate::store::metrics::set_meta(conn, "curseforge_revenue_ytd", &money(year_to_date))
    })?;

    // Le jeton d'envoi se relève au même passage, tant que la session est là.
    // Il n'a pas à être réclamé plus tard, ni saisi : la fenêtre est déjà
    // ouverte sur le bon compte, autant s'en servir une bonne fois.
    let token_note = if config::load_settings(&state.data_dir)
        .curseforge_upload_token
        .is_none()
    {
        tracing::debug!("relevé du jeton d'envoi CurseForge");
        match crate::publish_api::capture_token(&app, &state).await {
            Ok(true) => " · jeton d'envoi relevé",
            Ok(false) => " · aucun jeton d'envoi lisible",
            Err(error) => {
                tracing::debug!(%error, "relevé du jeton impossible");
                ""
            }
        }
    } else {
        ""
    };

    // La fenêtre a fini son travail : elle s'efface sans se fermer, pour garder
    // la session ouverte et repartir sans rien recharger au prochain relevé.
    let _ = window.hide();

    let detail = format!(
        "{} jours relevés · {} mods rattachés · {} mois de revenus{}{token_note}",
        series.len(),
        imported.len(),
        months.len(),
        if orphan_columns.is_empty() {
            String::new()
        } else {
            format!(
                " · colonnes sans mod connu : {}",
                orphan_columns.into_iter().collect::<Vec<_>>().join(", ")
            )
        }
    );

    Ok(CfCollect {
        needs_login: false,
        visited: vec![page.url],
        imported,
        points,
        detail,
    })
}

#[allow(dead_code)]
async fn collect_curseforge_by_browsing(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<CfCollect> {
    // La fenêtre reste cachée tant que la session tient : la collecte ne doit
    // rien réclamer à l'utilisateur.
    ensure_curseforge_window(&app, false)?;
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;

    let mut page = page_state(&app).await?;
    if asks_for_login(&page) {
        ensure_curseforge_window(&app, true)?;
        return Ok(CfCollect {
            needs_login: true,
            detail: "connecte-toi dans la fenêtre CurseForge, puis relance la collecte".into(),
            ..Default::default()
        });
    }

    // L'écoute se pose sur la page déjà chargée : l'injecter plus tôt empêchait
    // l'application web de démarrer.
    let armed = eval_in_window(&app, CAPTURE_SCRIPT).await?;

    let known: Vec<(i64, String)> = state.store.with(|conn| {
        Ok(p::list(conn)?
            .into_iter()
            .filter(|project| project.platform == Platform::CurseForge)
            .map(|project| (project.id, project.ext_id))
            .collect())
    })?;

    // Les pages à visiter sortent des liens du tableau de bord lui-même : ses
    // adresses changent au gré des refontes, ses liens non.
    let mut queue = crate::collect::worth_visiting(&page.links);
    queue.truncate(12);
    let mut visited: Vec<String> = Vec::new();

    for target in queue {
        let script = format!("(function () {{ location.href = {target:?}; return 'ok'; }})()");
        if eval_in_window(&app, &script).await.is_err() {
            continue;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2200)).await;
        // Chaque page recharge le document : l'écoute doit être reposée.
        let _ = eval_in_window(&app, CAPTURE_SCRIPT).await;
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
        if let Ok(next) = page_state(&app).await {
            visited.push(next.url.clone());
            page = next;
        }
    }

    let mut imported: Vec<CfImported> = Vec::new();
    let mut points: Option<i64> = crate::scrape::extract_points(&page.text);

    for capture in &page.captures {
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&capture.body) else {
            continue;
        };
        let series = crate::scrape::find_daily_series(&parsed);
        if series.is_empty() {
            continue;
        }
        let Some(project_id) = crate::collect::pick_project_id(&capture.url, &capture.body, &known)
        else {
            continue;
        };

        let title = state.store.with(|conn| {
            Ok(p::list(conn)?
                .into_iter()
                .find(|project| project.id == project_id)
                .map(|project| project.title)
                .unwrap_or_else(|| format!("projet {project_id}")))
        })?;

        state.store.with(|conn| {
            for point in &series {
                crate::store::metrics::upsert_daily(
                    conn,
                    project_id,
                    &point.day,
                    Some(point.value),
                    None,
                    None,
                )?;
            }
            Ok(())
        })?;

        imported.push(CfImported {
            title,
            days: series.len(),
            from: series.first().map(|p| p.day.clone()).unwrap_or_default(),
            to: series.last().map(|p| p.day.clone()).unwrap_or_default(),
        });
    }

    if points.is_none() {
        points = crate::scrape::extract_points(&page.text);
    }
    if let Some(value) = points {
        let today = sync::today_utc();
        let now = Utc::now().to_rfc3339();
        state
            .store
            .with(|conn| crate::store::metrics::record_cf_points(conn, &today, value, &now))?;
    }

    let detail = format!(
        "{armed} · {} pages visitées · {} réponses écoutées · {} séries retenues",
        visited.len(),
        page.captures.len(),
        imported.len()
    );

    Ok(CfCollect {
        needs_login: false,
        visited,
        imported,
        points,
        detail,
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
        return Err(AppError::Data(
            "aucune série datée dans cette source".into(),
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

#[cfg(test)]
mod tests {
    use super::account_page;

    #[test]
    fn builds_the_public_page_of_each_platform() {
        assert_eq!(
            account_page("modrinth", "DreykaOas").as_deref(),
            Some("https://modrinth.com/user/DreykaOas")
        );
        assert_eq!(
            account_page("curseforge", "DreykaOas_official").as_deref(),
            Some("https://www.curseforge.com/members/DreykaOas_official/projects")
        );
    }

    #[test]
    fn refuses_what_is_not_a_pseudonym() {
        assert_eq!(account_page("modrinth", "  "), None);
        assert_eq!(account_page("steam", "DreykaOas"), None);
        // Un nom porteur de séparateurs ne peut pas détourner l'adresse.
        assert_eq!(
            account_page("modrinth", "a/../../settings?x=1").as_deref(),
            Some("https://modrinth.com/user/a....settingsx1")
        );
    }
}
