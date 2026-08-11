use crate::config::{OauthApp, Session};
use crate::error::{AppError, Result};
use crate::providers::{http_client, USER_AGENT};
use chrono::Utc;
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const AUTHORIZE: &str = "https://api.modrinth.com/_internal/oauth/authorize";
const TOKEN: &str = "https://api.modrinth.com/_internal/oauth/token";
const PROVIDER: &str = "modrinth";

/// Portée demandée : lecture du profil, des projets, des versions, des notifications
/// et des analyses. Le token ne permet aucune écriture.
pub const SCOPES: &str =
    "USER_READ PROJECT_READ VERSION_READ NOTIFICATION_READ ANALYTICS PAYOUTS_READ";

/// Délai au-delà duquel on abandonne l'attente de la redirection.
pub const CALLBACK_TIMEOUT_SECS: u64 = 300;

const PAGE_OK: &str = "<!doctype html><meta charset=\"utf-8\"><title>Chartographer</title>\
<body style=\"background:#0d1013;color:#e6ebf0;font-family:system-ui;display:grid;place-items:center;height:100vh;margin:0\">\
<div style=\"text-align:center\"><h1>Connexion reussie</h1><p>Tu peux fermer cet onglet et revenir a Chartographer.</p></div>";

const PAGE_KO: &str = "<!doctype html><meta charset=\"utf-8\"><title>Chartographer</title>\
<body style=\"background:#0d1013;color:#e6ebf0;font-family:system-ui;display:grid;place-items:center;height:100vh;margin:0\">\
<div style=\"text-align:center\"><h1>Connexion refusee</h1><p>Retourne dans Chartographer pour reessayer.</p></div>";

pub fn urlencode(value: &str) -> String {
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

fn urldecode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Valeur anti-rejeu dérivée de l'horloge, d'une adresse d'allocation et du PID.
/// L'unicité par tentative suffit : la valeur ne protège rien d'autre que le retour local.
pub fn random_state() -> String {
    let seed = Box::new(0u8);
    let entropy = format!(
        "{:x}{:x}{:x}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        std::ptr::addr_of!(*seed) as usize,
        std::process::id()
    );
    entropy
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .cycle()
        .take(40)
        .collect()
}

pub fn authorize_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    format!(
        "{AUTHORIZE}?client_id={}&redirect_uri={}&scope={}&response_type=code&state={}",
        urlencode(client_id),
        urlencode(redirect_uri),
        urlencode(SCOPES),
        urlencode(state)
    )
}

/// Extrait les paramètres de la première ligne d'une requête HTTP vers `/callback`.
pub fn parse_callback(request_line: &str) -> Option<HashMap<String, String>> {
    let target = request_line.split_whitespace().nth(1)?;
    let (path, query) = target.split_once('?')?;
    if path != "/callback" {
        return None;
    }
    Some(
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .map(|(k, v)| (urldecode(k), urldecode(v)))
            .collect(),
    )
}

pub fn parse_token_response(raw: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    if let Some(token) = value.get("access_token").and_then(|v| v.as_str()) {
        return Ok(token.to_string());
    }
    let error = value
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("réponse inattendue");
    let description = value
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    Err(AppError::auth(PROVIDER, format!("{error} : {description}")))
}

async fn wait_for_callback(listener: TcpListener, expected_state: &str) -> Result<String> {
    loop {
        let (mut socket, _) = listener
            .accept()
            .await
            .map_err(|e| AppError::remote(PROVIDER, format!("écouteur local : {e}")))?;

        let mut buffer = [0u8; 4096];
        let read = socket.read(&mut buffer).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buffer[..read]);
        let first_line = request.lines().next().unwrap_or_default();

        let Some(params) = parse_callback(first_line) else {
            let _ = socket
                .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
                .await;
            continue;
        };

        let outcome = if let Some(error) = params.get("error") {
            Err(AppError::auth(PROVIDER, error.clone()))
        } else {
            match (params.get("code"), params.get("state")) {
                (Some(code), Some(state)) if state == expected_state => Ok(code.clone()),
                (Some(_), Some(_)) => Err(AppError::auth(
                    PROVIDER,
                    "paramètre state invalide".to_string(),
                )),
                _ => Err(AppError::auth(
                    PROVIDER,
                    "réponse d'autorisation incomplète".to_string(),
                )),
            }
        };

        let page = if outcome.is_ok() { PAGE_OK } else { PAGE_KO };
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            page.len(),
            page
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.shutdown().await;
        return outcome;
    }
}

