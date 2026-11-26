//! Client for the Rapidgator API v2 (https://rapidgator.net/article/api).
//!
//! Every answer has the shape `{ response, status, details }`. `status` is 200
//! on success; otherwise `details` carries a message meant to be shown to the
//! user as is.

use std::time::Duration;

use reqwest::Client;
use serde_json::Value;

use crate::model::Account;

const BASE: &str = "https://rapidgator.net/api/v2";

#[derive(Debug, Clone)]
pub enum ApiError {
    /// Rapidgator answered, and the answer was a refusal.
    Api { status: u16, details: String },
    /// Rapidgator could not be reached, or the answer made no sense.
    Network(String),
    /// There is no session and no way to open one.
    NoSession,
}

impl ApiError {
    /// 401 on anything but the login call means the token is no longer valid.
    pub fn is_session(&self) -> bool {
        matches!(self, ApiError::Api { status: 401, .. })
    }
}

/// A ready-to-use download link.
pub struct Ticket {
    pub url: String,
    /// Seconds Rapidgator asks to wait before using the link.
    pub delay: u64,
}

/// One row of `file/check_link`.
pub struct Checked {
    pub file_id: Option<String>,
    pub access: bool,
    pub name: Option<String>,
    pub size: Option<u64>,
}

pub struct Parsed {
    pub file_id: String,
    pub url: String,
}

#[derive(Clone)]
pub struct Api {
    http: Client,
}

impl Api {
    pub fn new() -> Self {
        let http = Client::builder()
            .user_agent(concat!("EnlaceTech/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .expect("no se pudo crear el cliente HTTP");
        Self { http }
    }

    pub fn http(&self) -> &Client {
        &self.http
    }

    /// POST with a form body, so neither the password nor the token end up in a URL.
    async fn call(&self, path: &str, params: &[(&str, &str)]) -> Result<Value, ApiError> {
        let res = self
            .http
            .post(format!("{BASE}/{path}"))
            .form(params)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| ApiError::Network(describe(&e)))?;

        let http_status = res.status().as_u16();
        let mut body: Value = res.json().await.map_err(|_| {
            ApiError::Network(format!(
                "Rapidgator dio una respuesta inesperada (HTTP {http_status})."
            ))
        })?;

        let status = number(&body["status"]).map_or(http_status, |n| n as u16);
        if status == 200 {
            return Ok(body.get_mut("response").map(Value::take).unwrap_or(Value::Null));
        }
        let details = body["details"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Error {status}"));
        Err(ApiError::Api { status, details })
    }

    pub async fn login(
        &self,
        email: &str,
        password: &str,
        code: Option<&str>,
    ) -> Result<(String, Account), ApiError> {
        let mut params = vec![("login", email), ("password", password)];
        if let Some(code) = code.map(str::trim).filter(|c| !c.is_empty()) {
            params.push(("code", code));
        }
        let response = self.call("user/login", &params).await?;
        let token = response["token"]
            .as_str()
            .filter(|t| !t.is_empty())
            .ok_or_else(|| ApiError::Network("Rapidgator no devolvió una sesión.".into()))?
            .to_owned();
        Ok((token, account_from(&response["user"], email)))
    }

    pub async fn user_info(&self, token: &str) -> Result<Account, ApiError> {
        let response = self.call("user/info", &[("token", token)]).await?;
        Ok(account_from(&response["user"], ""))
    }

    /// Up to 25 links per call.
    pub async fn check_links(&self, token: &str, urls: &[String]) -> Result<Vec<Checked>, ApiError> {
        let joined = urls.join(",");
        let response = self
            .call("file/check_link", &[("token", token), ("url", &joined)])
            .await?;
        let rows = response.as_array().map(Vec::as_slice).unwrap_or_default();
        Ok(rows
            .iter()
            .map(|row| Checked {
                file_id: row["url"].as_str().and_then(parse_link).map(|p| p.file_id),
                access: row["status"]
                    .as_str()
                    .is_some_and(|s| s.trim().eq_ignore_ascii_case("ACCESS")),
                name: row["filename"]
                    .as_str()
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .map(str::to_owned),
                size: number(&row["size"]),
            })
            .collect())
    }

    pub async fn download(&self, token: &str, file_id: &str) -> Result<Ticket, ApiError> {
        let response = self
            .call("file/download", &[("token", token), ("file_id", file_id)])
            .await?;
        let url = response["download_url"]
            .as_str()
            .filter(|u| !u.is_empty())
            .ok_or_else(|| ApiError::Api {
                status: 500,
                details: "Rapidgator no devolvió un enlace de descarga".into(),
            })?
            .to_owned();
        Ok(Ticket {
            url,
            delay: number(&response["delay"]).unwrap_or(0),
        })
    }
}

/// Accepts `https://rapidgator.net/file/<id>[/<name>.html]` and the `rg.to` short form.
pub fn parse_link(raw: &str) -> Option<Parsed> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let with_scheme = if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("https://{raw}")
    };
    let url = reqwest::Url::parse(&with_scheme).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    if host != "rapidgator.net" && host != "rg.to" {
        return None;
    }
    let mut segments = url.path_segments()?;
    if segments.next()? != "file" {
        return None;
    }
    let id = segments.next()?;
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(Parsed {
        file_id: id.to_owned(),
        url: with_scheme,
    })
}

