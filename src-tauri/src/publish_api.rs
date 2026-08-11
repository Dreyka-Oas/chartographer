//! Commandes de publication, appelées depuis l'onglet du même nom.
//!
//! Le travail se répartit en deux : Modrinth accepte tout par son interface
//! publique — créer un projet, y poser une version, supprimer l'une ou l'autre.
//! CurseForge n'accepte que le dépôt de fichiers sur un projet existant, et
//! demande un jeton d'auteur que l'application va chercher elle-même.

use crate::commands::AppState;
use crate::config;
use crate::error::{AppError, Result};
use crate::providers::curseforge_upload::UploadClient;
use crate::providers::modrinth::ModrinthClient;
use crate::publish::{self, Draft, GameVersion};
use serde::Serialize;
use tauri::State;

/// Ce qu'une plateforme a répondu, dit simplement.
#[derive(Debug, Serialize)]
pub struct Outcome {
    pub platform: String,
    pub ok: bool,
    /// Identifiant de ce qui vient d'être créé, quand la plateforme en rend un.
    pub id: Option<String>,
    pub detail: String,
}

impl Outcome {
    fn refused(platform: &str, status: u16, body: &str) -> Self {
        let mut reason = publish::refusal_reason(body).unwrap_or_else(|| {
            let excerpt: String = body.chars().take(180).collect();
            if excerpt.trim().is_empty() {
                format!("HTTP {status}")
            } else {
                excerpt
            }
        });
        // Modrinth répond 401 aussi bien pour un jeton invalide que pour un
        // jeton dépourvu du droit demandé : le dire évite une longue recherche.
        if status == 401 && platform == "modrinth" {
            reason.push_str(
                " · un jeton qui lit ne suffit pas à écrire : régénère-le en cochant \
                 PROJECT_CREATE, PROJECT_DELETE, VERSION_CREATE et VERSION_DELETE",
            );
        }
        Outcome {
            platform: platform.into(),
            ok: false,
            id: None,
            detail: format!("refusé ({status}) : {reason}"),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PublishReport {
    pub outcomes: Vec<Outcome>,
}

fn read_file(path: &str) -> Result<(String, Vec<u8>)> {
    let file = std::path::Path::new(path);
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AppError::Config(format!("chemin de fichier illisible : {path}")))?
        .to_string();
    if !publish::accepts_extension(&name) {
        return Err(AppError::Config(format!(
            "{name} n'est pas une archive publiable : seuls .jar, .zip, .mrpack et .litemod sont acceptés"
        )));
    }
    let bytes = std::fs::read(file)
        .map_err(|e| AppError::Config(format!("lecture de {name} impossible : {e}")))?;
    Ok((name, bytes))
}

fn modrinth_client(state: &AppState) -> Result<ModrinthClient> {
    let session = config::require_token(&state.data_dir)?;
    ModrinthClient::new(&session.token)
}

fn upload_client(state: &AppState) -> Result<UploadClient> {
    let token = config::load_settings(&state.data_dir)
        .curseforge_upload_token
        .ok_or_else(|| {
            AppError::Config(
                "aucun jeton d'envoi CurseForge : l'application doit d'abord le relever".into(),
            )
        })?;
    UploadClient::new(&token)
}

/// Page où CurseForge gère les jetons d'envoi de l'auteur.
///
/// Le tableau de bord actuel ne les montre plus : ils sont restés sur l'ancien
/// site. L'adresse publique y redirige en emportant la session, alors que viser
/// l'ancien site directement rend une page dépouillée.
const TOKEN_PAGE: &str = "https://www.curseforge.com/account/api-tokens";

/// Relève les jetons du compte en redemandant la page, plutôt qu'en la lisant
/// à l'écran.
///
/// Une fenêtre cachée ne dessine pas : son document reste à moitié vide, et le
/// tableau des jetons n'y paraît jamais. La page est donc redemandée depuis
/// elle-même — même origine, même session — et les jetons sont cherchés dans la
/// réponse, qui, elle, est toujours complète.
const READ_TOKENS: &str = r#"(function () {
  window.__cgTokens = null;
  var motif = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi;
  var uniques = [];
  function ajoute(source) {
    (String(source).match(motif) || []).forEach(function (t) {
      if (uniques.indexOf(t) < 0) uniques.push(t);
    });
  }
  ajoute(document.documentElement ? document.documentElement.innerHTML : '');
  // La page est aussi redemandée : une fenêtre cachée ne dessine pas tout, mais
  // la réponse, elle, est toujours entière.
  fetch(location.href, { credentials: 'include' })
    .then(function (r) { return r.text(); })
    .then(function (html) { ajoute(html); window.__cgTokens = uniques; })
    .catch(function () { window.__cgTokens = uniques; });
  return 'lecture lancee';
})()"#;

