//! Envoi de fichiers sur CurseForge.
//!
//! Rien à voir avec l'interface qui sert les statistiques : celle-ci vit sur le
//! domaine du jeu, s'authentifie par un jeton d'auteur porté en en-tête, et
//! accepte les requêtes faites hors d'un navigateur. C'est la voie qu'empruntent
//! les greffons de compilation depuis des années.
//!
//! Elle ne sait que deux choses : déposer un fichier sur un projet qui existe
//! déjà, et corriger les informations d'un fichier déposé. Créer un projet ou
//! supprimer un fichier ne s'y trouve pas.

use crate::error::{AppError, Result};
use crate::providers::http_client;
use crate::publish::GameVersion;

const PROVIDER: &str = "curseforge-upload";

/// Adresse du site pour Minecraft : l'interface d'envoi est relative au jeu.
const MINECRAFT: &str = "https://minecraft.curseforge.com";

pub struct UploadClient {
    http: reqwest::Client,
    token: String,
}

impl UploadClient {
    pub fn new(token: &str) -> Result<Self> {
        Ok(Self {
            http: http_client()?,
            token: token.to_string(),
        })
    }

    /// Catalogue des versions de jeu et des chargeurs, avec leurs identifiants.
    pub async fn game_versions(&self) -> Result<Vec<GameVersion>> {
        let response = self
            .http
            .get(format!("{MINECRAFT}/api/game/versions"))
            .header("X-Api-Token", &self.token)
            .send()
            .await
            .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
        if !status.is_success() {
            return Err(AppError::remote(
                PROVIDER,
                format!("HTTP {status} sur le catalogue des versions"),
            ));
        }
        serde_json::from_str(&body).map_err(|e| {
            AppError::Data(format!(
                "catalogue des versions illisible : {e} · début : {}",
                body.chars().take(160).collect::<String>()
            ))
        })
    }

    /// Dépose un fichier sur un projet existant.
    pub async fn upload(
        &self,
        project_id: i64,
        metadata: &str,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<(u16, String)> {
        let part = reqwest::multipart::Part::bytes(bytes.to_vec())
            .file_name(file_name.to_string())
            .mime_str("application/java-archive")
            .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
        let form = reqwest::multipart::Form::new()
            .text("metadata", metadata.to_string())
            .part("file", part);
        self.post(
            &format!("{MINECRAFT}/api/projects/{project_id}/upload-file"),
            form,
        )
        .await
    }

    /// Corrige les informations d'un fichier déjà déposé. Le corps doit porter
    /// le `fileID` visé.
    pub async fn update_file(&self, project_id: i64, metadata: &str) -> Result<(u16, String)> {
        let form = reqwest::multipart::Form::new().text("metadata", metadata.to_string());
        self.post(
            &format!("{MINECRAFT}/api/projects/{project_id}/update-file"),
            form,
        )
        .await
    }

    /// Un envoi ne se rejoue pas : une reprise déposerait le fichier deux fois.
    async fn post(&self, url: &str, form: reqwest::multipart::Form) -> Result<(u16, String)> {
        let response = self
            .http
            .post(url)
            .header("X-Api-Token", &self.token)
            .multipart(form)
            .send()
            .await
            .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
        Ok((status, body))
    }
}
