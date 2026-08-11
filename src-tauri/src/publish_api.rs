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

/// Page où CurseForge liste les jetons d'envoi de l'auteur. Le tableau de bord
/// actuel ne les montre plus : ils sont restés sur l'ancien site, où la session
/// vaut aussi.
const TOKEN_PAGE: &str = "https://legacy.curseforge.com/account/api-tokens";

/// Relève les jetons affichés sur la page du compte.
///
/// Ils y figurent en clair, dans le tableau ; d'autres identifiants de même
/// forme traînent parfois ailleurs dans la page, d'où la vérification qui suit.
const READ_TOKENS: &str = r#"(function () {
  var found = [];
  var pattern = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i;
  document.querySelectorAll('td, code, pre, input').forEach(function (node) {
    var text = (node.innerText || node.value || '').trim();
    var hit = text.match(pattern);
    if (hit && found.indexOf(hit[0]) < 0) found.push(hit[0]);
  });
  if (found.length === 0) {
    var all = (document.documentElement ? document.documentElement.innerHTML : '')
      .match(new RegExp(pattern.source, 'gi')) || [];
    all.forEach(function (t) { if (found.indexOf(t) < 0) found.push(t); });
  }
  return JSON.stringify(found);
})()"#;

/// Demande un nouveau jeton quand le compte n'en montre aucun.
const GENERATE_TOKEN: &str = r#"(function () {
  var nodes = document.querySelectorAll('button, a, input[type=submit]');
  for (var i = 0; i < nodes.length; i++) {
    var label = (nodes[i].innerText || nodes[i].value || '').trim();
    if (/générer un jeton|generate token|generate a token/i.test(label)) {
      nodes[i].click();
      return 'demandé';
    }
  }
  return 'bouton introuvable';
})()"#;

/// Relève le jeton d'envoi CurseForge sans rien demander à l'utilisateur.
///
/// La page du compte l'affiche en clair : l'application la lit avec la session
/// déjà ouverte, essaie chaque jeton trouvé contre l'interface d'envoi, et garde
/// celui qui répond. Aucun n'est affiché nulle part.
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

    let go = format!("(function () {{ location.href = {TOKEN_PAGE:?}; return 'ok'; }})()");
    crate::commands::eval_in_window(app, &go).await?;
    crate::commands::wait_until_loaded(&window).await;

    let mut candidates = read_tokens(app).await?;
    if candidates.is_empty() {
        crate::commands::eval_in_window(app, GENERATE_TOKEN).await?;
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        crate::commands::wait_until_loaded(&window).await;
        candidates = read_tokens(app).await?;
    }

    let mut kept = None;
    for candidate in candidates {
        if let Ok(client) = UploadClient::new(&candidate) {
            if client.game_versions().await.is_ok() {
                kept = Some(candidate);
                break;
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
    let go_back = format!("(function () {{ location.href = {target:?}; return 'ok'; }})()");
    let _ = crate::commands::eval_in_window(app, &go_back).await;

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

async fn read_tokens(app: &tauri::AppHandle) -> Result<Vec<String>> {
    let raw = crate::commands::eval_in_window(app, READ_TOKENS).await?;
    let unquoted: String = serde_json::from_str(&raw).unwrap_or(raw);
    Ok(serde_json::from_str(&unquoted).unwrap_or_default())
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
