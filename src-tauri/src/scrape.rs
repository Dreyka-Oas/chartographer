//! Lecture du solde de points CurseForge depuis la page de l'utilisateur.
//!
//! Aucune interface publique n'expose ce solde : ni l'API REST, ni le jeton de
//! dépôt. Il n'existe que sur le tableau de bord auteur, derrière une session.
//! L'application ouvre donc une fenêtre où l'utilisateur se connecte lui-même —
//! ses identifiants ne passent jamais par notre code — puis relit le texte de la
//! page.
//!
//! Le repérage se fait ici, sur du texte brut, pour être testable sans
//! navigateur. Il reste une heuristique : la valeur trouvée est proposée à
//! l'utilisateur, jamais enregistrée dans son dos.

/// Nombre écrit à l'anglaise ou à la française, séparateurs de milliers compris.
fn parse_number(raw: &str) -> Option<i64> {
    let digits: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    // Un séparateur décimal n'a pas de sens pour un nombre de points : tout ce
    // qui suit le dernier point ou la dernière virgule à deux décimales est
    // ignoré, le reste est du séparateur de milliers.
    let cleaned: String = digits.chars().filter(|c| c.is_ascii_digit()).collect();
    if cleaned.is_empty() || cleaned.len() > 12 {
        return None;
    }
    cleaned.parse().ok()
}

/// Cherche un solde de points dans le texte d'une page.
///
/// Deux tournures couvrent l'essentiel des mises en page : le nombre précède le
/// mot (« 1 240 points ») ou le suit après un séparateur (« Points : 1 240 »).
/// Les mentions du barème (« 0.05 USD per point ») ne portent pas de solde et
/// sont écartées : un solde est entier.
pub fn extract_points(text: &str) -> Option<i64> {
    let lower = text.to_lowercase();
    let bytes = lower.as_bytes();

    for (index, _) in lower.match_indices("point") {
        // Forme « 1 240 points » : on remonte les chiffres avant le mot.
        let before = &lower[..index];
        let trimmed = before.trim_end();
        if trimmed.len() < before.len() || before.ends_with(' ') {
            let start = trimmed
                .rfind(|c: char| !(c.is_ascii_digit() || c == ' ' || c == ',' || c == '.'))
                .map(|i| i + 1)
                .unwrap_or(0);
            let candidate = trimmed[start..].trim();
            if !candidate.is_empty() && candidate.chars().any(|c| c.is_ascii_digit()) {
                // Un barème s'écrit avec des décimales : ce n'est pas un solde.
                let decimal = candidate
                    .rsplit_once(['.', ','])
                    .is_some_and(|(_, tail)| tail.len() < 3 && !tail.is_empty());
                if !decimal {
                    if let Some(value) = parse_number(candidate) {
                        return Some(value);
                    }
                }
            }
        }

        // Forme « Points : 1 240 » : on lit après le mot et son séparateur.
        let after_word = index + "point".len();
        let rest = &lower[after_word.min(bytes.len())..];
        let rest = rest.trim_start_matches(|c: char| {
            c == 's' || c == ':' || c == '-' || c == '=' || c.is_whitespace()
        });
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == ' ' || c == ',' || c == '.'))
            .unwrap_or(rest.len());
        let candidate = rest[..end].trim();
        if !candidate.is_empty() && candidate.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            if let Some(value) = parse_number(candidate) {
                return Some(value);
            }
        }
    }
    None
}

/// Un point d'une série datée trouvée dans une réponse de l'API interne.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DailyPoint {
    pub day: String,
    pub value: i64,
}

/// Reconnaît une date `YYYY-MM-DD`, éventuellement suivie d'une heure.
fn day_from_text(raw: &str) -> Option<String> {
    let head: String = raw.chars().take(10).collect();
    let bytes = head.as_bytes();
    if head.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && head
            .chars()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return Some(head);
    }
    None
}

/// Reconnaît un horodatage Unix, en secondes ou en millisecondes.
fn day_from_epoch(value: i64) -> Option<String> {
    // Bornes larges mais crédibles : 2010 à 2100.
    let seconds = if (1_262_304_000..=4_102_444_800).contains(&value) {
        value
    } else if (1_262_304_000_000..=4_102_444_800_000).contains(&value) {
        value / 1000
    } else {
        return None;
    };
    chrono::DateTime::from_timestamp(seconds, 0).map(|d| d.format("%Y-%m-%d").to_string())
}

fn day_of(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => day_from_text(text),
        serde_json::Value::Number(number) => number.as_i64().and_then(day_from_epoch),
        _ => None,
    }
}

/// Cherche, dans une réponse quelconque, la plus longue série de couples
/// « date, nombre ».
///
/// Le tableau de bord CurseForge n'a pas d'interface documentée : plutôt que de
/// deviner un nom de champ qui changera, on reconnaît la forme d'une série
/// temporelle, quel que soit le nom des clés.
pub fn find_daily_series(value: &serde_json::Value) -> Vec<DailyPoint> {
    let mut best: Vec<DailyPoint> = Vec::new();
    collect_series(value, &mut best);
    best
}

