//! The app's brain: the list of links, the periodic checks and the downloads.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use reqwest::header::{CONTENT_DISPOSITION, CONTENT_TYPE, HeaderMap};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::model::{Account, LinkItem, Persisted, Settings, Status};
use crate::rapidgator::{self, Api, ApiError};
use crate::secrets::{self, Secret};

/// Minutes between automatic checks that the user can choose from.
pub const INTERVALS: [u32; 5] = [5, 10, 15, 30, 60];

const STATE_FILE: &str = "enlacetech.json";
/// `file/check_link` takes at most 25 links per call.
const BATCH: usize = 25;
/// Pause between two download probes, to go easy on Rapidgator's API.
const PROBE_GAP: Duration = Duration::from_millis(400);
/// A transfer that receives nothing for this long is given up.
const STALL: Duration = Duration::from_secs(60);

/// Runs an API call with the current token and, if Rapidgator says the session
/// is gone, opens a new one and tries exactly once more.
macro_rules! authed {
    ($engine:expr, |$token:ident| $call:expr) => {{
        let $token = $engine.ensure_token().await?;
        match $call.await {
            Err(error) if error.is_session() => {
                let $token = $engine.relogin(Some(&$token)).await?;
                $call.await
            }
            other => other,
        }
    }};
}

/// Everything the window needs to draw itself.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    links: Vec<LinkItem>,
    download_dir: String,
    default_dir: bool,
    interval_minutes: u32,
    remember_password: bool,
    notifications: bool,
    keep_in_tray: bool,
    account: Option<Account>,
    signed_in: bool,
    /// False until the saved session has been read from the keychain.
    ready: bool,
    checking: bool,
    last_check: Option<u64>,
    next_check: Option<u64>,
    notice: Option<String>,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddReport {
    pub added: u32,
    pub duplicates: u32,
    pub invalid: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    id: String,
    received: u64,
    total: Option<u64>,
    speed: u64,
}

#[derive(Default)]
struct Data {
    links: Vec<LinkItem>,
    settings: Settings,
    account: Option<Account>,
    token: Option<String>,
    password: Option<String>,
    last_check: Option<u64>,
    next_check: Option<u64>,
    notice: Option<String>,
}

impl Data {
    fn signed_in(&self) -> bool {
        self.token.is_some() || self.password.is_some()
    }

    fn link_mut(&mut self, id: &str) -> Option<&mut LinkItem> {
        self.links.iter_mut().find(|l| l.id == id)
    }
}

impl LinkItem {
    /// States a check must leave alone.
    fn busy(&self) -> bool {
        matches!(self.status, Status::Downloading | Status::Completed)
    }

    fn mark_missing(&mut self, raw: &str) {
        self.status = Status::Missing;
        self.reason = Some(
            "El archivo no existe en Rapidgator: fue eliminado, es privado o el enlace está mal escrito."
                .into(),
        );
        self.raw_reason = Some(raw.to_owned());
    }

    /// Rapidgator answered the download request with an error.
    fn refuse(&mut self, status: u16, details: &str) {
        if status == 404 {
            self.mark_missing(&clean(details));
        } else {
            self.status = Status::Unavailable;
            self.reason = Some(explain(details));
            self.raw_reason = Some(clean(details));
        }
    }
}

enum DlError {
    Cancelled,
    /// Rapidgator said no: (status, details).
    Refused(u16, String),
    /// The transfer broke.
    Failed(String),
}

impl From<ApiError> for DlError {
    fn from(error: ApiError) -> Self {
        match error {
            ApiError::Api { status, details } => DlError::Refused(status, details),
            ApiError::Network(message) => DlError::Failed(message),
            ApiError::NoSession => {
                DlError::Failed("La sesión de Rapidgator caducó. Inicia sesión de nuevo en Ajustes.".into())
            }
        }
    }
}

impl From<std::io::Error> for DlError {
    fn from(error: std::io::Error) -> Self {
        DlError::Failed(format!("No se pudo guardar el archivo: {error}"))
    }
}

pub struct Engine {
    app: AppHandle,
    api: Api,
    file: PathBuf,
    /// Name the session is stored under in the OS keychain: the app's identifier.
    service: String,
    /// True once the tray icon exists. Without it, a hidden window could never come back.
    tray_ready: AtomicBool,
    /// What the tray last said, so it is only touched when the text changes.
    tray_text: Mutex<String>,
    data: Mutex<Data>,
    downloads: Mutex<HashMap<String, CancellationToken>>,
    /// Names of the links that became downloadable during the check in progress.
    fresh: Mutex<Vec<String>>,
    checking: AtomicBool,
    rerun: AtomicBool,
    ready: AtomicBool,
    /// Restarts the countdown to the next automatic check.
    wake: Notify,
    /// One login at a time.
    auth: tokio::sync::Mutex<()>,
    /// One write of the state file at a time.
    saving: tokio::sync::Mutex<()>,
}