/// The link without the file name, which is all Rapidgator needs to identify it.
pub fn canonical(file_id: &str) -> String {
    format!("https://rapidgator.net/file/{file_id}")
}

pub fn describe(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "Rapidgator tardó demasiado en responder.".into()
    } else if error.is_connect() {
        "No se pudo conectar con Rapidgator. Revisa tu conexión a internet.".into()
    } else {
        "Se interrumpió la comunicación con Rapidgator.".into()
    }
}

fn account_from(user: &Value, fallback_email: &str) -> Account {
    Account {
        email: user["email"]
            .as_str()
            .filter(|e| !e.is_empty())
            .unwrap_or(fallback_email)
            .to_owned(),
        is_premium: truthy(&user["is_premium"]),
        premium_end: number(&user["premium_end_time"]).filter(|n| *n > 0),
        traffic_left: number(&user["traffic"]["left"]),
        traffic_total: number(&user["traffic"]["total"]),
    }
}

/// Rapidgator sends numbers sometimes as numbers and sometimes as strings.
fn number(value: &Value) -> Option<u64> {
    match value {
        Value::Number(n) => n.as_u64().or_else(|| n.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64)),
        Value::String(s) => s.trim().parse::<u64>().ok(),
        _ => None,
    }
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => matches!(s.trim(), "1" | "true"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_usual_link_shapes() {
        let full = parse_link("https://rapidgator.net/file/0a2d1c33d77b765bd410ab2b4fd98b9d/video.part1.rar.html").unwrap();
        assert_eq!(full.file_id, "0a2d1c33d77b765bd410ab2b4fd98b9d");

        let bare = parse_link("rapidgator.net/file/12345678").unwrap();
        assert_eq!(bare.file_id, "12345678");
        assert_eq!(bare.url, "https://rapidgator.net/file/12345678");

        assert_eq!(parse_link("http://www.rapidgator.net/file/abc123").unwrap().file_id, "abc123");
        assert_eq!(parse_link("https://rg.to/file/abc123/x.zip.html").unwrap().file_id, "abc123");
    }

    #[test]
    fn rejects_what_is_not_a_rapidgator_file() {
        assert!(parse_link("").is_none());
        assert!(parse_link("https://example.com/file/abc123").is_none());
        assert!(parse_link("https://rapidgator.net/folder/123").is_none());
        assert!(parse_link("https://rapidgator.net/file/").is_none());
        assert!(parse_link("https://evil.com/?https://rapidgator.net/file/abc").is_none());
        assert!(parse_link("ftp://rapidgator.net/file/abc").is_none());
    }

    #[test]
    fn reads_numbers_in_both_forms() {
        assert_eq!(number(&serde_json::json!(597304)), Some(597304));
        assert_eq!(number(&serde_json::json!("4398046511104")), Some(4398046511104));
        assert_eq!(number(&Value::Null), None);
        assert!(truthy(&serde_json::json!(true)));
        assert!(!truthy(&serde_json::json!(false)));
        assert!(!truthy(&Value::Null));
    }
}