/// Demande un nouveau jeton, nommé pour qu'on sache d'où il vient.
///
/// Un jeton existant n'est jamais réaffiché : le site ne le montre qu'une fois,
/// à sa création. En obtenir un est donc le seul moyen d'en avoir un, et le nom
/// permet de le reconnaître — et de le révoquer — depuis le compte.
const GENERATE_TOKEN: &str = r#"(function () {
  var champ = document.querySelector('input[name=name]');
  if (!champ) { return 'champ absent'; }
  var setter = Object.getOwnPropertyDescriptor(
    window.HTMLInputElement.prototype, 'value'
  ).set;
  setter.call(champ, 'Chartographer');
  champ.dispatchEvent(new Event('input', { bubbles: true }));
  champ.dispatchEvent(new Event('change', { bubbles: true }));
  var formulaire = champ.form;
  if (!formulaire) { return 'formulaire absent'; }
  var envoi = formulaire.querySelector('input[type=submit], button[type=submit]');
  if (envoi) { envoi.click(); return 'demandé'; }
  formulaire.submit();
  return 'envoyé';
})()"#;

/// Obtient le jeton d'envoi CurseForge sans rien demander à l'utilisateur.
///
/// Un jeton déjà émis n'est jamais réaffiché : le compte ne le montre qu'à sa
/// création. L'application en demande donc un, nommé « Chartographer », le lit
/// sur la page qui suit, puis l'essaie contre l'interface d'envoi avant de le
/// garder. Il ne ressort jamais vers la fenêtre.
#[tauri::command]
pub async fn capture_curseforge_token(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<bool> {
    capture_token(&app, &state).await
}

/// Même travail, appelable hors commande : le démarrage s'en sert pour éprouver
/// la publication sans passer par l'interface.
pub async fn capture_token(app: &tauri::AppHandle, state: &AppState) -> Result<bool> {
    let window = crate::commands::cf_window(app).await?;
    let previous = crate::commands::eval_in_window(app, "location.href")
        .await
        .unwrap_or_default();

    // Une fenêtre cachée ne dessine pas, et le tableau des jetons n'arrive
    // qu'au rendu. On la sort donc de sa cachette — mais loin hors de l'écran,
    // pour que personne ne la voie passer.
    let posted = window.outer_position().ok();
    let _ = window.set_position(tauri::PhysicalPosition::new(-8000, -8000));
    let _ = window.show();

    crate::commands::navigate(app, &window, TOKEN_PAGE, "api-tokens").await?;

    // Le compte n'affiche jamais un jeton déjà émis : on en demande un, nommé,
    // et on le lit sur la page qui suit — la seule où il paraît en clair.
    let asked = crate::commands::eval_in_window(app, GENERATE_TOKEN).await?;
    tracing::debug!(reponse = %asked, "jeton d'envoi demandé au compte");
    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
    crate::commands::wait_until_loaded(&window).await;
    crate::commands::wait_until_settled(&window).await;
    let candidates = read_tokens(app).await?;

    let mut kept = None;
    for candidate in candidates {
        if let Ok(client) = UploadClient::new(&candidate) {
            match client.game_versions().await {
                Ok(list) => {
                    tracing::debug!(
                        versions = list.len(),
                        "jeton accepté par l'interface d'envoi"
                    );
                    kept = Some(candidate);
                    break;
                }
                Err(error) => tracing::debug!(%error, "jeton refusé"),
            }
        }
    }

    // Retour d'où l'on vient : la collecte des statistiques attend le tableau
    // de bord, pas la page des jetons.
    let back = previous.trim_matches('"').to_string();
    let target = if back.contains("curseforge.com") && !back.contains("api-tokens") {
        back
    } else {
        "https://authors.curseforge.com/".to_string()
    };
    let _ = crate::commands::navigate(app, &window, &target, "curseforge.com").await;

    // La fenêtre retourne d'où elle vient : cachée, et à sa place.
    let _ = window.hide();
    if let Some(position) = posted {
        let _ = window.set_position(position);
    }

    match kept {
        Some(token) => {
            let mut settings = config::load_settings(&state.data_dir);
            settings.curseforge_upload_token = Some(token);
            config::save_settings(&state.data_dir, &settings)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Ouvre le tableau de bord et se met à regarder.
///
/// CurseForge ne documente pas comment son site crée un projet ni retire un
/// fichier, et un corps deviné ne récolte qu'une erreur serveur muette.
/// L'application observe donc le geste une fois, fait par son auteur, et sait
/// ensuite le refaire seule.
#[tauri::command]
pub async fn watch_curseforge(app: tauri::AppHandle) -> Result<String> {
    crate::commands::cf_window(&app).await?;
    crate::commands::open_curseforge_window(app.clone())?;
    let raw = crate::commands::eval_in_window(&app, crate::commands::WATCH_SCRIPT).await?;
    Ok(raw.trim_matches('"').to_string())
}

/// Relève ce que la fenêtre a vu passer et en tire les gestes réutilisables.
#[tauri::command]
pub async fn learn_curseforge(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<crate::gestures::Gesture>> {
    let raw =
        crate::commands::eval_in_window(&app, "JSON.stringify(window.__cgWatch || [])").await?;
    let unquoted: String = serde_json::from_str(&raw).unwrap_or(raw);
    let observed: Vec<crate::gestures::Observed> =
        serde_json::from_str(&unquoted).unwrap_or_default();

    // Ce qui a été appris auparavant ne s'efface pas : chaque séance complète
    // le carnet, geste par geste.
    let mut known = stored_gestures(&state)?;
    for gesture in crate::gestures::learn(&observed) {
        match known
            .iter_mut()
            .find(|g| g.method == gesture.method && g.pattern == gesture.pattern)
        {
            Some(existing) => *existing = gesture,
            None => known.push(gesture),
        }
    }
    let raw = serde_json::to_string(&known)?;
    state
        .store
        .with(|conn| crate::store::metrics::set_meta(conn, "curseforge_gestures", &raw))?;
    Ok(known)
}

/// Gestes déjà appris, pour que l'interface dise ce qu'elle sait faire.
#[tauri::command]
pub fn curseforge_gestures(state: State<'_, AppState>) -> Result<Vec<crate::gestures::Gesture>> {
    stored_gestures(&state)
}

fn stored_gestures(state: &AppState) -> Result<Vec<crate::gestures::Gesture>> {
    let raw = state
        .store
        .with(|conn| crate::store::metrics::get_meta(conn, "curseforge_gestures"))?;
    Ok(raw
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default())
}

/// Refait un geste appris dans la fenêtre du tableau de bord.
///
/// L'appel part de la page, avec sa session : c'est la seule façon de franchir
/// le filtre qui refuse toute requête faite hors d'un navigateur.
async fn replay(
    app: &tauri::AppHandle,
    method: &str,
    url: &str,
    body: Option<&str>,
) -> Result<(i64, String)> {
    let body_literal = match body {
        Some(body) => serde_json::to_string(body)?,
        None => "null".to_string(),
    };
    let script = format!(
        r#"(function () {{
          window.__cgReplay = null;
          var init = {{ method: {method:?}, credentials: 'include' }};
          var corps = {body_literal};
          if (corps !== null) {{
            init.headers = {{ 'Content-Type': 'application/json' }};
            init.body = corps;
          }}
          fetch({url:?}, init).then(function (r) {{
            return r.text().then(function (t) {{
              window.__cgReplay = {{ s: r.status, t: t.slice(0, 4000) }};
            }});
          }}).catch(function (e) {{
            window.__cgReplay = {{ s: -1, t: String(e) }};
          }});
          return 'parti';
        }})()"#
    );
    crate::commands::eval_in_window(app, &script).await?;

    // La réponse revient quand elle revient : on repasse voir plutôt que de
    // fixer une attente qui serait trop courte un jour et perdue le reste.
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let raw = crate::commands::eval_in_window(app, "JSON.stringify(window.__cgReplay)").await?;
        let unquoted: String = serde_json::from_str(&raw).unwrap_or(raw);
        if unquoted == "null" || unquoted.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(&unquoted).unwrap_or_default();
        return Ok((
            value["s"].as_i64().unwrap_or(-1),
            value["t"].as_str().unwrap_or_default().to_string(),
        ));
    }
    Err(AppError::Remote {
        provider: "CurseForge".into(),
        detail: "le tableau de bord n'a pas répondu".into(),
    })
}

/// Crée un projet CurseForge en refaisant le geste appris.
#[tauri::command]
pub async fn create_curseforge_project(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    name: String,
    summary: String,
) -> Result<Outcome> {
    let gestures = stored_gestures(&state)?;
    let Some(gesture) = crate::gestures::creation(&gestures) else {
        return Err(AppError::Config(
            "CurseForge ne publie pas comment son site crée un projet : montre-le une fois \
             depuis l'onglet Publication, l'application saura ensuite le refaire"
                .into(),
        ));
    };
    let body = crate::gestures::adapt_body(
        &gesture.body,
        &[
            ("name", serde_json::json!(name)),
            ("summary", serde_json::json!(summary)),
        ],
    );
    let (status, response) = replay(&app, &gesture.method, &gesture.pattern, Some(&body)).await?;
    if !(200..300).contains(&status) {
        return Ok(Outcome::refused("curseforge", status as u16, &response));
    }
    let id = serde_json::from_str::<serde_json::Value>(&response)
        .ok()
        .and_then(|v| v["id"].as_i64())
        .map(|id| id.to_string());
    Ok(Outcome {
        platform: "curseforge".into(),
        ok: true,
        id,
        detail: format!("projet « {name} » créé"),
    })
}

/// Retire un fichier CurseForge en refaisant le geste appris.
#[tauri::command]
pub async fn delete_curseforge_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_id: i64,
    file_id: i64,
) -> Result<Outcome> {
    let gestures = stored_gestures(&state)?;
    let Some(gesture) = crate::gestures::file_removal(&gestures) else {
        return Err(AppError::Config(
            "l'interface d'envoi CurseForge ne sait pas retirer un fichier : montre le geste \
             une fois depuis l'onglet Publication, l'application saura ensuite le refaire"
                .into(),
        ));
    };
    // Deux repères au plus : le projet puis le fichier, ou le fichier seul.
    let url = if gesture.pattern.contains("{2}") {
        crate::gestures::fill(&gesture.pattern, &[project_id, file_id])
    } else {
        crate::gestures::fill(&gesture.pattern, &[file_id])
    };
    let body = (!gesture.body.is_empty()).then_some(gesture.body.as_str());
    let (status, response) = replay(&app, &gesture.method, &url, body).await?;
    if !(200..300).contains(&status) {
        return Ok(Outcome::refused("curseforge", status as u16, &response));
    }
    Ok(Outcome {
        platform: "curseforge".into(),
        ok: true,
        id: Some(file_id.to_string()),
        detail: format!("fichier {file_id} retiré"),
    })
}