fn collect_series(value: &serde_json::Value, best: &mut Vec<DailyPoint>) {
    match value {
        serde_json::Value::Array(items) => {
            let mut series: Vec<DailyPoint> = Vec::new();
            for item in items {
                if let serde_json::Value::Object(fields) = item {
                    let day = fields.values().find_map(day_of);
                    // Le nombre retenu est le premier entier qui n'est pas la date.
                    let number = fields.iter().find_map(|(_, v)| {
                        let candidate = v.as_i64()?;
                        if day_of(v).is_some() {
                            return None;
                        }
                        Some(candidate)
                    });
                    if let (Some(day), Some(value)) = (day, number) {
                        series.push(DailyPoint { day, value });
                    }
                }
            }
            if series.len() > best.len() {
                *best = series;
            }
            for item in items {
                collect_series(item, best);
            }
        }
        serde_json::Value::Object(fields) => {
            for item in fields.values() {
                collect_series(item, best);
            }
        }
        _ => {}
    }
}

/// Court extrait autour du mot cherché, pour que l'utilisateur vérifie d'un
/// coup d'œil ce que la lecture a retenu.
pub fn excerpt_around(text: &str, needle: &str, radius: usize) -> String {
    let lower = text.to_lowercase();
    let Some(found) = lower.find(&needle.to_lowercase()) else {
        return text.chars().take(radius * 2).collect();
    };
    let start = text[..found]
        .char_indices()
        .rev()
        .nth(radius)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let end = text[found..]
        .char_indices()
        .nth(radius)
        .map(|(i, _)| found + i)
        .unwrap_or(text.len());
    text[start..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(raw: &str) -> serde_json::Value {
        serde_json::from_str(raw).unwrap()
    }

    #[test]
    fn finds_a_series_whatever_the_field_names() {
        let series = find_daily_series(&json(
            r#"[{"date":"2026-08-01","downloads":42},{"date":"2026-08-02","downloads":51}]"#,
        ));
        assert_eq!(series.len(), 2);
        assert_eq!(
            series[0],
            DailyPoint {
                day: "2026-08-01".into(),
                value: 42
            }
        );
        assert_eq!(series[1].value, 51);
    }

    #[test]
    fn digs_through_the_envelope() {
        let series = find_daily_series(&json(
            r#"{"data":{"stats":{"points":[
                {"day":"2026-07-30","count":7},
                {"day":"2026-07-31","count":9},
                {"day":"2026-08-01","count":11}]}}}"#,
        ));
        assert_eq!(series.len(), 3);
        assert_eq!(series[2].value, 11);
    }

    #[test]
    fn reads_unix_timestamps_in_seconds_and_milliseconds() {
        // Même instant exprimé en secondes puis en millisecondes, à un jour d'écart.
        let series = find_daily_series(&json(
            r#"[{"t":1785888000,"v":3},{"t":1785974400000,"v":5}]"#,
        ));
        assert_eq!(series.len(), 2);
        assert_eq!(series[0].day, "2026-08-05");
        assert_eq!(series[1].day, "2026-08-06");
    }

    #[test]
    fn keeps_the_longest_series_of_the_response() {
        let series = find_daily_series(&json(
            r#"{"short":[{"date":"2026-08-01","n":1}],
                "long":[{"date":"2026-08-01","n":1},{"date":"2026-08-02","n":2},
                        {"date":"2026-08-03","n":3}]}"#,
        ));
        assert_eq!(series.len(), 3);
    }

    #[test]
    fn ignores_a_response_without_any_dated_series() {
        assert!(find_daily_series(&json(r#"{"user":"DreykaOas","points":132}"#)).is_empty());
        assert!(find_daily_series(&json("[1, 2, 3]")).is_empty());
    }

    #[test]
    fn excerpt_centres_on_the_word() {
        let page = "En-tête inutile. Solde actuel : 320 points disponibles. Pied de page.";
        let out = excerpt_around(page, "point", 20);
        assert!(out.contains("320 points"), "extrait obtenu : {out}");
    }

    #[test]
    fn excerpt_falls_back_on_the_beginning() {
        // Mot introuvable : on rend le début de la page, sur la largeur demandée.
        assert_eq!(excerpt_around("court texte", "absent", 40), "court texte");
        assert_eq!(excerpt_around("abcdefghij", "absent", 3), "abcdef");
    }

    #[test]
    fn reads_a_balance_written_before_the_word() {
        assert_eq!(extract_points("Your balance: 1 240 points"), Some(1240));
        assert_eq!(extract_points("1,240 Points available"), Some(1240));
    }

    #[test]
    fn reads_a_balance_written_after_the_word() {
        assert_eq!(extract_points("Points: 132"), Some(132));
        assert_eq!(extract_points("Reward points = 4 501"), Some(4501));
    }

    #[test]
    fn ignores_the_rate_and_keeps_looking() {
        // Le barème precede souvent le solde sur la même page.
        let page = "The value of a single point is 0.05 USD. Your balance: 320 points";
        assert_eq!(extract_points(page), Some(320));
    }

    #[test]
    fn returns_nothing_when_the_page_holds_no_balance() {
        assert_eq!(
            extract_points("Connecte-toi pour voir ton tableau de bord"),
            None
        );
        assert_eq!(extract_points(""), None);
    }
}