impl Engine {
    pub fn new(app: &AppHandle) -> Arc<Self> {
        let file = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(STATE_FILE);

        let Persisted {
            mut links,
            mut settings,
            account,
        } = std::fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();

        // A download that was running when the app closed has to be checked again.
        for link in &mut links {
            if link.status == Status::Downloading {
                link.status = Status::Pending;
            }
        }
        if !INTERVALS.contains(&settings.interval_minutes) {
            settings.interval_minutes = Settings::default().interval_minutes;
        }

        Arc::new(Self {
            app: app.clone(),
            api: Api::new(),
            file,
            service: app.config().identifier.clone(),
            tray_ready: AtomicBool::new(false),
            tray_text: Mutex::new(String::new()),
            data: Mutex::new(Data {
                links,
                settings,
                account,
                ..Default::default()
            }),
            downloads: Mutex::new(HashMap::new()),
            fresh: Mutex::new(Vec::new()),
            checking: AtomicBool::new(false),
            rerun: AtomicBool::new(false),
            ready: AtomicBool::new(false),
            wake: Notify::new(),
            auth: tokio::sync::Mutex::new(()),
            saving: tokio::sync::Mutex::new(()),
        })
    }

    /// Restores the session and starts the automatic checks.
    pub fn start(self: &Arc<Self>) {
        let engine = self.clone();
        tauri::async_runtime::spawn(async move { engine.run().await });
    }

    async fn run(self: Arc<Self>) {
        let service = self.service.clone();
        let secret = tauri::async_runtime::spawn_blocking(move || secrets::load(&service))
            .await
            .ok()
            .flatten();
        self.with(|d| {
            if let Some(secret) = secret {
                d.token = secret.token;
                d.password = secret.password;
                if d.account.as_ref().is_none_or(|a| a.email != secret.email) {
                    d.account = Some(Account {
                        email: secret.email,
                        ..Default::default()
                    });
                }
            }
        });
        self.ready.store(true, Ordering::SeqCst);
        self.emit();

        tokio::time::sleep(Duration::from_millis(1200)).await;
        self.run_check(None, false).await;

        loop {
            let minutes = self.with(|d| d.settings.interval_minutes.max(1));
            let wait = Duration::from_secs(u64::from(minutes) * 60);
            self.with(|d| d.next_check = Some(now_ms() + wait.as_millis() as u64));
            self.emit();
            tokio::select! {
                _ = tokio::time::sleep(wait) => self.run_check(None, false).await,
                // The interval changed or a manual check just ran: start counting again.
                _ = self.wake.notified() => {}
            }
        }
    }

    // ───────────────────────────── state ─────────────────────────────

    fn with<R>(&self, f: impl FnOnce(&mut Data) -> R) -> R {
        f(&mut lock(&self.data))
    }

    fn default_dir(&self) -> PathBuf {
        let path = self.app.path();
        path.download_dir()
            .or_else(|_| path.home_dir().map(|home| home.join("Downloads")))
            .unwrap_or_else(|_| PathBuf::from("."))
    }

