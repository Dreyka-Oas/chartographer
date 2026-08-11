use std::collections::HashSet;

pub const FUZZY_THRESHOLD: f64 = 0.88;

#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: i64,
    pub slug: Option<String>,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub modrinth_id: i64,
    pub curseforge_id: i64,
    pub confidence: f64,
}

pub fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Apparie chaque projet Modrinth avec au plus un projet CurseForge.
/// Slug exact, puis titre exact, puis Jaro-Winkler sans ambiguïté.
pub fn match_projects(modrinth: &[Candidate], curseforge: &[Candidate]) -> Vec<Match> {
    let mut claimed: HashSet<i64> = HashSet::new();
    let mut out: Vec<Match> = Vec::new();

    for pass in 0..3 {
        for m in modrinth {
            if out.iter().any(|x| x.modrinth_id == m.id) {
                continue;
            }
            let found = match pass {
                0 => exact(m, curseforge, &claimed, |c| {
                    c.slug.as_deref().unwrap_or_default()
                }),
                1 => exact(m, curseforge, &claimed, |c| c.title.as_str()),
                _ => fuzzy(m, curseforge, &claimed),
            };
            if let Some((cf_id, confidence)) = found {
                claimed.insert(cf_id);
                out.push(Match {
                    modrinth_id: m.id,
                    curseforge_id: cf_id,
                    confidence,
                });
            }
        }
    }
    out
}

fn exact<F>(
    m: &Candidate,
    pool: &[Candidate],
    claimed: &HashSet<i64>,
    field: F,
) -> Option<(i64, f64)>
where
    F: Fn(&Candidate) -> &str,
{
    let needle = normalize(field(m));
    if needle.is_empty() {
        return None;
    }
    pool.iter()
        .find(|c| !claimed.contains(&c.id) && normalize(field(c)) == needle)
        .map(|c| (c.id, 1.0))
}

fn fuzzy(m: &Candidate, pool: &[Candidate], claimed: &HashSet<i64>) -> Option<(i64, f64)> {
    let needle = normalize(&m.title);
    let mut hits: Vec<(i64, f64)> = pool
        .iter()
        .filter(|c| !claimed.contains(&c.id))
        .map(|c| (c.id, strsim::jaro_winkler(&needle, &normalize(&c.title))))
        .filter(|(_, score)| *score >= FUZZY_THRESHOLD)
        .collect();

    if hits.len() != 1 {
        return None;
    }
    let (id, score) = hits.pop().unwrap();
    Some((id, score.min(0.999)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(id: i64, slug: &str, title: &str) -> Candidate {
        Candidate {
            id,
            slug: Some(slug.into()),
            title: title.into(),
        }
    }

    #[test]
    fn normalize_strips_separators_and_case() {
        assert_eq!(normalize("Custom Clear Lag"), "customclearlag");
        assert_eq!(normalize("no-night-skip"), "nonightskip");
        assert_eq!(normalize("Vein_Vantage"), "veinvantage");
    }

    #[test]
    fn exact_slug_wins() {
        let m = vec![c(1, "vein-vantage", "Vein Vantage")];
        let cf = vec![c(10, "vein-vantage", "Something Else")];
        let out = match_projects(&m, &cf);
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].modrinth_id, out[0].curseforge_id), (1, 10));
        assert_eq!(out[0].confidence, 1.0);
    }

    #[test]
    fn exact_title_matches_when_slugs_differ() {
        let m = vec![c(1, "mobsblocker", "Mobs Blocker")];
        let cf = vec![c(10, "mobblocker", "Mobs Blocker")];
        let out = match_projects(&m, &cf);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].confidence, 1.0);
    }

    #[test]
    fn fuzzy_title_matches_colony_project() {
        let m = vec![c(1, "colony", "Colony")];
        let cf = vec![c(10, "colony-project", "Colony Project")];
        let out = match_projects(&m, &cf);
        assert_eq!(out.len(), 1);
        assert!(out[0].confidence >= 0.88 && out[0].confidence < 1.0);
    }

    #[test]
    fn ambiguous_fuzzy_produces_no_match() {
        let m = vec![c(1, "colony", "Colony")];
        let cf = vec![c(10, "colonies", "Colonies"), c(11, "colonyx", "Colony X")];
        assert!(match_projects(&m, &cf).is_empty());
    }

    #[test]
    fn unrelated_projects_do_not_match() {
        let m = vec![c(1, "fake-fps", "Fake FPS")];
        let cf = vec![c(10, "zone-cleaner", "Zone Cleaner")];
        assert!(match_projects(&m, &cf).is_empty());
    }

    #[test]
    fn each_curseforge_project_is_claimed_once() {
        let m = vec![
            c(1, "health-tag", "Health Tag"),
            c(2, "healthtag", "Health Tag"),
        ];
        let cf = vec![c(10, "health-tag", "Health Tag")];
        assert_eq!(match_projects(&m, &cf).len(), 1);
    }
}
