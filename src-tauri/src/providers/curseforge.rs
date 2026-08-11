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

pub fn parse_project(raw: &str) -> Result<CfProject> {
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
    })
}

pub struct CurseForgeClient {
    http: reqwest::Client,
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