    pub fn download_dir(&self) -> PathBuf {
        self.with(|d| d.settings.download_dir.clone())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_dir())
    }

    pub fn snapshot(&self) -> Snapshot {
        let default_dir = self.default_dir();
        self.with(|d| Snapshot {
            links: d.links.clone(),
            download_dir: d
                .settings
                .download_dir
                .clone()
                .unwrap_or_else(|| default_dir.to_string_lossy().into_owned()),
            default_dir: d.settings.download_dir.is_none(),
            interval_minutes: d.settings.interval_minutes,
            remember_password: d.settings.remember_password,
            notifications: d.settings.notifications,
            keep_in_tray: d.settings.keep_in_tray,
            account: d.account.clone(),
            signed_in: d.signed_in(),
            ready: self.ready.load(Ordering::SeqCst),
            checking: self.checking.load(Ordering::SeqCst),
            last_check: d.last_check,
            next_check: d.next_check,
            notice: d.notice.clone(),
        })
    }

    pub fn emit(&self) {
        let snapshot = self.snapshot();
        self.refresh_tray(&snapshot.links);
        let _ = self.app.emit("snapshot", snapshot);
    }

    /// Keeps the tray's status line and tooltip in step with the list.
    fn refresh_tray(&self, links: &[LinkItem]) {
        let text = tray_summary(links);
        {
            let mut last = lock(&self.tray_text);
            if *last == text {
                return;
            }
            last.clone_from(&text);
        }
        crate::tray::set_status(&self.app, &text);
    }

    pub async fn save(&self) {
        let _one_at_a_time = self.saving.lock().await;
        let bytes = self.with(|d| {
            serde_json::to_vec_pretty(&Persisted {
                links: d.links.clone(),
                settings: d.settings.clone(),
                account: d.account.clone(),
            })
        });
        let Ok(bytes) = bytes else { return };
        if let Some(dir) = self.file.parent() {
            let _ = tokio::fs::create_dir_all(dir).await;
        }
        let tmp = self.file.with_extension("json.tmp");
        if tokio::fs::write(&tmp, bytes).await.is_ok() {
            let _ = tokio::fs::rename(&tmp, &self.file).await;
        }
    }

    // ───────────────────────────── links ─────────────────────────────

    /// Adds every Rapidgator link found in `text`. Returns the report for the
    /// user and the ids of the links that are new.
    pub fn add_links(&self, text: &str) -> (AddReport, Vec<String>) {
        let mut report = AddReport::default();
        let mut fresh: Vec<LinkItem> = Vec::new();
        let now = now_ms();

        self.with(|d| {
            for raw in text.split_whitespace() {
                let Some(parsed) = rapidgator::parse_link(raw) else {
                    report.invalid.push(raw.chars().take(90).collect());
                    continue;
                };
                let known = d.links.iter().chain(fresh.iter()).any(|l| l.file_id == parsed.file_id);
                if known {
                    report.duplicates += 1;
                    continue;
                }
                report.added += 1;
                fresh.push(LinkItem {
                    id: parsed.file_id.clone(),
                    file_id: parsed.file_id,
                    url: parsed.url,
                    added_at: now,
                    ..Default::default()
                });
            }
            let ids = fresh.iter().map(|l| l.id.clone()).collect();
            // Newest on top, in the order they were pasted.
            d.links.splice(0..0, fresh);
            (report, ids)
        })
    }

    pub async fn remove(&self, id: &str) {
        if let Some(cancel) = lock(&self.downloads).remove(id) {
            cancel.cancel();
        }
        self.with(|d| d.links.retain(|l| l.id != id));
        self.save().await;
        self.emit();
    }

    pub async fn clear_completed(&self) {
        self.with(|d| d.links.retain(|l| l.status != Status::Completed));
        self.save().await;
        self.emit();
    }

    pub fn saved_path(&self, id: &str) -> Option<PathBuf> {
        self.with(|d| d.link_mut(id).and_then(|l| l.saved_path.clone()))
            .map(PathBuf::from)
    }

    // ──────────────────────────── settings ────────────────────────────

    pub async fn set_download_dir(&self, dir: Option<PathBuf>) {
        self.with(|d| d.settings.download_dir = dir.map(|p| p.to_string_lossy().into_owned()));
        self.save().await;
        self.emit();
    }

    pub async fn set_interval(&self, minutes: u32) -> Result<(), String> {
        if !INTERVALS.contains(&minutes) {
            return Err("Ese intervalo no está disponible.".into());
        }
        self.with(|d| d.settings.interval_minutes = minutes);
        self.save().await;
        self.wake.notify_one();
        self.emit();
        Ok(())
    }

    pub async fn set_notifications(&self, enabled: bool) {
        self.with(|d| d.settings.notifications = enabled);
        self.save().await;
        self.emit();
    }

    pub async fn set_keep_in_tray(&self, enabled: bool) {
        self.with(|d| d.settings.keep_in_tray = enabled);
        self.save().await;
        self.emit();
    }

    /// Called once the tray icon has been created.
    pub fn tray_is_ready(&self) {
        self.tray_ready.store(true, Ordering::SeqCst);
    }

    /// Whether closing the window should hide it instead of quitting. Never
    /// true without a tray icon: there would be no way to get the window back.
    pub fn stays_in_tray(&self) -> bool {
        self.tray_ready.load(Ordering::SeqCst) && self.with(|d| d.settings.keep_in_tray)
    }

    /// The window was closed but the app keeps running. The first time, say
    /// so, because it looks exactly like quitting.
    pub fn window_hidden(self: &Arc<Self>) {
        let (first_time, tell) = self.with(|d| {
            let first_time = !d.settings.tray_hint_shown;
            d.settings.tray_hint_shown = true;
            (first_time, first_time && d.settings.notifications)
        });
        if tell {
            let body = format!(
                "Seguirá comprobando tus enlaces desde {}. Para cerrarlo del todo, elige Salir en ese menú.",
                crate::tray::PLACE
            );
            let _ = self.notify("EnlaceTech sigue abierto", &body);
        }
        if first_time {
            let engine = self.clone();
            tauri::async_runtime::spawn(async move { engine.save().await });
        }
    }

    /// Sends a sample notification, whatever the setting and the window's focus.
    pub fn test_notification(&self) -> Result<(), String> {
        self.notify(
            "EnlaceTech",
            "Así te avisaré cuando un enlace quede listo para descargar.",
        )
        .map_err(|_| "No se pudo enviar el aviso.".to_owned())
    }

    fn notify(&self, title: &str, body: &str) -> tauri_plugin_notification::Result<()> {
        self.app.notification().builder().title(title).body(body).show()
    }

    /// Tells the user about the links that just became downloadable, unless
    /// they turned notifications off or are already looking at the list.
    fn announce(&self) {
        let names = std::mem::take(&mut *lock(&self.fresh));
        if names.is_empty() || !self.with(|d| d.settings.notifications) {
            return;
        }
        let watching = self
            .app
            .get_webview_window("main")
            .and_then(|window| window.is_focused().ok())
            .unwrap_or(false);
        if watching {
            return;
        }
        let (title, body) = availability_message(&names);
        let _ = self.notify(&title, &body);
    }

    // ───────────────────────────── session ─────────────────────────────

    pub async fn login(
        &self,
        email: String,
        password: String,
        code: Option<String>,
        remember: bool,
    ) -> Result<(), String> {
        let email = email.trim().to_owned();
        if email.is_empty() || password.is_empty() {
            return Err("Escribe tu correo y tu contraseña de Rapidgator.".into());
        }

        let outcome = {
            let _one_at_a_time = self.auth.lock().await;
            self.api.login(&email, &password, code.as_deref()).await
        };
        match outcome {
            Ok((token, account)) => {
                self.with(|d| {
                    d.token = Some(token);
                    d.account = Some(account);
                    d.password = remember.then_some(password);
                    d.settings.remember_password = remember;
                    d.notice = None;
                });
                self.persist_secret().await;
                self.save().await;
                self.emit();
                Ok(())
            }
            Err(ApiError::Api { details, .. }) => Err(explain_login(&details)),
            Err(ApiError::Network(message)) => Err(message),
            Err(ApiError::NoSession) => Err("No se pudo iniciar sesión.".into()),
        }
    }

    pub async fn logout(&self) {
        self.with(|d| {
            d.token = None;
            d.password = None;
            d.account = None;
            d.notice = None;
        });
        self.persist_secret().await;
        self.save().await;
        self.emit();
    }

    async fn ensure_token(&self) -> Result<String, ApiError> {
        match self.with(|d| d.token.clone()) {
            Some(token) => Ok(token),
            None => self.relogin(None).await,
        }
    }

    /// Opens a new session with the remembered password. `stale` is the token
    /// that just stopped working.
    async fn relogin(&self, stale: Option<&str>) -> Result<String, ApiError> {
        let _one_at_a_time = self.auth.lock().await;

        // Another task may have renewed the session while this one waited.
        if let Some(current) = self.with(|d| d.token.clone()) {
            if Some(current.as_str()) != stale {
                return Ok(current);
            }
        }

        let credentials = self.with(|d| {
            let email = d.account.as_ref().map(|a| a.email.clone())?;
            Some((email, d.password.clone()?))
        });
        let Some((email, password)) = credentials else {
            self.end_session("La sesión de Rapidgator caducó. Inicia sesión de nuevo en Ajustes.")
                .await;
            return Err(ApiError::NoSession);
        };

        match self.api.login(&email, &password, None).await {
            Ok((token, account)) => {
                self.with(|d| {
                    d.token = Some(token.clone());
                    d.account = Some(account);
                });
                self.persist_secret().await;
                Ok(token)
            }
            // Never insist with a password Rapidgator rejected: repeated wrong
            // logins get the IP blocked.
            Err(ApiError::Api { details, .. }) => {
                self.end_session(format!(
                    "Rapidgator no dejó renovar la sesión ({}). Inicia sesión de nuevo en Ajustes.",
                    clean(&details)
                ))
                .await;
                Err(ApiError::NoSession)
            }
            Err(error) => Err(error),
        }
    }

    async fn end_session(&self, notice: impl Into<String>) {
        let notice = notice.into();
        self.with(|d| {
            d.token = None;
            d.password = None;
            d.notice = Some(notice);
        });
        self.persist_secret().await;
        self.emit();
    }

    /// Mirrors the session into the OS keychain, or removes it when signed out.
    async fn persist_secret(&self) {
        let secret = self.with(|d| {
            if !d.signed_in() {
                return None;
            }
            d.account.as_ref().map(|a| Secret {
                email: a.email.clone(),
                password: d.password.clone(),
                token: d.token.clone(),
            })
        });
        let service = self.service.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || match secret {
            Some(secret) => {
                secrets::save(&service, &secret);
            }
            None => secrets::clear(&service),
        })
        .await;
    }

    // ───────────────────────────── checks ─────────────────────────────

    /// Manual check of one link or of all of them.
    pub fn check_now(self: &Arc<Self>, only: Option<String>) -> Result<(), String> {
        if !self.with(|d| d.signed_in()) {
            return Err("Inicia sesión en Ajustes para comprobar los enlaces.".into());
        }
        let engine = self.clone();
        tauri::async_runtime::spawn(async move {
            let all = only.is_none();
            engine.run_check(only.map(|id| vec![id]), true).await;
            if all {
                engine.wake.notify_one();
            }
        });
        Ok(())
    }

    /// Checks `only` (or every link). With `force`, links already known to be
    /// available are asked again too.
    pub async fn run_check(&self, only: Option<Vec<String>>, force: bool) {
        if self.checking.swap(true, Ordering::SeqCst) {
            // A check is already running; it will go around once more when done.
            self.rerun.store(true, Ordering::SeqCst);
            return;
        }

        let mut only = only;
        let mut force = force;
        loop {
            let outcome = self.check_pass(only.take(), force).await;
            self.announce();
            self.with(|d| {
                for link in &mut d.links {
                    link.checking = false;
                }
                match outcome {
                    Ok(true) => {
                        d.last_check = Some(now_ms());
                        d.notice = None;
                    }
                    Ok(false) => {}
                    Err(ApiError::Network(message)) => d.notice = Some(message),
                    Err(ApiError::Api { details, .. }) => {
                        d.notice = Some(format!("Rapidgator respondió: {}", clean(&details)));
                    }
                    // `end_session` already left its own notice.
                    Err(ApiError::NoSession) => {}
                }
            });
            if !self.rerun.swap(false, Ordering::SeqCst) {
                break;
            }
            force = false;
        }

        self.checking.store(false, Ordering::SeqCst);
        self.save().await;
        self.emit();
    }

    /// One round: account info, existence of every link (in batches), and a
    /// download probe for the ones that exist. `Ok(false)` means there was
    /// nothing to do.
    async fn check_pass(&self, only: Option<Vec<String>>, force: bool) -> Result<bool, ApiError> {
        // (id, was already available)
        let targets: Vec<(String, bool)> = self.with(|d| {
            if !d.signed_in() {
                return Vec::new();
            }
            let mut targets = Vec::new();
            for link in &mut d.links {
                if link.busy() || only.as_ref().is_some_and(|ids| !ids.contains(&link.id)) {
                    continue;
                }
                link.checking = true;
                targets.push((link.id.clone(), link.status == Status::Available));
            }
            targets
        });
        if targets.is_empty() {
            return Ok(false);
        }
        self.emit();

        let mut account = authed!(self, |token| self.api.user_info(&token))?;
        self.with(|d| {
            if account.email.is_empty() {
                if let Some(previous) = &d.account {
                    account.email = previous.email.clone();
                }
            }
            d.account = Some(account);
        });

        for batch in targets.chunks(BATCH) {
            let urls: Vec<String> = batch.iter().map(|(id, _)| rapidgator::canonical(id)).collect();
            let rows = authed!(self, |token| self.api.check_links(&token, &urls))?;
            let now = now_ms();

            let to_probe: Vec<String> = self.with(|d| {
                let mut to_probe = Vec::new();
                for (index, (id, was_available)) in batch.iter().enumerate() {
                    let Some(link) = d.link_mut(id) else { continue };
                    if link.busy() {
                        link.checking = false;
                        continue;
                    }
                    // Match by file id; fall back to position when the answer
                    // has one row per link in the order they were sent.
                    let row = rows
                        .iter()
                        .find(|r| r.file_id.as_deref() == Some(id.as_str()))
                        .or_else(|| (rows.len() == batch.len()).then(|| &rows[index]));
                    link.checked_at = Some(now);
                    match row {
                        Some(row) if row.access => {
                            if let Some(name) = &row.name {
                                link.name = Some(name.clone());
                            }
                            if row.size.is_some() {
                                link.size = row.size;
                            }
                            if force || !*was_available {
                                to_probe.push(id.clone());
                            } else {
                                link.checking = false;
                            }
                        }
                        Some(_) => {
                            link.mark_missing("NO ACCESS");
                            link.checking = false;
                        }
                        // Rapidgator said nothing about this link: leave it as it was.
                        None => link.checking = false,
                    }
                }
                to_probe
            });
            self.emit();

            // Asking for the download link is the only way to know whether
            // Rapidgator will actually let this account download the file now.
            for (n, id) in to_probe.iter().enumerate() {
                if n > 0 {
                    tokio::time::sleep(PROBE_GAP).await;
                }
                let outcome = authed!(self, |token| self.api.download(&token, id));
                let now = now_ms();
                match outcome {
                    Ok(ticket) => self.with(|d| {
                        let Some(link) = d.link_mut(id) else { return };
                        link.checking = false;
                        if link.busy() {
                            return;
                        }
                        link.checked_at = Some(now);
                        if link.status != Status::Available {
                            let name = link.name.clone().unwrap_or_else(|| "Un enlace".to_owned());
                            lock(&self.fresh).push(name);
                        }
                        link.status = Status::Available;
                        link.raw_reason = None;
                        link.reason = (ticket.delay > 0).then(|| {
                            format!(
                                "Rapidgator pide esperar {} antes de empezar la descarga.",
                                human_secs(ticket.delay)
                            )
                        });
                    }),
                    Err(ApiError::Api { status, details }) if status != 401 => self.with(|d| {
                        let Some(link) = d.link_mut(id) else { return };
                        link.checking = false;
                        if link.busy() {
                            return;
                        }
                        link.checked_at = Some(now);
                        link.refuse(status, &details);
                    }),
                    Err(error) => return Err(error),
                }
                self.emit();
            }
        }
        Ok(true)
    }

    // ──────────────────────────── downloads ────────────────────────────

    pub fn start_download(self: &Arc<Self>, id: &str) -> Result<(), String> {
        self.with(|d| {
            if !d.signed_in() {
                return Err("Inicia sesión en Ajustes para descargar.".to_owned());
            }
            let link = d
                .link_mut(id)
                .ok_or_else(|| "Ese enlace ya no está en la lista.".to_owned())?;
            if link.status == Status::Downloading {
                return Err("Ese archivo ya se está descargando.".to_owned());
            }
            link.status = Status::Downloading;
            link.checking = false;
            link.received = 0;
            link.speed = 0;
            link.reason = None;
            link.raw_reason = None;
            link.saved_path = None;
            Ok(())
        })?;

        let cancel = CancellationToken::new();
        lock(&self.downloads).insert(id.to_owned(), cancel.clone());
        self.emit();

        let engine = self.clone();
        let id = id.to_owned();
        tauri::async_runtime::spawn(async move { engine.download_task(id, cancel).await });
        Ok(())
    }

    pub fn cancel_download(&self, id: &str) {
        if let Some(cancel) = lock(&self.downloads).get(id) {
            cancel.cancel();
        }
    }

    async fn download_task(&self, id: String, cancel: CancellationToken) {
        // Path of the partial file, so it can be removed if the download does not finish.
        let part: Mutex<Option<PathBuf>> = Mutex::new(None);
        let outcome = tokio::select! {
            _ = cancel.cancelled() => Err(DlError::Cancelled),
            result = self.download_file(&id, &part) => result,
        };
        if outcome.is_err() {
            let leftover = lock(&part).take();
            if let Some(path) = leftover {
                let _ = tokio::fs::remove_file(path).await;
            }
        }
        lock(&self.downloads).remove(&id);

        self.with(|d| {
            let Some(link) = d.link_mut(&id) else { return };
            link.speed = 0;
            match outcome {
                Ok(path) => {
                    link.status = Status::Completed;
                    link.saved_path = Some(path.to_string_lossy().into_owned());
                    link.reason = None;
                    link.raw_reason = None;
                    if let Some(size) = link.size {
                        link.received = size;
                    }
                }
                Err(DlError::Cancelled) => {
                    link.status = Status::Available;
                    link.reason = None;
                    link.received = 0;
                }
                Err(DlError::Refused(status, details)) => {
                    link.received = 0;
                    link.checked_at = Some(now_ms());
                    link.refuse(status, &details);
                }
                Err(DlError::Failed(message)) => {
                    link.status = Status::Failed;
                    link.reason = Some(message);
                    link.raw_reason = None;
                    link.received = 0;
                }
            }
        });
        self.save().await;
        self.emit();
    }

    async fn download_file(&self, id: &str, part: &Mutex<Option<PathBuf>>) -> Result<PathBuf, DlError> {
        let (known_name, known_size) = self
            .with(|d| d.link_mut(id).map(|l| (l.name.clone(), l.size)))
            .ok_or(DlError::Cancelled)?;

        let ticket = authed!(self, |token| self.api.download(&token, id))?;

        if ticket.delay > 0 {
            let message = format!(
                "Rapidgator pide esperar {} antes de empezar…",
                human_secs(ticket.delay)
            );
            self.with(|d| {
                if let Some(link) = d.link_mut(id) {
                    link.reason = Some(message);
                }
            });
            self.emit();
            tokio::time::sleep(Duration::from_secs(ticket.delay.min(3600))).await;
            self.with(|d| {
                if let Some(link) = d.link_mut(id) {
                    link.reason = None;
                }
            });
            self.emit();
        }

        let dir = self.download_dir();
        tokio::fs::create_dir_all(&dir).await.map_err(|error| {
            DlError::Failed(format!(
                "No se pudo usar la carpeta de descargas ({}): {error}",
                dir.display()
            ))
        })?;

        let response = self
            .api
            .http()
            .get(&ticket.url)
            .send()
            .await
            .map_err(|e| DlError::Failed(rapidgator::describe(&e)))?;
        if !response.status().is_success() {
            return Err(DlError::Failed(format!(
                "El servidor de Rapidgator respondió con HTTP {}.",
                response.status().as_u16()
            )));
        }
        let is_page = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.trim_start().starts_with("text/html"));
        if is_page {
            return Err(DlError::Failed(
                "Rapidgator devolvió una página web en lugar del archivo. Vuelve a comprobar el enlace."
                    .into(),
            ));
        }

        let total = response.content_length().filter(|n| *n > 0).or(known_size);
        let name = known_name
            .or_else(|| disposition_name(response.headers()))
            .unwrap_or_else(|| id.to_owned());
        let target = unique_path(&dir, &sanitize(&name));
        let partial = part_of(&target);
        *lock(part) = Some(partial.clone());

        let mut file = tokio::fs::File::create(&partial).await?;
        let mut stream = response.bytes_stream();
        let mut received: u64 = 0;
        let mut speed: f64 = 0.0;
        let mut mark = Instant::now();
        let mut mark_bytes: u64 = 0;
        self.progress(id, 0, total, 0);

        loop {
            let next = tokio::time::timeout(STALL, stream.next()).await.map_err(|_| {
                DlError::Failed("La descarga se detuvo: no llegaron datos durante un minuto.".into())
            })?;
            let Some(chunk) = next else { break };
            let chunk = chunk.map_err(|e| DlError::Failed(rapidgator::describe(&e)))?;
            file.write_all(&chunk).await?;
            received += chunk.len() as u64;

            let elapsed = mark.elapsed();
            if elapsed >= Duration::from_millis(300) {
                let instant = (received - mark_bytes) as f64 / elapsed.as_secs_f64();
                speed = if speed == 0.0 { instant } else { speed * 0.7 + instant * 0.3 };
                mark = Instant::now();
                mark_bytes = received;
                self.progress(id, received, total, speed as u64);
            }
        }
        file.flush().await?;
        drop(file);

        if total.is_some_and(|total| received < total) {
            return Err(DlError::Failed(
                "La conexión se cortó antes de terminar la descarga.".into(),
            ));
        }
        tokio::fs::rename(&partial, &target).await?;
        *lock(part) = None;
        Ok(target)
    }

    fn progress(&self, id: &str, received: u64, total: Option<u64>, speed: u64) {
        self.with(|d| {
            if let Some(link) = d.link_mut(id) {
                link.received = received;
                link.speed = speed;
                if link.size.is_none() {
                    link.size = total;
                }
            }
        });
        let _ = self.app.emit(
            "progress",
            Progress {
                id: id.to_owned(),
                received,
                total,
                speed,
            },
        );
    }
}

