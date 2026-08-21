//! Apprentissage des gestes du tableau de bord CurseForge.
//!
//! Son interface publique d'envoi ne sait que déposer un fichier : ni créer un
//! projet, ni retirer quoi que ce soit. Son tableau de bord, lui, fait les deux
//!, par une interface interne que personne ne documente et dont les corps
//! attendus ne se devinent pas : un envoi incomplet ne récolte qu'une erreur
//! serveur muette.
//!
//! Plutôt que de tâtonner, l'application regarde l'auteur faire le geste une
//! fois, en note la forme exacte, et sait ensuite le refaire seule. Ce module
//! tient la partie qui se décide sans réseau : reconnaître un geste utile,
//! généraliser son adresse, et réécrire son corps pour un autre projet.

use serde::{Deserialize, Serialize};

/// Un appel observé dans la page.
#[derive(Debug, Clone, Deserialize)]
pub struct Observed {
    #[serde(rename = "m")]
    pub method: String,
    #[serde(rename = "u")]
    pub url: String,
    #[serde(rename = "s")]
    pub status: i64,
    #[serde(default, rename = "envoi")]
    pub sent: String,
}

/// Un geste retenu, prêt à être refait.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gesture {
    pub method: String,
    /// Adresse où les identifiants ont laissé la place à des repères.
    pub pattern: String,
    /// Corps envoyé, tel quel. Il sert de patron à la prochaine fois.
    pub body: String,
    /// Ce que le serveur a répondu ce jour-là.
    pub status: i64,
}

/// Vrai si l'appel mérite d'être retenu : une écriture sur l'interface interne,
/// et qui a abouti. Les mesures d'audience et les lectures n'apprennent rien.
pub fn worth_learning(observed: &Observed) -> bool {
    let method = observed.method.to_uppercase();
    if !matches!(method.as_str(), "POST" | "PUT" | "PATCH" | "DELETE") {
        return false;
    }
    if !observed.url.contains("/_api/") {
        return false;
    }
    if observed.url.contains("clarity.ms") || observed.url.contains("analytics") {
        return false;
    }
    (200..300).contains(&observed.status)
}

/// Remplace par un repère chaque segment d'adresse qui n'est qu'un nombre.
///
/// `/_api/project-files/6123456` devient `/_api/project-files/{1}` : la forme
/// survit au changement d'identifiant, qui est justement ce qui varie.
pub fn generalize(url: &str) -> String {
    let (path, query) = match url.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (url, None),
    };
    let mut index = 0;
    let generalized: Vec<String> = path
        .split('/')
        .map(|segment| {
            if !segment.is_empty() && segment.chars().all(|c| c.is_ascii_digit()) {
                index += 1;
                format!("{{{index}}}")
            } else {
                segment.to_string()
            }
        })
        .collect();
    let mut out = generalized.join("/");
    if let Some(query) = query {
        out.push('?');
        out.push_str(query);
    }
    out
}

/// Remet des identifiants à la place des repères, dans l'ordre.
pub fn fill(pattern: &str, values: &[i64]) -> String {
    let mut out = pattern.to_string();
    for (position, value) in values.iter().enumerate() {
        out = out.replace(&format!("{{{}}}", position + 1), &value.to_string());
    }
    out
}

/// Réécrit le corps appris pour un autre projet.
///
/// Seules les clés nommées changent ; tout le reste, les champs obligatoires
/// dont on ignore le rôle, et il y en a, est reconduit tel que le site l'avait
/// envoyé. C'est là tout l'intérêt d'avoir regardé plutôt que deviné.
pub fn adapt_body(body: &str, changes: &[(&str, serde_json::Value)]) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(body) else {
        return body.to_string();
    };
    apply(&mut value, changes);
    value.to_string()
}

fn apply(value: &mut serde_json::Value, changes: &[(&str, serde_json::Value)]) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, replacement) in changes {
                if map.contains_key(*key) {
                    map.insert((*key).to_string(), replacement.clone());
                }
            }
            for (_, nested) in map.iter_mut() {
                apply(nested, changes);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                apply(item, changes);
            }
        }
        _ => {}
    }
}

/// Retient les gestes utiles d'une observation, sans doublon.
///
/// Un même geste refait plusieurs fois n'apprend rien de plus : seule la
/// dernière forme est conservée, car c'est celle qui vaut aujourd'hui.
pub fn learn(observed: &[Observed]) -> Vec<Gesture> {
    let mut out: Vec<Gesture> = Vec::new();
    for entry in observed.iter().filter(|o| worth_learning(o)) {
        let gesture = Gesture {
            method: entry.method.to_uppercase(),
            pattern: generalize(&entry.url),
            body: entry.sent.clone(),
            status: entry.status,
        };
        match out
            .iter_mut()
            .find(|g| g.method == gesture.method && g.pattern == gesture.pattern)
        {
            Some(existing) => *existing = gesture,
            None => out.push(gesture),
        }
    }
    out
}

