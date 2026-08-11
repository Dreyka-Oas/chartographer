use crate::error::{AppError, Result};
use crate::providers::{http_client, send_with_retry};
use serde::Deserialize;

const WIDGET: &str = "https://api.cfwidget.com";
const PROVIDER: &str = "curseforge";

#[derive(Debug, Clone, Deserialize)]
pub struct CfAuthorProject {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfAuthor {
    pub id: i64,
    pub username: String,
    #[serde(default)]
    pub projects: Vec<CfAuthorProject>,
}

#[derive(Debug, Clone)]
pub struct CfProject {
    pub id: i64,
    pub title: String,
    pub slug: Option<String>,
    pub project_type: Option<String>,
    pub url: Option<String>,
    pub thumbnail: Option<String>,
    pub created_at: Option<String>,
    pub downloads_total: i64,
    pub downloads_monthly: i64,
    pub files: Vec<CfFile>,
}

/// Un fichier publié sur CurseForge.
///
/// CFWidget ne donne aucune série quotidienne, mais il liste les fichiers avec
/// leur date de mise en ligne, leurs versions de jeu, leur chargeur et un
/// compteur de téléchargements. C'est le seul historique disponible côté
/// CurseForge : une répartition par version, datée, et non un suivi jour par
/// jour. Le compteur par fichier est partiel — la somme reste bien inférieure au
/// total du projet — il sert donc à répartir, pas à totaliser.
#[derive(Debug, Clone, PartialEq)]
pub struct CfFile {
    pub id: i64,
    pub display: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub downloads: i64,
    pub uploaded_at: Option<String>,
}

/// Chargeurs connus tels que CurseForge les nomme dans la liste des versions.
const LOADERS: [&str; 6] = ["Forge", "NeoForge", "Fabric", "Quilt", "Rift", "LiteLoader"];

/// Sépare la liste plate de CurseForge en versions de jeu et chargeurs.
///
/// Le même tableau mélange les trois familles : `["1.21.8", "NeoForge",
/// "Server"]`. Les mentions de côté (`Server`, `Client`) ne décrivent ni l'une
/// ni l'autre et sont écartées.
pub fn split_versions(entries: &[String]) -> (Vec<String>, Vec<String>) {
    let mut game = Vec::new();
    let mut loaders = Vec::new();
    for entry in entries {
        if LOADERS.iter().any(|l| l.eq_ignore_ascii_case(entry)) {
            loaders.push(entry.clone());
        } else if entry
            .split('.')
            .next()
            .is_some_and(|head| !head.is_empty() && head.chars().all(|c| c.is_ascii_digit()))
        {
            game.push(entry.clone());
        }
    }
    (game, loaders)
}

pub fn parse_files(raw: &str) -> Vec<CfFile> {
    #[derive(Deserialize)]
    struct RawFile {
        id: i64,
        #[serde(default)]
        display: String,
        #[serde(default)]
        versions: Vec<String>,
        #[serde(default)]
        downloads: i64,
        uploaded_at: Option<String>,
    }
    #[derive(Deserialize)]
    struct Raw {
        #[serde(default)]
        files: Vec<RawFile>,
    }

    let Ok(raw) = serde_json::from_str::<Raw>(raw) else {
        return Vec::new();
    };
    raw.files
        .into_iter()
        .map(|file| {
            let (game_versions, loaders) = split_versions(&file.versions);
            CfFile {
                id: file.id,
                display: file.display,
                game_versions,
                loaders,
                downloads: file.downloads,
                uploaded_at: file.uploaded_at,
            }
        })
        .collect()
}

/// CFWidget répond 202 lorsqu'un rafraîchissement est mis en file d'attente.
#[derive(Debug, Clone)]
pub enum CfFetch {
    Ready(Box<CfProject>),
    Queued,
}

pub fn slug_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim_end_matches('/');
    let last = trimmed.rsplit('/').next()?;
    if last.is_empty() || last.starts_with("http") {
        return None;
    }
    Some(last.to_string())
}

pub fn parse_author(raw: &str) -> Result<CfAuthor> {
    Ok(serde_json::from_str(raw)?)
}