/// Fichiers d'un projet CurseForge, tels que son tableau de bord les liste.
#[tauri::command]
pub async fn curseforge_files(app: tauri::AppHandle, project_id: i64) -> Result<serde_json::Value> {
    let url = format!(
        "/_api/project-files?filter=%7B%22projectId%22%3A%22{project_id}%22%7D\
         &range=%5B0%2C24%5D&sort=%5B%22DateCreated%22%2C%22DESC%22%5D"
    );
    let (status, body) = replay(&app, "GET", &url, None).await?;
    if !(200..300).contains(&status) {
        return Err(AppError::Remote {
            provider: "CurseForge".into(),
            detail: format!("liste des fichiers refusée ({status})"),
        });
    }
    Ok(serde_json::from_str(&body).unwrap_or(serde_json::Value::Null))
}

/// Lit les jetons affichés, en laissant à la page le temps de les afficher.
///
/// Le tableau arrive après le reste de la page : attendre une taille stable ne
/// suffit pas, elle grossit par paliers. On attend donc exactement ce qu'on
/// cherche, et on s'arrête dès qu'il paraît.
async fn read_tokens(app: &tauri::AppHandle) -> Result<Vec<String>> {
    crate::commands::eval_in_window(app, READ_TOKENS).await?;
    let mut found: Vec<String> = Vec::new();
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let raw = crate::commands::eval_in_window(app, "JSON.stringify(window.__cgTokens)").await?;
        let unquoted: String = serde_json::from_str(&raw).unwrap_or(raw);
        if unquoted == "null" || unquoted.is_empty() {
            continue;
        }
        found = serde_json::from_str(&unquoted).unwrap_or_default();
        break;
    }
    tracing::debug!(trouves = found.len(), "jetons lus sur la page du compte");
    Ok(found)
}

