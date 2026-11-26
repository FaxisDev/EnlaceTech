use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    /// Added but never checked.
    #[default]
    Pending,
    /// Rapidgator handed out a download link on the last check.
    Available,
    /// The file exists but Rapidgator refused the download; `reason` says why.
    Unavailable,
    /// The file does not exist (deleted, private or a wrong link).
    Missing,
    Downloading,
    Completed,
    /// The transfer itself broke (network, disk); `reason` says why.
    Failed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkItem {
    pub id: String,
    pub url: String,
    pub file_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub status: Status,
    /// Explanation in Spanish of why the link cannot be downloaded.
    #[serde(default)]
    pub reason: Option<String>,
    /// The message exactly as Rapidgator sent it.
    #[serde(default)]
    pub raw_reason: Option<String>,
    #[serde(default)]
    pub added_at: u64,
    #[serde(default)]
    pub checked_at: Option<u64>,
    #[serde(default)]
    pub saved_path: Option<String>,

    // Live values: sent to the window, ignored when the state file is read back.
    #[serde(default, skip_deserializing)]
    pub checking: bool,
    #[serde(default, skip_deserializing)]
    pub received: u64,
    #[serde(default, skip_deserializing)]
    pub speed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// `None` means the system's Downloads folder.
    pub download_dir: Option<String>,
    pub interval_minutes: u32,
    pub remember_password: bool,
    /// Send a system notification when a link becomes downloadable.
    pub notifications: bool,
    /// Closing the window hides it and the app keeps checking from the tray.
    pub keep_in_tray: bool,
    /// The one-time "still running in the tray" notice has been shown.
    pub tray_hint_shown: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_dir: None,
            interval_minutes: 15,
            remember_password: true,
            notifications: true,
            keep_in_tray: true,
            tray_hint_shown: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Account {
    pub email: String,
    pub is_premium: bool,
    /// Unix seconds.
    pub premium_end: Option<u64>,
    pub traffic_left: Option<u64>,
    pub traffic_total: Option<u64>,
}

/// What is written to the state file. Secrets are never part of it.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Persisted {
    pub links: Vec<LinkItem>,
    pub settings: Settings,
    pub account: Option<Account>,
}
