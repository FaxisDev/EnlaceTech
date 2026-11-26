//! The Rapidgator session, kept in the operating system's keychain
//! (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux).
//!
//! `service` is the app's identifier, so each build of the app (the real one,
//! a test one) keeps its own session.

use serde::{Deserialize, Serialize};

const ENTRY: &str = "rapidgator-session";

#[derive(Serialize, Deserialize)]
pub struct Secret {
    pub email: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
}

fn entry(service: &str) -> Option<keyring::Entry> {
    keyring::Entry::new(service, ENTRY).ok()
}

pub fn load(service: &str) -> Option<Secret> {
    let raw = entry(service)?.get_password().ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save(service: &str, secret: &Secret) -> bool {
    let Ok(raw) = serde_json::to_string(secret) else {
        return false;
    };
    entry(service).is_some_and(|e| e.set_password(&raw).is_ok())
}

pub fn clear(service: &str) {
    if let Some(e) = entry(service) {
        let _ = e.delete_credential();
    }
}