// ───────────────────────────── helpers ─────────────────────────────

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn human_secs(secs: u64) -> String {
    match secs {
        0..=89 => format!("{secs} s"),
        90..=5399 => format!("{} min", (secs + 30) / 60),
        _ => format!("{} h {} min", secs / 3600, (secs % 3600) / 60),
    }
}

/// One line about how the links are doing, for the tray menu and its tooltip.
fn tray_summary(links: &[LinkItem]) -> String {
    if links.is_empty() {
        return "Sin enlaces guardados".to_owned();
    }
    let count = |wanted: &[Status]| links.iter().filter(|l| wanted.contains(&l.status)).count();
    let ready = count(&[Status::Available]);
    let active = count(&[Status::Downloading]);
    let waiting = count(&[Status::Pending, Status::Unavailable, Status::Missing, Status::Failed]);

    let mut parts = Vec::new();
    match ready {
        0 => {}
        1 => parts.push("1 lista para descargar".to_owned()),
        n => parts.push(format!("{n} listas para descargar")),
    }
    if active > 0 {
        parts.push(format!("{active} descargándose"));
    }
    if waiting > 0 {
        parts.push(format!("{waiting} en espera"));
    }
    if parts.is_empty() {
        "Todo descargado".to_owned()
    } else {
        parts.join(" · ")
    }
}