/// Pseudo du propriétaire d'un projet CFWidget.
/// Le membre portant le titre `Owner` prime ; à défaut on prend le premier.
pub fn parse_owner(raw: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Member {
        #[serde(default)]
        title: String,
        username: String,
    }
    #[derive(Deserialize)]
    struct Raw {
        #[serde(default)]
        members: Vec<Member>,
    }

    let raw: Raw = serde_json::from_str(raw).ok()?;
    raw.members
        .iter()
        .find(|m| m.title.eq_ignore_ascii_case("owner"))
        .or_else(|| raw.members.first())
        .map(|m| m.username.clone())
}

pub fn parse_project(raw_body: &str) -> Result<CfProject> {
    let raw = raw_body;
    #[derive(Deserialize)]
    struct Downloads {
        #[serde(default)]
        total: i64,
        #[serde(default)]
        monthly: i64,
    }
    #[derive(Deserialize)]
    struct Urls {
        #[serde(default)]
        curseforge: String,
        #[serde(default)]
        project: String,
    }
    #[derive(Deserialize)]
    struct Raw {
        id: i64,
        title: String,
        #[serde(rename = "type")]
        project_type: Option<String>,
        #[serde(default)]
        urls: Option<Urls>,
        #[serde(default)]
        downloads: Option<Downloads>,
        thumbnail: Option<String>,
        created_at: Option<String>,
    }

    let raw: Raw = serde_json::from_str(raw)?;
    let url = raw
        .urls
        .as_ref()
        .map(|u| {
            if u.curseforge.is_empty() {
                u.project.clone()
            } else {
                u.curseforge.clone()
            }
        })
        .filter(|u| !u.is_empty());
    let downloads = raw.downloads.unwrap_or(Downloads {
        total: 0,
        monthly: 0,
    });

    Ok(CfProject {
        id: raw.id,
        slug: url.as_deref().and_then(slug_from_url),
        title: raw.title,
        project_type: raw.project_type,
        url,
        thumbnail: raw.thumbnail,
        created_at: raw.created_at,
        downloads_total: downloads.total,
        downloads_monthly: downloads.monthly,
        files: parse_files(raw_body),
    })
}

pub struct CurseForgeClient {
    http: reqwest::Client,
}

