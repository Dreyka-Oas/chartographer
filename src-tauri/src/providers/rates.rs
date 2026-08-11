//! Taux de change, pour afficher les revenus dans la devise choisie.
//!
//! Les deux plateformes paient en dollars. Afficher un montant en euros sans le
//! convertir donnerait un chiffre faux : le taux est donc relevé, et sa date
//! affichée avec lui. La source publie les taux de référence de la Banque
//! centrale européenne, sans clé ni compte.

use crate::error::{AppError, Result};
use crate::providers::http_client;

const PROVIDER: &str = "taux de change";
const BASE: &str = "https://api.frankfurter.dev/v1/latest";

/// Un taux relevé, avec le jour auquel il se rapporte.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rate {
    /// Code de la devise cible, en trois lettres.
    pub currency: String,
    /// Combien vaut un dollar dans cette devise.
    pub rate: f64,
    /// Jour du taux, `YYYY-MM-DD`.
    pub day: String,
}

/// Lit la réponse de la source : `{"base":"USD","date":"…","rates":{"EUR":0.865}}`.
pub fn parse_rate(raw: &str, currency: &str) -> Option<Rate> {
    let root: serde_json::Value = serde_json::from_str(raw).ok()?;
    let rate = root["rates"][currency].as_f64()?;
    Some(Rate {
        currency: currency.to_string(),
        rate,
        day: root["date"].as_str().unwrap_or_default().to_string(),
    })
}

/// Relève le taux du dollar vers la devise demandée.
///
/// Le dollar vers lui-même ne demande aucun appel : c'est un.
pub async fn usd_to(currency: &str) -> Result<Rate> {
    let currency = currency.trim().to_uppercase();
    if currency == "USD" {
        return Ok(Rate {
            currency,
            rate: 1.0,
            day: String::new(),
        });
    }
    let url = format!("{BASE}?base=USD&symbols={currency}");
    let response = http_client()?
        .get(&url)
        .send()
        .await
        .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
    if !status.is_success() {
        return Err(AppError::remote(PROVIDER, format!("HTTP {status}")));
    }
    parse_rate(&body, &currency).ok_or_else(|| {
        AppError::Data(format!(
            "aucun taux pour {currency} · réponse : {}",
            body.chars().take(120).collect::<String>()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Réponse réelle de la source, relevée le 11 août 2026.
    const BODY: &str = r#"{"amount":1.0,"base":"USD","date":"2026-08-10",
      "rates":{"EUR":0.86543,"GBP":0.7405}}"#;

    #[test]
    fn reads_the_rate_and_its_day() {
        let rate = parse_rate(BODY, "EUR").unwrap();
        assert_eq!(rate.currency, "EUR");
        assert_eq!(rate.rate, 0.86543);
        assert_eq!(rate.day, "2026-08-10");
    }

    #[test]
    fn a_currency_absent_from_the_answer_yields_nothing() {
        assert!(parse_rate(BODY, "JPY").is_none());
        assert!(parse_rate("pas du json", "EUR").is_none());
    }

    #[tokio::test]
    async fn the_dollar_needs_no_conversion() {
        let rate = usd_to("usd").await.unwrap();
        assert_eq!(rate.rate, 1.0);
        assert_eq!(rate.currency, "USD");
    }
}