/// Catalogue des versions de jeu CurseForge, gardé en base entre deux envois.
///
/// Il change à chaque sortie de Minecraft mais pas d'une minute à l'autre : le
/// relire à chaque publication ne servirait qu'à ralentir.
#[tauri::command]
pub async fn curseforge_game_versions(state: State<'_, AppState>) -> Result<Vec<GameVersion>> {
    game_versions(&state).await
}

async fn game_versions(state: &AppState) -> Result<Vec<GameVersion>> {
    let cached = state
        .store
        .with(|conn| crate::store::metrics::get_meta(conn, "curseforge_game_versions"))?;
    if let Some(raw) = cached {
        if let Ok(list) = serde_json::from_str::<Vec<GameVersion>>(&raw) {
            if !list.is_empty() {
                return Ok(list);
            }
        }
    }
    let list = upload_client(state)?.game_versions().await?;
    let raw = serde_json::to_string(&list)?;
    state
        .store
        .with(|conn| crate::store::metrics::set_meta(conn, "curseforge_game_versions", &raw))?;
    Ok(list)
}

/// Met une version en ligne, sur l'une ou l'autre plateforme, ou les deux.
///
/// Un refus d'un côté n'empêche pas l'autre : chaque plateforme rend son propre
/// compte rendu, et l'appelant voit ce qui est passé et ce qui a échoué.
#[tauri::command]
pub async fn publish_version(
    state: State<'_, AppState>,
    draft: Draft,
    file_path: String,
) -> Result<PublishReport> {
    let (file_name, bytes) = read_file(&file_path)?;
    let mut outcomes = Vec::new();

    if let Some(project_id) = draft.modrinth_project_id.clone() {
        let data = draft.modrinth_data();
        match modrinth_client(&state)?
            .create_version(&data, &file_name, &bytes)
            .await
        {
            Ok((status, body)) if (200..300).contains(&status) => outcomes.push(Outcome {
                platform: "modrinth".into(),
                ok: true,
                id: publish::modrinth_version_id(&body),
                detail: format!("version déposée sur {project_id}"),
            }),
            Ok((status, body)) => outcomes.push(Outcome::refused("modrinth", status, &body)),
            Err(error) => outcomes.push(Outcome {
                platform: "modrinth".into(),
                ok: false,
                id: None,
                detail: error.to_string(),
            }),
        }
    }

    if let Some(project_id) = draft.curseforge_project_id {
        match publish_to_curseforge(&state, &draft, project_id, &file_name, &bytes).await {
            Ok(outcome) => outcomes.push(outcome),
            Err(error) => outcomes.push(Outcome {
                platform: "curseforge".into(),
                ok: false,
                id: None,
                detail: error.to_string(),
            }),
        }
    }

    Ok(PublishReport { outcomes })
}