#[cfg(test)]
mod file_tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "id": 1002185,
      "title": "Mobs Blocker",
      "files": [
        {"id": 7366095, "display": "mobs_blocker-1.2.0-neoforge-1.21.8.jar",
         "versions": ["1.21.8", "NeoForge", "Server"], "downloads": 0,
         "uploaded_at": "2025-12-22T17:18:34.903Z"},
        {"id": 5001, "display": "mobblocker-1.0.0-forge-1.20.1.jar",
         "versions": ["1.20.1", "Forge"], "downloads": 1674,
         "uploaded_at": "2024-04-12T09:35:14.403Z"}
      ]
    }"#;

    #[test]
    fn split_versions_separates_game_versions_from_loaders() {
        let entries = vec![
            "1.21.8".to_string(),
            "NeoForge".to_string(),
            "Server".to_string(),
        ];
        let (game, loaders) = split_versions(&entries);
        assert_eq!(game, vec!["1.21.8"]);
        assert_eq!(loaders, vec!["NeoForge"]);
    }

    #[test]
    fn split_versions_drops_side_labels() {
        let entries = vec!["Client".to_string(), "Fabric".to_string()];
        let (game, loaders) = split_versions(&entries);
        assert!(game.is_empty(), "`Client` n'est pas une version de jeu");
        assert_eq!(loaders, vec!["Fabric"]);
    }

    #[test]
    fn parse_files_reads_the_published_history() {
        let files = parse_files(SAMPLE);
        assert_eq!(files.len(), 2);
        assert_eq!(files[1].downloads, 1674);
        assert_eq!(files[1].game_versions, vec!["1.20.1"]);
        assert_eq!(files[1].loaders, vec!["Forge"]);
        assert_eq!(
            files[1].uploaded_at.as_deref(),
            Some("2024-04-12T09:35:14.403Z")
        );
    }

    #[test]
    fn parse_project_carries_the_files_along() {
        let project = parse_project(SAMPLE).unwrap();
        assert_eq!(project.files.len(), 2);
    }

    #[test]
    fn parse_files_tolerates_a_body_without_files() {
        assert!(parse_files(r#"{"id": 1, "title": "x"}"#).is_empty());
        assert!(parse_files("pas du json").is_empty());
    }
}

impl CurseForgeClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http: http_client()?,
        })
    }

    pub async fn author(&self, username: &str) -> Result<CfAuthor> {
        let url = format!("{WIDGET}/author/search/{username}");
        let response = send_with_retry(PROVIDER, || self.http.get(&url)).await?;
        if response.status().as_u16() == 404 {
            return Err(AppError::Config(format!(
                "pseudo CurseForge introuvable : {username}"
            )));
        }
        let body = response
            .text()
            .await
            .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
        parse_author(&body)
    }

    pub async fn project(&self, id: i64) -> Result<CfFetch> {
        let url = format!("{WIDGET}/{id}");
        let response = send_with_retry(PROVIDER, || self.http.get(&url)).await?;
        if response.status().as_u16() == 202 {
            return Ok(CfFetch::Queued);
        }
        let body = response
            .text()
            .await
            .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
        Ok(CfFetch::Ready(Box::new(parse_project(&body)?)))
    }

    /// Cherche le propriétaire d'un mod CurseForge à partir d'un slug.
    /// Sert à déduire le pseudo auteur sans rien demander à l'utilisateur :
    /// les slugs Modrinth sont réessayés un par un jusqu'à ce que l'un réponde.
    pub async fn owner_of_slug(&self, slug: &str) -> Option<String> {
        let url = format!("{WIDGET}/minecraft/mc-mods/{slug}");
        let response = send_with_retry(PROVIDER, || self.http.get(&url))
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        parse_owner(&response.text().await.ok()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_author_lists_projects() {
        let raw = r#"{"id":108432004,"username":"DreykaOas_official",
            "projects":[{"id":1002185,"name":"Mobs Blocker"},{"id":1412622,"name":"Extended Time"}]}"#;
        let out = parse_author(raw).unwrap();
        assert_eq!(out.username, "DreykaOas_official");
        assert_eq!(out.projects.len(), 2);
        assert_eq!(out.projects[0].id, 1_002_185);
    }

    #[test]
    fn parse_project_reads_downloads_and_url() {
        let raw = r#"{"id":1002185,"title":"Mobs Blocker","type":"Mods",
            "urls":{"curseforge":"https://www.curseforge.com/minecraft/mc-mods/mobblocker",
                    "project":"https://www.curseforge.com/minecraft/mc-mods/mobblocker"},
            "downloads":{"monthly":0,"total":86753},
            "thumbnail":"https://media.forgecdn.net/x.png",
            "created_at":"2024-04-13T11:39:21.023Z"}"#;
        let out = parse_project(raw).unwrap();
        assert_eq!(out.downloads_total, 86_753);
        assert_eq!(out.downloads_monthly, 0);
        assert_eq!(out.slug.as_deref(), Some("mobblocker"));
        assert_eq!(out.title, "Mobs Blocker");
    }

    #[test]
    fn parse_owner_prefers_the_owner_title() {
        let raw = r#"{"members":[{"title":"Contributor","username":"Someone","id":1},
                                 {"title":"Owner","username":"DreykaOas_official","id":108432004}]}"#;
        assert_eq!(parse_owner(raw).as_deref(), Some("DreykaOas_official"));
    }

    #[test]
    fn parse_owner_falls_back_to_the_first_member() {
        let raw = r#"{"members":[{"title":"Maintainer","username":"Someone","id":1}]}"#;
        assert_eq!(parse_owner(raw).as_deref(), Some("Someone"));
    }

    #[test]
    fn parse_owner_returns_none_without_members() {
        assert_eq!(parse_owner(r#"{"members":[]}"#), None);
        assert_eq!(parse_owner("pas du json"), None);
    }

    #[test]
    fn slug_from_url_handles_missing_and_trailing_slash() {
        assert_eq!(
            slug_from_url("https://www.curseforge.com/minecraft/mc-mods/zone-cleaner"),
            Some("zone-cleaner".into())
        );
        assert_eq!(
            slug_from_url("https://www.curseforge.com/minecraft/mc-mods/zone-cleaner/"),
            Some("zone-cleaner".into())
        );
        assert_eq!(slug_from_url(""), None);
    }
}
