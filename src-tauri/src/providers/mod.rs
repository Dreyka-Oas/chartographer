pub mod curseforge;
pub mod modrinth;

use crate::error::{AppError, Result};

pub const USER_AGENT: &str = concat!("Dreyka-Oas/chartographer/", env!("CARGO_PKG_VERSION"));
pub const MAX_RETRIES: u32 = 3;
pub const MAX_BACKOFF_MS: u64 = 8_000;
/// Modrinth accepte 300 requêtes par minute ; on reste large sous le plafond.
pub const ANALYTICS_BATCH: usize = 10;

pub fn backoff_ms(attempt: u32) -> u64 {
    (400u64 << attempt.min(16)).min(MAX_BACKOFF_MS)
}

pub fn should_retry(status: u16) -> bool {
    status == 429 || (500..600).contains(&status)
}

pub fn chunk_ids(ids: &[String], size: usize) -> Vec<Vec<String>> {
    ids.chunks(size.max(1)).map(|c| c.to_vec()).collect()
}

pub fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::Remote {
            provider: "http".into(),
            detail: e.to_string(),
        })
}

/// Exécute une requête avec retry borné sur 429 et 5xx.
/// `make` est rappelé à chaque tentative car un RequestBuilder n'est pas clonable.
pub async fn send_with_retry(
    provider: &str,
    make: impl Fn() -> reqwest::RequestBuilder,
) -> Result<reqwest::Response> {
    let mut attempt = 0;
    loop {
        match make().send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                if status == 401 || status == 403 {
                    return Err(AppError::auth(provider, format!("HTTP {status}")));
                }
                if !should_retry(status) {
                    return Ok(response);
                }
                if attempt >= MAX_RETRIES {
                    return Err(AppError::remote(
                        provider,
                        format!("HTTP {status} après {attempt} reprises"),
                    ));
                }
            }
            Err(e) => {
                if attempt >= MAX_RETRIES {
                    return Err(AppError::remote(provider, e.to_string()));
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(backoff_ms(attempt))).await;
        attempt += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_is_bounded() {
        assert_eq!(backoff_ms(0), 400);
        assert_eq!(backoff_ms(1), 800);
        assert_eq!(backoff_ms(2), 1600);
        assert_eq!(backoff_ms(9), MAX_BACKOFF_MS);
    }

    #[test]
    fn only_transient_statuses_are_retried() {
        assert!(should_retry(429));
        assert!(should_retry(500));
        assert!(should_retry(503));
        assert!(!should_retry(400));
        assert!(!should_retry(401));
        assert!(!should_retry(404));
        assert!(!should_retry(200));
    }

    #[test]
    fn chunking_respects_batch_size() {
        let ids: Vec<String> = (0..25).map(|i| i.to_string()).collect();
        let chunks = chunk_ids(&ids, 10);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 10);
        assert_eq!(chunks[2].len(), 5);
    }
}
