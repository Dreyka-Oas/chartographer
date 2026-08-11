//! Collecte automatique des statistiques du tableau de bord CurseForge.
//!
//! Le site est protégé par un filtre anti-robot : toute requête faite hors d'un
//! navigateur reçoit un 403, quel que soit le jeton. Aucune interface publique
//! n'expose non plus ces chiffres. La seule voie est donc une fenêtre de
//! navigateur, celle de l'application, dans laquelle l'utilisateur se connecte
//! une fois. Ensuite, tout se fait sans lui : l'application parcourt ses propres
//! pages, écoute les réponses qu'elles reçoivent et en tire les séries.
//!
//! Ce module tient les parties raisonnables et testables : reconnaître à quel
//! projet appartient une réponse, et repérer les pages du tableau de bord qui
//! valent le détour.

/// Cherche, dans une adresse ou un corps de réponse, l'identifiant d'un des
/// projets connus.
///
/// Les identifiants CurseForge sont des nombres longs et distinctifs : les
/// trouver dans une réponse suffit à savoir qu'elle la concerne. On exige une
/// frontière autour du nombre pour ne pas confondre `1002185` avec `31002185`.
pub fn pick_project_id(url: &str, body: &str, known: &[(i64, String)]) -> Option<i64> {
    let haystacks = [url, body];
    for (id, ext_id) in known {
        for hay in haystacks {
            if contains_standalone_number(hay, ext_id) {
                return Some(*id);
            }
        }
    }
    None
}

fn contains_standalone_number(hay: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let bytes = hay.as_bytes();
    let mut start = 0usize;
    while let Some(found) = hay[start..].find(needle) {
        let at = start + found;
        let before_ok = at == 0 || !bytes[at - 1].is_ascii_digit();
        let after = at + needle.len();
        let after_ok = after >= bytes.len() || !bytes[after].is_ascii_digit();
        if before_ok && after_ok {
            return true;
        }
        start = at + 1;
        if start >= hay.len() {
            break;
        }
    }
    false
}

/// Une mesure quotidienne rattachée à un projet CurseForge.
#[derive(Debug, Clone, PartialEq)]
pub struct DailyDownload {
    /// Identifiant externe CurseForge, ou nom normalisé si l'un manque.
    pub key: String,
    pub day: String,
    pub downloads: i64,
}

fn day_from_epoch_ms(value: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(value / 1000, 0).map(|d| d.format("%Y-%m-%d").to_string())
}

/// Lit la réponse `statistics/queries/downloads` du tableau de bord.
///
/// Chaque entrée porte la date en millisecondes et une colonne par projet,
/// nommée soit par le titre du mod en minuscules, soit `project-<id>` quand le
/// titre manque. La colonne `downloads` n'est pas un total mais la première
/// série : elle est ignorée.
pub fn parse_downloads_query(raw: &str) -> Vec<DailyDownload> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    let Some(rows) = root["queryResult"]["data"].as_array() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for row in rows {
        let Some(fields) = row.as_object() else {
            continue;
        };
        let Some(day) = fields
            .get("downloadDate")
            .and_then(|v| v.as_i64())
            .and_then(day_from_epoch_ms)
        else {
            continue;
        };
        for (name, value) in fields {
            if name == "downloadDate" || name == "downloads" || name == "undefined" {
                continue;
            }
            let Some(downloads) = value.as_i64() else {
                continue;
            };
            out.push(DailyDownload {
                key: name.trim().to_lowercase(),
                day: day.clone(),
                downloads,
            });
        }
    }
    out
}

/// Rapproche une colonne de la réponse d'un projet connu.
///
/// La colonne porte soit `project-<identifiant>`, soit le titre du mod en
/// minuscules : les deux mènent au même projet.
pub fn match_column(column: &str, known: &[(i64, String, String)]) -> Option<i64> {
    let column = column.trim().to_lowercase();
    if let Some(ext_id) = column.strip_prefix("project-") {
        return known
            .iter()
            .find(|(_, known_ext, _)| known_ext == ext_id)
            .map(|(id, _, _)| *id);
    }
    known
        .iter()
        .find(|(_, _, title)| title.to_lowercase() == column)
        .map(|(id, _, _)| *id)
}

/// Vrai si l'adresse et le texte montrent une page de connexion.
///
/// Constaté sur place : le tableau de bord renvoie vers `sso.curseforge.com`,
/// une page d'identification déléguée qui ne propose que des fournisseurs
/// tiers — Google, Discord, GitHub, Twitch, WeChat. Son adresse ne contient ni
/// « login » ni « signin », d'où cette reconnaissance explicite.
pub fn is_login_page(url: &str, text: &str) -> bool {
    let url = url.to_lowercase();
    if url.contains("sso.curseforge.com")
        || url.contains("/oidc/")
        || url.contains("/interaction/")
        || url.contains("/login")
        || url.contains("/signin")
    {
        return true;
    }
    let text = text.to_lowercase();
    text.contains("log in with") || text.contains("welcome back to curseforge")
}