async fn exchange_code(app: &OauthApp, code: &str, redirect_uri: &str) -> Result<String> {
    let client = http_client()?;
    let response = client
        .post(TOKEN)
        .header("User-Agent", USER_AGENT)
        .header("Authorization", &app.client_secret)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", app.client_id.as_str()),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;

    let body = response
        .text()
        .await
        .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
    parse_token_response(&body)
}

/// Ouvre le navigateur, attend la redirection, échange le code et renvoie la session.
/// `open_browser` est injecté pour que l'appelant décide comment ouvrir l'URL.
pub async fn login(
    app: &OauthApp,
    open_browser: impl FnOnce(&str) -> Result<()>,
) -> Result<Session> {
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| {
        AppError::remote(PROVIDER, format!("impossible d'ouvrir un port local : {e}"))
    })?;
    let port = listener
        .local_addr()
        .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let state = random_state();

    open_browser(&authorize_url(&app.client_id, &redirect_uri, &state))?;

    let code = tokio::time::timeout(
        std::time::Duration::from_secs(CALLBACK_TIMEOUT_SECS),
        wait_for_callback(listener, &state),
    )
    .await
    .map_err(|_| AppError::auth(PROVIDER, "délai d'autorisation dépassé".to_string()))??;

    let token = exchange_code(app, &code, &redirect_uri).await?;
    let user = crate::providers::modrinth::ModrinthClient::new(&token)?
        .me()
        .await?;

    Ok(Session {
        token,
        user_id: user.id,
        username: user.username,
        obtained_at: Utc::now().to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_contains_every_required_parameter() {
        let url = authorize_url("cid", "http://127.0.0.1:7345/callback", "st4te");
        assert!(url.starts_with("https://api.modrinth.com/_internal/oauth/authorize?"));
        assert!(url.contains("client_id=cid"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("state=st4te"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A7345%2Fcallback"));
        assert!(url.contains(&format!("scope={}", urlencode(SCOPES))));
    }

    #[test]
    fn parse_callback_extracts_code_and_state() {
        let query = parse_callback("GET /callback?code=abc123&state=st4te HTTP/1.1").unwrap();
        assert_eq!(query.get("code").map(String::as_str), Some("abc123"));
        assert_eq!(query.get("state").map(String::as_str), Some("st4te"));
    }

    #[test]
    fn parse_callback_decodes_percent_escapes() {
        let query =
            parse_callback("GET /callback?error=access_denied&description=a%20refuse HTTP/1.1")
                .unwrap();
        assert_eq!(
            query.get("description").map(String::as_str),
            Some("a refuse")
        );
    }

    #[test]
    fn parse_callback_rejects_other_paths() {
        assert!(parse_callback("GET /favicon.ico HTTP/1.1").is_none());
    }

    #[test]
    fn state_values_are_unique_and_long_enough() {
        let a = random_state();
        let b = random_state();
        assert_ne!(a, b);
        assert!(a.len() >= 32);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn parse_token_response_reads_access_token() {
        let raw = r#"{"access_token":"mrp_xyz","token_type":"Bearer","expires_in":1209600}"#;
        assert_eq!(parse_token_response(raw).unwrap(), "mrp_xyz");
    }

    #[test]
    fn parse_token_response_surfaces_the_api_error() {
        let raw =
            r#"{"error":"invalid_client","description":"The provided client id was invalid"}"#;
        let message = parse_token_response(raw).unwrap_err().to_string();
        assert!(message.contains("invalid_client"));
        assert!(message.contains("client id was invalid"));
    }
}