async fn publish_to_curseforge(
    state: &AppState,
    draft: &Draft,
    project_id: i64,
    file_name: &str,
    bytes: &[u8],
) -> Result<Outcome> {
    // CurseForge ne connaît ses versions que par des nombres : on traduit ce que
    // l'auteur a coché, chargeurs compris, avant de partir.
    let catalogue = game_versions(state).await?;
    let wanted: Vec<String> = draft
        .game_versions
        .iter()
        .chain(draft.loaders.iter())
        .cloned()
        .collect();
    let (ids, missing) = publish::resolve_game_versions(&wanted, &catalogue);
    if ids.is_empty() {
        return Err(AppError::Config(format!(
            "aucune version reconnue par CurseForge parmi : {}",
            wanted.join(", ")
        )));
    }

    let metadata = draft.curseforge_metadata(&ids);
    let (status, body) = upload_client(state)?
        .upload(project_id, &metadata, file_name, bytes)
        .await?;
    if !(200..300).contains(&status) {
        return Ok(Outcome::refused("curseforge", status, &body));
    }
    let note = if missing.is_empty() {
        String::new()
    } else {
        format!(" · non reconnues : {}", missing.join(", "))
    };
    Ok(Outcome {
        platform: "curseforge".into(),
        ok: true,
        id: publish::curseforge_file_id(&body).map(|id| id.to_string()),
        detail: format!("fichier déposé sur le projet {project_id}{note}"),
    })
}