/// Retient les adresses du tableau de bord qui méritent d'être visitées.
///
/// Le tableau de bord change de forme au gré des refontes : plutôt que de coder
/// des adresses en dur, on part des liens de la page et on garde ceux qui
/// restent sur le domaine et parlent de projets ou de statistiques.
pub fn worth_visiting(links: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for link in links {
        if !link.starts_with("https://authors.curseforge.com/") {
            continue;
        }
        let lower = link.to_lowercase();
        let interesting = ["project", "analytic", "statistic", "dashboard", "insight"]
            .iter()
            .any(|word| lower.contains(word));
        // Les pages d'édition et de fichiers ne portent pas de série.
        let noisy = ["/edit", "/files", "/upload", "/settings", "/logout"]
            .iter()
            .any(|word| lower.contains(word));
        if interesting && !noisy && !out.contains(link) {
            out.push(link.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known() -> Vec<(i64, String)> {
        vec![(7, "1002185".into()), (9, "444333".into())]
    }

    #[test]
    fn recognises_a_project_from_the_address() {
        let id = pick_project_id(
            "https://authors.curseforge.com/dashboard/project/1002185/analytics",
            "{}",
            &known(),
        );
        assert_eq!(id, Some(7));
    }

    #[test]
    fn recognises_a_project_from_the_body() {
        let id = pick_project_id("https://x/y", r#"{"projectId":444333}"#, &known());
        assert_eq!(id, Some(9));
    }

    #[test]
    fn refuses_a_number_that_merely_contains_the_identifier() {
        assert_eq!(
            pick_project_id("https://x/31002185", r#"{"id":91002185}"#, &known()),
            None,
            "31002185 n'est pas 1002185"
        );
    }

    /// Réponse réelle du tableau de bord, relevée sur le compte de l'auteur.
    const DOWNLOADS: &str = r#"{"id":6390,"queryResult":{"data":[
      {"downloadDate":1778630400000,"downloads":152,"custom clear lag":3,
       "mobs blocker":152,"project-1007955":0,"vein vantage":13},
      {"downloadDate":1778716800000,"downloads":154,"custom clear lag":0,
       "mobs blocker":154,"project-1007955":2,"vein vantage":9}
    ]}}"#;

    #[test]
    fn reads_the_daily_downloads_of_each_mod() {
        let series = parse_downloads_query(DOWNLOADS);
        let blocker: Vec<&DailyDownload> =
            series.iter().filter(|d| d.key == "mobs blocker").collect();
        assert_eq!(blocker.len(), 2);
        assert_eq!(blocker[0].day, "2026-05-13");
        assert_eq!(blocker[0].downloads, 152);
        assert_eq!(blocker[1].day, "2026-05-14");
        assert_eq!(blocker[1].downloads, 154);
    }

    #[test]
    fn ignores_the_leading_total_column() {
        let series = parse_downloads_query(DOWNLOADS);
        assert!(
            !series.iter().any(|d| d.key == "downloads"),
            "la colonne downloads double la première série, elle n'est pas un total"
        );
    }

    #[test]
    fn matches_a_column_by_title_or_by_identifier() {
        let known = vec![
            (7, "1002185".to_string(), "Mobs Blocker".to_string()),
            (11, "1007955".to_string(), "No Name".to_string()),
        ];
        assert_eq!(match_column("mobs blocker", &known), Some(7));
        assert_eq!(match_column("project-1007955", &known), Some(11));
        assert_eq!(match_column("inconnu", &known), None);
    }

    #[test]
    fn tolerates_a_response_of_another_shape() {
        assert!(parse_downloads_query("{}").is_empty());
        assert!(parse_downloads_query("pas du json").is_empty());
    }

    #[test]
    fn recognises_the_delegated_login_page() {
        // Adresse relevée sur place lors du diagnostic.
        assert!(is_login_page(
            "https://sso.curseforge.com/oidc/interaction/nYfFS6kb-EW-QRdF85yqs",
            "Welcome back to CurseForge Log in with Google"
        ));
        assert!(is_login_page(
            "https://authors.curseforge.com/",
            "Welcome back to CurseForge"
        ));
    }

    #[test]
    fn does_not_mistake_the_dashboard_for_a_login_page() {
        assert!(!is_login_page(
            "https://authors.curseforge.com/dashboard/projects",
            "Projects Downloads Analytics"
        ));
    }

    #[test]
    fn keeps_only_dashboard_pages_that_may_carry_series() {
        let links = vec![
            "https://authors.curseforge.com/dashboard/projects".to_string(),
            "https://authors.curseforge.com/dashboard/project/1002185/files".to_string(),
            "https://www.curseforge.com/minecraft/mc-mods/mobblocker".to_string(),
            "https://authors.curseforge.com/dashboard/analytics".to_string(),
        ];
        let kept = worth_visiting(&links);
        assert_eq!(
            kept,
            vec![
                "https://authors.curseforge.com/dashboard/projects",
                "https://authors.curseforge.com/dashboard/analytics"
            ]
        );
    }

    #[test]
    fn ignores_duplicates() {
        let links = vec![
            "https://authors.curseforge.com/dashboard/projects".to_string(),
            "https://authors.curseforge.com/dashboard/projects".to_string(),
        ];
        assert_eq!(worth_visiting(&links).len(), 1);
    }
}
