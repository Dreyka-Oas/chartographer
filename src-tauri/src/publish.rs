//! Mise en ligne d'une version sur les deux plateformes.
//!
//! Les deux acceptent un envoi en plusieurs parties, mais ne parlent pas la même
//! langue : Modrinth attend un champ `data` en JSON et nomme ses fichiers, tandis
//! que CurseForge attend un champ `metadata` et désigne les versions de jeu par
//! des identifiants numériques qu'il faut d'abord aller chercher.
//!
//! Ce module tient tout ce qui se décide sans réseau : la forme des corps
//! envoyés, la reconnaissance des réponses, et la traduction des versions de jeu
//! en identifiants CurseForge.

use serde::{Deserialize, Serialize};

/// Extensions que les deux plateformes acceptent.
const ACCEPTED: [&str; 4] = ["jar", "zip", "mrpack", "litemod"];

pub fn accepts_extension(file_name: &str) -> bool {
    file_name
        .rsplit_once('.')
        .map(|(_, ext)| ACCEPTED.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Une version prête à partir, décrite une seule fois pour les deux plateformes.
#[derive(Debug, Clone, Deserialize)]
pub struct Draft {
    /// Identifiant Modrinth du projet, absent si l'on ne publie que sur CurseForge.
    pub modrinth_project_id: Option<String>,
    /// Identifiant CurseForge du projet, absent si l'on ne publie que sur Modrinth.
    pub curseforge_project_id: Option<i64>,
    pub name: String,
    pub version_number: String,
    #[serde(default)]
    pub changelog: String,
    /// Versions du jeu, sous la forme lisible : `1.21.1`.
    pub game_versions: Vec<String>,
    /// Chargeurs de mods : `fabric`, `neoforge`…
    pub loaders: Vec<String>,
    /// `release`, `beta` ou `alpha`.
    pub release_type: String,
    /// Retient le fichier côté CurseForge au lieu de le publier tout de suite.
    #[serde(default)]
    pub manual_release: bool,
}

fn sane_release_type(raw: &str) -> &str {
    match raw.to_lowercase().as_str() {
        "beta" => "beta",
        "alpha" => "alpha",
        _ => "release",
    }
}

impl Draft {
    /// Corps `data` attendu par Modrinth, le fichier étant envoyé sous `file`.
    pub fn modrinth_data(&self) -> String {
        let body = serde_json::json!({
            "project_id": self.modrinth_project_id.clone().unwrap_or_default(),
            "name": self.name,
            "version_number": self.version_number,
            "changelog": self.changelog,
            "dependencies": [],
            "game_versions": self.game_versions,
            "loaders": self.loaders,
            "version_type": sane_release_type(&self.release_type),
            "featured": false,
            "file_parts": ["file"],
            "primary_file": "file",
        });
        body.to_string()
    }

    /// Corps `metadata` attendu par CurseForge.
    ///
    /// Le journal des changements y est du texte brut par défaut, et les versions
    /// de jeu comme les chargeurs sont désignés par les identifiants numériques
    /// que le catalogue du site attribue.
    pub fn curseforge_metadata(&self, game_version_ids: &[i64]) -> String {
        serde_json::json!({
            "changelog": self.changelog,
            "changelogType": "text",
            "displayName": self.name,
            "gameVersions": game_version_ids,
            "releaseType": sane_release_type(&self.release_type),
            "isMarkedForManualRelease": self.manual_release,
        })
        .to_string()
    }
}

/// Une entrée du catalogue des versions de jeu CurseForge.
///
/// Le même catalogue porte les versions du jeu et les chargeurs de mods : les
/// deux sont des "versions" à ses yeux, distinguées par leur type.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GameVersion {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default, rename = "gameVersionTypeID")]
    pub type_id: i64,
}

/// Traduit des noms lisibles en identifiants du catalogue.
///
/// La comparaison ignore la casse et accepte le nom comme le raccourci, car
/// CurseForge écrit "Fabric" là où Modrinth écrit "fabric". Les noms sans
/// correspondance sont rendus à part : mieux vaut le dire que publier une
/// version rattachée à la mauvaise cible.
pub fn resolve_game_versions(
    wanted: &[String],
    catalogue: &[GameVersion],
) -> (Vec<i64>, Vec<String>) {
    let mut ids = Vec::new();
    let mut missing = Vec::new();
    for name in wanted {
        let needle = name.trim().to_lowercase();
        let found = catalogue.iter().find(|entry| {
            entry.name.to_lowercase() == needle || entry.slug.to_lowercase() == needle
        });
        match found {
            Some(entry) if !ids.contains(&entry.id) => ids.push(entry.id),
            Some(_) => {}
            None => missing.push(name.clone()),
        }
    }
    (ids, missing)
}

/// Identifiant de la version créée par Modrinth.
pub fn modrinth_version_id(response: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(response)
        .ok()?
        .get("id")?
        .as_str()
        .map(|id| id.to_string())
}

/// Identifiant du fichier accepté par CurseForge.
pub fn curseforge_file_id(response: &str) -> Option<i64> {
    serde_json::from_str::<serde_json::Value>(response)
        .ok()?
        .get("id")?
        .as_i64()
}

/// Message d'erreur lisible tiré d'une réponse refusée, quelle que soit la
/// plateforme : chacune place son explication dans un champ différent.
pub fn refusal_reason(response: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(response).ok()?;
    for key in ["description", "errorMessage", "message", "error"] {
        if let Some(text) = value.get(key).and_then(|v| v.as_str()) {
            return Some(text.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> Draft {
        Draft {
            modrinth_project_id: Some("AABBCCDD".into()),
            curseforge_project_id: Some(1002185),
            name: "Mobs Blocker 1.2.0".into(),
            version_number: "1.2.0".into(),
            changelog: "Corrige le rechargement".into(),
            game_versions: vec!["1.21.1".into()],
            loaders: vec!["fabric".into()],
            release_type: "release".into(),
            manual_release: false,
        }
    }

    #[test]
    fn only_archive_formats_are_accepted() {
        assert!(accepts_extension("mod-1.2.0.jar"));
        assert!(accepts_extension("PACK.MRPACK"));
        assert!(!accepts_extension("notes.txt"));
        assert!(!accepts_extension("sans-extension"));
    }

    #[test]
    fn modrinth_data_names_the_file_part() {
        let raw = draft().modrinth_data();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(value["project_id"], "AABBCCDD");
        assert_eq!(value["version_number"], "1.2.0");
        assert_eq!(value["file_parts"][0], "file");
        assert_eq!(value["primary_file"], "file");
        assert_eq!(value["version_type"], "release");
        assert_eq!(value["game_versions"][0], "1.21.1");
        assert_eq!(value["loaders"][0], "fabric");
    }

    #[test]
    fn an_unknown_release_type_falls_back_on_release() {
        let mut d = draft();
        d.release_type = "n'importe quoi".into();
        let value: serde_json::Value = serde_json::from_str(&d.modrinth_data()).unwrap();
        assert_eq!(value["version_type"], "release");
    }

    #[test]
    fn curseforge_metadata_carries_numeric_versions() {
        let raw = draft().curseforge_metadata(&[9990, 7499]);
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(value["displayName"], "Mobs Blocker 1.2.0");
        assert_eq!(value["gameVersions"][0], 9990);
        assert_eq!(value["gameVersions"][1], 7499);
        assert_eq!(value["changelogType"], "text");
        assert_eq!(value["isMarkedForManualRelease"], false);
    }

    #[test]
    fn a_manual_release_is_held_back() {
        let mut d = draft();
        d.manual_release = true;
        let value: serde_json::Value = serde_json::from_str(&d.curseforge_metadata(&[1])).unwrap();
        assert_eq!(value["isMarkedForManualRelease"], true);
    }

    fn catalogue() -> Vec<GameVersion> {
        vec![
            GameVersion {
                id: 9990,
                name: "1.21.1".into(),
                slug: "1-21-1".into(),
                type_id: 75125,
            },
            GameVersion {
                id: 7499,
                name: "Fabric".into(),
                slug: "fabric".into(),
                type_id: 68441,
            },
        ]
    }

    #[test]
    fn game_versions_and_loaders_share_one_catalogue() {
        let wanted = vec!["1.21.1".to_string(), "fabric".to_string()];
        let (ids, missing) = resolve_game_versions(&wanted, &catalogue());
        assert_eq!(ids, vec![9990, 7499]);
        assert!(missing.is_empty());
    }

    #[test]
    fn an_unknown_version_is_reported_rather_than_dropped_silently() {
        let wanted = vec!["1.21.1".to_string(), "1.99".to_string()];
        let (ids, missing) = resolve_game_versions(&wanted, &catalogue());
        assert_eq!(ids, vec![9990]);
        assert_eq!(missing, vec!["1.99"]);
    }

    #[test]
    fn the_same_version_named_twice_counts_once() {
        let wanted = vec!["1.21.1".to_string(), "1-21-1".to_string()];
        let (ids, _) = resolve_game_versions(&wanted, &catalogue());
        assert_eq!(ids, vec![9990]);
    }

    #[test]
    fn reads_the_identifier_each_platform_returns() {
        assert_eq!(
            modrinth_version_id(r#"{"id":"xR4psbM0","project_id":"AABBCCDD"}"#),
            Some("xR4psbM0".into())
        );
        assert_eq!(curseforge_file_id(r#"{"id":6123456}"#), Some(6123456));
        assert_eq!(modrinth_version_id("pas du json"), None);
    }

    #[test]
    fn reads_the_reason_of_a_refusal() {
        assert_eq!(
            refusal_reason(r#"{"error":"invalid_input","description":"version_number déjà pris"}"#),
            Some("version_number déjà pris".into())
        );
        assert_eq!(
            refusal_reason(r#"{"errorCode":1018,"errorMessage":"Le fichier existe déjà"}"#),
            Some("Le fichier existe déjà".into())
        );
        assert_eq!(refusal_reason("<html>403</html>"), None);
    }
}