/// Crée un projet Modrinth. CurseForge n'a pas d'équivalent : son premier dépôt
/// se fait sur son site.
#[tauri::command]
pub async fn create_modrinth_project(
    state: State<'_, AppState>,
    slug: String,
    title: String,
    description: String,
    body: String,
    project_type: String,
    categories: Vec<String>,
) -> Result<Outcome> {
    let data = serde_json::json!({
        "slug": slug,
        "title": title,
        "description": description,
        "body": body,
        "project_type": project_type,
        "categories": categories,
        "client_side": "required",
        "server_side": "required",
        "license_id": "MIT",
        "is_draft": true,
        "initial_versions": [],
    })
    .to_string();

    let (status, response) = modrinth_client(&state)?.create_project(&data).await?;
    if !(200..300).contains(&status) {
        return Ok(Outcome::refused("modrinth", status, &response));
    }
    Ok(Outcome {
        platform: "modrinth".into(),
        ok: true,
        id: publish::modrinth_version_id(&response),
        detail: format!("projet {slug} créé en brouillon"),
    })
}

#[tauri::command]
pub async fn delete_modrinth_version(
    state: State<'_, AppState>,
    version_id: String,
) -> Result<Outcome> {
    let (status, body) = modrinth_client(&state)?.delete_version(&version_id).await?;
    Ok(deletion_outcome(
        status,
        &body,
        &format!("version {version_id}"),
    ))
}

#[tauri::command]
pub async fn delete_modrinth_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Outcome> {
    let (status, body) = modrinth_client(&state)?.delete_project(&project_id).await?;
    Ok(deletion_outcome(
        status,
        &body,
        &format!("projet {project_id}"),
    ))
}

fn deletion_outcome(status: u16, body: &str, what: &str) -> Outcome {
    if (200..300).contains(&status) {
        Outcome {
            platform: "modrinth".into(),
            ok: true,
            id: None,
            detail: format!("{what} supprimé"),
        }
    } else {
        Outcome::refused("modrinth", status, body)
    }
}
