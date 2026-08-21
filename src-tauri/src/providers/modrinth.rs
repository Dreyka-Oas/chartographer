use crate::error::{AppError, Result};
use crate::providers::{http_client, send_with_retry, ANALYTICS_BATCH};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

const BASE: &str = "https://api.modrinth.com";
const PROVIDER: &str = "modrinth";

pub type SeriesMap = HashMap<String, BTreeMap<i64, i64>>;
pub type RevenueMap = HashMap<String, BTreeMap<i64, Decimal>>;
pub type CountryMap = HashMap<String, HashMap<String, i64>>;

#[derive(Debug, Clone, Deserialize)]
pub struct ModrinthUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub payout_data: PayoutData,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PayoutData {
    #[serde(default)]
    pub balance: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModrinthProject {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub project_type: Option<String>,
    #[serde(default)]
    pub downloads: i64,
    #[serde(default)]
    pub followers: i64,
    pub icon_url: Option<String>,
    pub published: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModrinthVersion {
    pub id: String,
    pub version_number: Option<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub downloads: i64,
    pub date_published: Option<String>,
}

/// Solde de reversement Modrinth. `available` est retirable immédiatement,
/// `pending` mûrit encore, et `dates` est l'échéancier mensuel, les entrées
/// postérieures à aujourd'hui sont donc des revenus à venir.
#[derive(Debug, Clone, Default, Deserialize, serde::Serialize)]
pub struct PayoutBalance {
    #[serde(default)]
    pub available: String,
    #[serde(default)]
    pub pending: String,
    #[serde(default)]
    pub withdrawn_lifetime: String,
    #[serde(default)]
    pub withdrawn_ytd: String,
    #[serde(default)]
    pub dates: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub occurred_at: String,
    pub kind: String,
    pub project_ext_id: Option<String>,
    pub detail: String,
}

pub fn timestamp_to_day(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

pub fn parse_series(raw: &str) -> Result<SeriesMap> {
    let parsed: HashMap<String, HashMap<String, i64>> = serde_json::from_str(raw)?;
    Ok(parsed
        .into_iter()
        .map(|(project, points)| {
            let series = points
                .into_iter()
                .filter_map(|(ts, value)| ts.parse::<i64>().ok().map(|t| (t, value)))
                .collect();
            (project, series)
        })
        .collect())
}

pub fn parse_revenue(raw: &str) -> Result<RevenueMap> {
    let parsed: HashMap<String, HashMap<String, String>> = serde_json::from_str(raw)?;
    Ok(parsed
        .into_iter()
        .map(|(project, points)| {
            let series = points
                .into_iter()
                .filter_map(|(ts, value)| {
                    let ts = ts.parse::<i64>().ok()?;
                    let amount = Decimal::from_str(&value).ok()?;
                    Some((ts, amount))
                })
                .collect();
            (project, series)
        })
        .collect())
}

pub fn parse_countries(raw: &str) -> Result<CountryMap> {
    Ok(serde_json::from_str(raw)?)
}

pub fn parse_projects(raw: &str) -> Result<Vec<ModrinthProject>> {
    Ok(serde_json::from_str(raw)?)
}

pub fn parse_versions(raw: &str) -> Result<Vec<ModrinthVersion>> {
    Ok(serde_json::from_str(raw)?)
}

pub fn parse_payout_balance(raw: &str) -> Result<PayoutBalance> {
    Ok(serde_json::from_str(raw)?)
}

pub fn parse_notifications(raw: &str) -> Result<Vec<Notification>> {
    #[derive(Deserialize)]
    struct Raw {
        created: String,
        body: serde_json::Value,
    }
    let rows: Vec<Raw> = serde_json::from_str(raw)?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let kind = r
                .body
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let project_ext_id = r
                .body
                .get("project_id")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let detail = match (r.body.get("old_status"), r.body.get("new_status")) {
                (Some(old), Some(new)) => format!(
                    "{} vers {}",
                    old.as_str().unwrap_or_default(),
                    new.as_str().unwrap_or_default()
                ),
                _ => r.body.to_string(),
            };
            Notification {
                occurred_at: r.created,
                kind,
                project_ext_id,
                detail,
            }
        })
        .collect())
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub struct ModrinthClient {
    http: reqwest::Client,
    token: String,
}

impl ModrinthClient {
    pub fn new(token: &str) -> Result<Self> {
        Ok(Self {
            http: http_client()?,
            token: token.to_string(),
        })
    }