/// Title and body of the notification for links that became downloadable.
fn availability_message(names: &[String]) -> (String, String) {
    const SHOWN: usize = 3;
    match names {
        [only] => ("Ya se puede descargar".to_owned(), only.clone()),
        _ => {
            let mut body = names.iter().take(SHOWN).cloned().collect::<Vec<_>>().join(", ");
            if names.len() > SHOWN {
                body.push_str(&format!(" y {} más", names.len() - SHOWN));
            }
            (format!("{} enlaces listos para descargar", names.len()), body)
        }
    }
}

/// Rapidgator's message without its "Error:" prefix.
fn clean(details: &str) -> String {
    let text = details.trim();
    let text = text
        .strip_prefix("Error:")
        .or_else(|| text.strip_prefix("Error."))
        .unwrap_or(text)
        .trim();
    text.chars().take(240).collect()
}

/// A Spanish explanation of why Rapidgator refused a download. It is a best
/// guess from the wording; the original message is always shown next to it.
fn explain(details: &str) -> String {
    let d = details.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| d.contains(w));

    let text = if has(&["traffic", "quota", "bandwidth"]) {
        "Se agotó el tráfico de descarga de tu cuenta. Se volverá a intentar en la próxima comprobación."
    } else if has(&["same time", "simultaneous", "parallel", "already downloading"]) {
        "Tu cuenta ya tiene otra descarga en curso en Rapidgator."
    } else if has(&["wait", "delay", "minute", "hour"]) {
        "Rapidgator pide esperar antes de la siguiente descarga."
    } else if has(&["limit"]) {
        "Alcanzaste un límite de descargas de Rapidgator."
    } else if has(&["premium"]) {
        "Rapidgator solo permite esta descarga con una cuenta premium."
    } else if has(&["denied by ip", "ip address", "unusual ip"]) {
        "Rapidgator rechazó la petición desde tu dirección IP."
    } else if has(&["blocked", "banned", "abuse", "copyright", "illegal"]) {
        "Rapidgator bloqueó este archivo."
    } else if has(&["private"]) {
        "El archivo es privado."
    } else if has(&["session"]) {
        "La sesión de Rapidgator caducó."
    } else {
        "Rapidgator no permite descargar este archivo ahora."
    };
    text.to_owned()
}

