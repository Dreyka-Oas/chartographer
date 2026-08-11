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