/// Cherche parmi les gestes appris celui qui crée un projet.
pub fn creation(gestures: &[Gesture]) -> Option<&Gesture> {
    gestures
        .iter()
        .find(|g| g.method == "POST" && g.pattern.ends_with("/_api/projects"))
}

/// Cherche celui qui retire un fichier : une suppression, ou l'appel qui range
/// le fichier hors de la vue quand le site préfère ce détour.
pub fn file_removal(gestures: &[Gesture]) -> Option<&Gesture> {
    gestures
        .iter()
        .find(|g| g.method == "DELETE" && g.pattern.contains("file"))
        .or_else(|| {
            gestures.iter().find(|g| {
                g.pattern.contains("file")
                    && (g.pattern.contains("archive")
                        || g.pattern.contains("delete")
                        || g.method == "PUT")
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed(method: &str, url: &str, status: i64, sent: &str) -> Observed {
        Observed {
            method: method.into(),
            url: url.into(),
            status,
            sent: sent.into(),
        }
    }

    #[test]
    fn only_successful_writes_on_the_internal_interface_are_learned() {
        assert!(worth_learning(&observed(
            "POST",
            "/_api/projects",
            200,
            "{}"
        )));
        assert!(!worth_learning(&observed("GET", "/_api/projects", 200, "")));
        assert!(!worth_learning(&observed(
            "POST",
            "https://o.clarity.ms/collect",
            204,
            ""
        )));
        assert!(
            !worth_learning(&observed("POST", "/_api/projects", 400, "{}")),
            "un geste refusé n'apprend pas comment faire"
        );
    }

    #[test]
    fn identifiers_leave_room_for_markers() {
        assert_eq!(
            generalize("/_api/project-files/6123456"),
            "/_api/project-files/{1}"
        );
        assert_eq!(
            generalize("/_api/projects/1002185/files/6123456"),
            "/_api/projects/{1}/files/{2}"
        );
        assert_eq!(generalize("/_api/projects"), "/_api/projects");
    }

    #[test]
    fn the_query_string_is_left_alone() {
        assert_eq!(
            generalize("/_api/project-files/42?force=true"),
            "/_api/project-files/{1}?force=true"
        );
    }

    #[test]
    fn markers_take_identifiers_back_in_order() {
        assert_eq!(
            fill("/_api/projects/{1}/files/{2}", &[1002185, 6123456]),
            "/_api/projects/1002185/files/6123456"
        );
    }

    #[test]
    fn a_learned_body_is_rewritten_field_by_field() {
        let learned = r#"{"name":"Ancien","gameId":432,"summary":"vieux",
          "nested":{"name":"Ancien"},"licenseTypeId":7}"#;
        let adapted = adapt_body(
            learned,
            &[
                ("name", serde_json::json!("Nouveau")),
                ("summary", serde_json::json!("neuf")),
            ],
        );
        let value: serde_json::Value = serde_json::from_str(&adapted).unwrap();
        assert_eq!(value["name"], "Nouveau");
        assert_eq!(value["summary"], "neuf");
        assert_eq!(value["nested"]["name"], "Nouveau");
        // Ce qu'on ne comprend pas, on le reconduit : c'est tout l'intérêt.
        assert_eq!(value["gameId"], 432);
        assert_eq!(value["licenseTypeId"], 7);
    }

    #[test]
    fn a_body_that_is_not_json_survives_untouched() {
        assert_eq!(adapt_body("pas du json", &[]), "pas du json");
    }

    #[test]
    fn the_same_gesture_repeated_keeps_its_latest_form() {
        let seen = vec![
            observed("POST", "/_api/projects", 200, r#"{"name":"un"}"#),
            observed("POST", "/_api/projects", 200, r#"{"name":"deux"}"#),
        ];
        let learned = learn(&seen);
        assert_eq!(learned.len(), 1);
        assert_eq!(learned[0].body, r#"{"name":"deux"}"#);
    }

    #[test]
    fn creation_and_removal_are_recognised_among_the_rest() {
        let seen = vec![
            observed("POST", "/_api/projects", 201, r#"{"name":"un"}"#),
            observed("PUT", "/_api/projects/description/1002185", 200, "{}"),
            observed("DELETE", "/_api/project-files/6123456", 204, ""),
        ];
        let learned = learn(&seen);
        assert_eq!(creation(&learned).unwrap().method, "POST");
        assert_eq!(
            file_removal(&learned).unwrap().pattern,
            "/_api/project-files/{1}"
        );
    }

    #[test]
    fn nothing_is_invented_when_nothing_was_observed() {
        let learned = learn(&[]);
        assert!(creation(&learned).is_none());
        assert!(file_removal(&learned).is_none());
    }
}