fn explain_login(details: &str) -> String {
    let d = details.to_lowercase();
    if d.contains("login or password") {
        "El correo o la contraseña no son correctos.".into()
    } else if d.contains("auth code") || d.contains("2fa") {
        "El código de verificación en dos pasos falta o no es válido.".into()
    } else {
        format!("Rapidgator no aceptó el inicio de sesión: {}", clean(details))
    }
}

fn disposition_name(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(CONTENT_DISPOSITION)?.to_str().ok()?;
    let start = value.find("filename=")? + "filename=".len();
    let name = value[start..].split(';').next()?.trim().trim_matches('"').trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// Makes a name safe to use as a file name on any of the three systems.
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').trim();
    if cleaned.is_empty() {
        "descarga".into()
    } else {
        cleaned.to_owned()
    }
}

fn part_of(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".part");
    PathBuf::from(name)
}

/// `name`, or `name (1)`, `name (2)`… if something with that name is already there.
fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let as_path = Path::new(name);
    let stem = as_path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let extension = as_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();

    let mut candidate = dir.join(name);
    let mut n = 1;
    while candidate.exists() || part_of(&candidate).exists() {
        candidate = dir.join(format!("{stem} ({n}){extension}"));
        n += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explains_the_usual_refusals() {
        assert!(explain("Error: You have reached quota of premium downloaded information").contains("tráfico"));
        assert!(explain("Error: This file can be downloaded by premium only").contains("premium"));
        assert!(explain("Error: Denied by IP").contains("IP"));
        assert!(explain("Error: something new").contains("no permite"));
        assert_eq!(clean("Error: File not found"), "File not found");
    }

    #[test]
    fn refusals_set_the_right_status() {
        let mut link = LinkItem::default();
        link.refuse(404, "Error: File not found");
        assert_eq!(link.status, Status::Missing);
        assert_eq!(link.raw_reason.as_deref(), Some("File not found"));

        let mut link = LinkItem::default();
        link.refuse(403, "Error: This file can be downloaded by premium only");
        assert_eq!(link.status, Status::Unavailable);
        assert!(link.reason.is_some());
    }

    #[test]
    fn file_names_are_made_safe() {
        assert_eq!(sanitize("a/b\\c:d.rar"), "a_b_c_d.rar");
        assert_eq!(sanitize("  ..  "), "descarga");
        assert_eq!(sanitize("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(part_of(Path::new("/x/y.zip")), PathBuf::from("/x/y.zip.part"));
    }

    #[test]
    fn existing_files_are_never_overwritten() {
        let dir = std::env::temp_dir().join(format!("enlacetech-test-{}", now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(unique_path(&dir, "v.mkv"), dir.join("v.mkv"));
        std::fs::write(dir.join("v.mkv"), b"x").unwrap();
        assert_eq!(unique_path(&dir, "v.mkv"), dir.join("v (1).mkv"));
        std::fs::write(dir.join("v (1).mkv.part"), b"x").unwrap();
        assert_eq!(unique_path(&dir, "v.mkv"), dir.join("v (2).mkv"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_tray_sums_up_the_list() {
        let with = |statuses: &[Status]| -> Vec<LinkItem> {
            statuses
                .iter()
                .map(|status| LinkItem { status: *status, ..Default::default() })
                .collect()
        };

        assert_eq!(tray_summary(&[]), "Sin enlaces guardados");
        assert_eq!(tray_summary(&with(&[Status::Completed])), "Todo descargado");
        assert_eq!(tray_summary(&with(&[Status::Available])), "1 lista para descargar");
        assert_eq!(
            tray_summary(&with(&[
                Status::Available,
                Status::Available,
                Status::Downloading,
                Status::Unavailable,
                Status::Missing,
                Status::Pending,
                Status::Completed,
            ])),
            "2 listas para descargar · 1 descargándose · 3 en espera"
        );
    }

    #[test]
    fn notifications_name_what_became_available() {
        let names = |n: usize| (1..=n).map(|i| format!("f{i}.rar")).collect::<Vec<_>>();

        let (title, body) = availability_message(&names(1));
        assert_eq!(title, "Ya se puede descargar");
        assert_eq!(body, "f1.rar");

        let (title, body) = availability_message(&names(3));
        assert_eq!(title, "3 enlaces listos para descargar");
        assert_eq!(body, "f1.rar, f2.rar, f3.rar");

        let (title, body) = availability_message(&names(5));
        assert_eq!(title, "5 enlaces listos para descargar");
        assert_eq!(body, "f1.rar, f2.rar, f3.rar y 2 más");
    }

    #[test]
    fn waits_read_naturally() {
        assert_eq!(human_secs(45), "45 s");
        assert_eq!(human_secs(600), "10 min");
        assert_eq!(human_secs(7500), "2 h 5 min");
    }
}