    async fn get_text(&self, url: &str) -> Result<String> {
        let response = send_with_retry(PROVIDER, || {
            self.http.get(url).header("Authorization", &self.token)
        })
        .await?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
        if !status.is_success() {
            return Err(AppError::remote(
                PROVIDER,
                format!("HTTP {status} sur {url}"),
            ));
        }
        Ok(body)
    }

    /// Envoi en plusieurs parties, pour créer une version ou un projet.
    ///
    /// La réponse est rendue telle quelle, corps et état : l'appelant sait mieux
    /// que ce client ce qu'un refus signifie pour lui. Une seule tentative,
    /// contrairement aux lectures, un envoi rejoué déposerait deux fois le même
    /// fichier, et le refus se lit dans le corps plutôt que dans une erreur
    /// d'authentification muette.
    async fn post_multipart(
        &self,
        url: &str,
        form: reqwest::multipart::Form,
    ) -> Result<(u16, String)> {
        let response = self
            .http
            .post(url)
            .header("Authorization", &self.token)
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

    /// Crée une version sur un projet existant. Le fichier voyage sous le nom
    /// `file`, celui que le corps `data` annonce dans `file_parts`.
    pub async fn create_version(
        &self,
        data: &str,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<(u16, String)> {
        let part = reqwest::multipart::Part::bytes(bytes.to_vec())
            .file_name(file_name.to_string())
            .mime_str("application/java-archive")
            .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
        let form = reqwest::multipart::Form::new()
            .text("data", data.to_string())
            .part("file", part);
        self.post_multipart(&format!("{BASE}/v2/version"), form)
            .await
    }

    /// Crée un projet. L'icône est facultative.
    pub async fn create_project(&self, data: &str) -> Result<(u16, String)> {
        let form = reqwest::multipart::Form::new().text("data", data.to_string());
        self.post_multipart(&format!("{BASE}/v2/project"), form)
            .await
    }

    /// Supprime une version. Rend l'état et le corps, vide en cas de succès.
    pub async fn delete_version(&self, version_id: &str) -> Result<(u16, String)> {
        self.delete(&format!("{BASE}/v2/version/{version_id}"))
            .await
    }

    /// Supprime un projet, avec tout ce qu'il contient.
    pub async fn delete_project(&self, project_id: &str) -> Result<(u16, String)> {
        self.delete(&format!("{BASE}/v2/project/{project_id}"))
            .await
    }

    async fn delete(&self, url: &str) -> Result<(u16, String)> {
        let response = self
            .http
            .delete(url)
            .header("Authorization", &self.token)
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

    pub async fn me(&self) -> Result<ModrinthUser> {
        let body = self.get_text(&format!("{BASE}/v2/user")).await?;
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn projects(&self, user_id: &str) -> Result<Vec<ModrinthProject>> {
        let body = self
            .get_text(&format!("{BASE}/v2/user/{user_id}/projects"))
            .await?;
        parse_projects(&body)
    }

    pub async fn versions(&self, project_id: &str) -> Result<Vec<ModrinthVersion>> {
        let body = self
            .get_text(&format!("{BASE}/v2/project/{project_id}/version"))
            .await?;
        parse_versions(&body)
    }

    pub async fn payout_balance(&self) -> Result<PayoutBalance> {
        let body = self.get_text(&format!("{BASE}/v3/payout/balance")).await?;
        parse_payout_balance(&body)
    }

    pub async fn notifications(&self, user_id: &str) -> Result<Vec<Notification>> {
        let body = self
            .get_text(&format!("{BASE}/v2/user/{user_id}/notifications"))
            .await?;
        parse_notifications(&body)
    }

    fn analytics_url(
        &self,
        path: &str,
        ids: &[String],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> String {
        let ids_json = serde_json::to_string(ids).unwrap_or_else(|_| "[]".into());
        format!(
            "{BASE}/v3/analytics/{path}?project_ids={}&start_date={}&end_date={}&resolution_minutes=1440",
            urlencode(&ids_json),
            urlencode(&start.to_rfc3339()),
            urlencode(&end.to_rfc3339())
        )
    }

    pub async fn analytics_downloads(
        &self,
        ids: &[String],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<SeriesMap> {
        let mut merged = SeriesMap::new();
        for batch in crate::providers::chunk_ids(ids, ANALYTICS_BATCH) {
            let body = self
                .get_text(&self.analytics_url("downloads", &batch, start, end))
                .await?;
            merged.extend(parse_series(&body)?);
        }
        Ok(merged)
    }

    pub async fn analytics_views(
        &self,
        ids: &[String],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<SeriesMap> {
        let mut merged = SeriesMap::new();
        for batch in crate::providers::chunk_ids(ids, ANALYTICS_BATCH) {
            let body = self
                .get_text(&self.analytics_url("views", &batch, start, end))
                .await?;
            merged.extend(parse_series(&body)?);
        }
        Ok(merged)
    }

    pub async fn analytics_revenue(
        &self,
        ids: &[String],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<RevenueMap> {
        let mut merged = RevenueMap::new();
        for batch in crate::providers::chunk_ids(ids, ANALYTICS_BATCH) {
            let body = self
                .get_text(&self.analytics_url("revenue", &batch, start, end))
                .await?;
            merged.extend(parse_revenue(&body)?);
        }
        Ok(merged)
    }

    pub async fn analytics_countries(
        &self,
        ids: &[String],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<CountryMap> {
        let mut merged = CountryMap::new();
        for batch in crate::providers::chunk_ids(ids, ANALYTICS_BATCH) {
            let body = self
                .get_text(&self.analytics_url("countries/downloads", &batch, start, end))
                .await?;
            merged.extend(parse_countries(&body)?);
        }
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_series_reads_string_timestamps() {
        let raw =
            r#"{"6P28kqbu":{"1784073600":838,"1783641600":863},"W0JWCVNo":{"1784073600":14}}"#;
        let out = parse_series(raw).unwrap();
        assert_eq!(out["6P28kqbu"][&1_783_641_600], 863);
        assert_eq!(out["6P28kqbu"][&1_784_073_600], 838);
        assert_eq!(out["W0JWCVNo"].len(), 1);
    }

    #[test]
    fn parse_revenue_keeps_full_decimal_precision() {
        let raw = r#"{"W0JWCVNo":{"1785888000":"0.00762273691987854525"}}"#;
        let out = parse_revenue(raw).unwrap();
        let value = out["W0JWCVNo"][&1_785_888_000];
        assert_eq!(value.to_string(), "0.00762273691987854525");
    }

    #[test]
    fn parse_countries_keeps_special_keys_apart() {
        let raw = r#"{"W0JWCVNo":{"DE":88,"XX":558,"":454,"US":456}}"#;
        let out = parse_countries(raw).unwrap();
        assert_eq!(out["W0JWCVNo"]["DE"], 88);
        assert_eq!(out["W0JWCVNo"]["XX"], 558);
        assert_eq!(out["W0JWCVNo"][""], 454);
        assert_eq!(out["W0JWCVNo"].len(), 4);
    }

    #[test]
    fn timestamp_to_day_is_utc() {
        assert_eq!(timestamp_to_day(1_784_073_600), "2026-07-15");
    }

    #[test]
    fn parse_projects_maps_all_fields() {
        let raw = r#"[{"id":"6P28kqbu","slug":"vein-vantage","title":"Vein Vantage",
            "project_type":"mod","downloads":176968,"followers":6,
            "icon_url":"https://cdn.modrinth.com/x.png","published":"2024-06-01T10:00:00Z"}]"#;
        let out = parse_projects(raw).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].slug, "vein-vantage");
        assert_eq!(out[0].downloads, 176_968);
        assert_eq!(out[0].followers, 6);
    }

    #[test]
    fn parse_payout_balance_keeps_precision_and_schedule() {
        let raw = r#"{"available":"12.6251208063540247","pending":"4.84400277701479185061",
            "withdrawn_lifetime":"70.4200000000000000","withdrawn_ytd":"15.3600000000000000",
            "dates":{"2026-10-30T00:00:00Z":"0.57035192308093763305",
                     "2026-08-29T00:00:00Z":"2.54944831105631864138"}}"#;
        let out = parse_payout_balance(raw).unwrap();
        assert_eq!(out.available, "12.6251208063540247");
        assert_eq!(out.pending, "4.84400277701479185061");
        assert_eq!(out.dates.len(), 2);
        assert_eq!(
            out.dates.keys().next().map(String::as_str),
            Some("2026-08-29T00:00:00Z"),
            "l'echeancier doit rester trie par date"
        );
    }

    #[test]
    fn parse_payout_balance_tolerates_missing_fields() {
        let out = parse_payout_balance("{}").unwrap();
        assert!(out.available.is_empty());
        assert!(out.dates.is_empty());
    }

    #[test]
    fn parse_notifications_renders_status_change() {
        let raw = r#"[{"id":"OT8","read":false,"created":"2026-03-15T21:51:39.314925Z",
            "body":{"type":"status_change","project_id":"YCu7AAOD",
                    "old_status":"processing","new_status":"approved"}}]"#;
        let out = parse_notifications(raw).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, "status_change");
        assert_eq!(out[0].project_ext_id.as_deref(), Some("YCu7AAOD"));
        assert!(out[0].detail.contains("processing"));
        assert!(out[0].detail.contains("approved"));
    }
}
