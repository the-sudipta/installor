mod db;
mod detect;
mod error;
mod installer;
mod models;
mod sync;

use crate::{
    db::Database,
    installer::{downloader, executor, file_finder, plan},
    models::*,
    sync::{merge, public_sheet, sheets_client::SheetsClient},
};
use directories::UserDirs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};

const APPS_SCRIPT_TEMPLATE: &str = include_str!("../../apps-script/Code.gs");
const APPS_SCRIPT_SOURCE_URL: &str =
    "https://github.com/the-sudipta/installor/blob/v0.2.0/apps-script/Code.gs";

#[derive(Clone)]
struct AppState {
    db: Database,
    http: reqwest::Client,
}
type CmdResult<T> = Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn secret_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new("com.installor.desktop", "apps-script-secret").map_err(err)
}
fn get_secret() -> String {
    secret_entry()
        .and_then(|e| e.get_password().map_err(err))
        .unwrap_or_default()
}
fn default_download() -> String {
    UserDirs::new()
        .and_then(|u| u.download_dir().map(|p| p.join("Installor")))
        .unwrap_or_else(|| PathBuf::from("Installor"))
        .to_string_lossy()
        .into_owned()
}
fn machine_id(db: &Database) -> CmdResult<String> {
    if let Some(id) = db.setting("machine_id").map_err(err)? {
        Ok(id)
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        db.set_setting("machine_id", &id).map_err(err)?;
        Ok(id)
    }
}
fn should_queue(db: &Database) -> bool {
    db.setting("apps_script_url")
        .ok()
        .flatten()
        .is_some_and(|v| !v.is_empty())
}

#[tauri::command]
fn get_catalog(state: State<'_, AppState>) -> CmdResult<CatalogBundle> {
    state.db.list_bundle().map_err(err)
}

#[tauri::command]
fn save_catalog(mut entry: CatalogEntry, state: State<'_, AppState>) -> CmdResult<CatalogEntry> {
    if entry.name.trim().is_empty() {
        return Err("Software name is required".into());
    }
    let now = now_rfc3339();
    if entry.id.is_empty() {
        entry.id = uuid::Uuid::new_v4().to_string();
        entry.created_at = now.clone();
    } else if entry.created_at.is_empty() {
        entry.created_at = state
            .db
            .get_catalog(&entry.id)
            .map(|e| e.created_at)
            .unwrap_or_else(|_| now.clone());
    }
    entry.updated_at = now;
    state
        .db
        .upsert_catalog(&entry, should_queue(&state.db))
        .map_err(err)?;
    Ok(entry)
}

#[tauri::command]
fn delete_catalog(id: String, state: State<'_, AppState>) -> CmdResult<()> {
    state
        .db
        .delete_catalog(&id, should_queue(&state.db))
        .map_err(err)
}

#[tauri::command]
fn delete_catalog_bulk(ids: Vec<String>, state: State<'_, AppState>) -> CmdResult<usize> {
    if ids.is_empty() {
        return Ok(0);
    }
    state
        .db
        .delete_catalog_bulk(&ids, should_queue(&state.db))
        .map_err(err)
}

fn load_settings(db: &Database) -> AppSettings {
    AppSettings {
        public_sheet_url: db
            .setting("public_sheet_url")
            .ok()
            .flatten()
            .unwrap_or_default(),
        apps_script_url: db
            .setting("apps_script_url")
            .ok()
            .flatten()
            .unwrap_or_default(),
        download_folder: db
            .setting("download_folder")
            .ok()
            .flatten()
            .unwrap_or_else(default_download),
        machine_label: db
            .setting("machine_label")
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                std::env::var("COMPUTERNAME")
                    .or_else(|_| std::env::var("HOSTNAME"))
                    .unwrap_or_else(|_| "This computer".into())
            }),
        recursive_installer_search: db
            .setting("recursive_installer_search")
            .ok()
            .flatten()
            .is_some_and(|v| v == "true"),
        has_shared_secret: !get_secret().is_empty(),
    }
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> AppSettings {
    load_settings(&state.db)
}

#[tauri::command]
fn get_sync_state(state: State<'_, AppState>) -> CmdResult<SyncState> {
    Ok(SyncState {
        configured: should_queue(&state.db) && !get_secret().is_empty(),
        pending_changes: state.db.pending_count().map_err(err)?,
        last_synced_at: state
            .db
            .setting("last_synced_at")
            .map_err(err)?
            .unwrap_or_default(),
    })
}

#[tauri::command]
fn prepare_apps_script() -> CmdResult<AppsScriptSetup> {
    let current = get_secret();
    let secret_created = current.is_empty();
    let secret = if secret_created {
        format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        )
    } else {
        current
    };
    if secret_created {
        secret_entry()?.set_password(&secret).map_err(err)?;
    }
    Ok(AppsScriptSetup {
        script: APPS_SCRIPT_TEMPLATE.replace("REPLACE_WITH_YOUR_OWN_SECRET", &secret),
        secret_created,
        source_url: APPS_SCRIPT_SOURCE_URL.into(),
    })
}

#[tauri::command]
fn save_settings(
    mut settings: AppSettings,
    shared_secret: Option<String>,
    state: State<'_, AppState>,
) -> CmdResult<AppSettings> {
    let device_name = settings.machine_label.trim().to_string();
    if device_name.is_empty() {
        return Err("Device name is required".into());
    }
    settings.machine_label = device_name.clone();
    let previous_device_name = load_settings(&state.db).machine_label;
    for (k, v) in [
        ("public_sheet_url", settings.public_sheet_url.as_str()),
        ("apps_script_url", settings.apps_script_url.as_str()),
        ("download_folder", settings.download_folder.as_str()),
        ("machine_label", settings.machine_label.as_str()),
    ] {
        state.db.set_setting(k, v).map_err(err)?;
    }
    state
        .db
        .set_setting(
            "recursive_installer_search",
            if settings.recursive_installer_search {
                "true"
            } else {
                "false"
            },
        )
        .map_err(err)?;
    if let Some(secret) = shared_secret.filter(|s| !s.is_empty()) {
        secret_entry()?.set_password(&secret).map_err(err)?;
    }
    if previous_device_name != device_name {
        let id = machine_id(&state.db)?;
        state
            .db
            .rename_machine(&id, &device_name, should_queue(&state.db))
            .map_err(err)?;
    }
    Ok(load_settings(&state.db))
}

#[tauri::command]
async fn import_public_sheet(url: String, state: State<'_, AppState>) -> CmdResult<CatalogBundle> {
    let bundle = public_sheet::import(&state.http, &url).await.map_err(err)?;
    state
        .db
        .set_setting("public_sheet_url", &url)
        .map_err(err)?;
    merge::import_bundle(&state.db, bundle).map_err(err)
}

fn configured_client(db: &Database) -> CmdResult<SheetsClient> {
    let url = db
        .setting("apps_script_url")
        .map_err(err)?
        .filter(|v| !v.is_empty())
        .ok_or("Apps Script URL is not configured")?;
    let token = get_secret();
    if token.is_empty() {
        return Err("Apps Script shared secret is not configured".into());
    }
    Ok(SheetsClient::new(url, token))
}

#[tauri::command]
async fn test_apps_script_connection(state: State<'_, AppState>) -> CmdResult<PingResponse> {
    configured_client(&state.db)?.ping().await.map_err(err)
}

#[tauri::command]
async fn sync_now(state: State<'_, AppState>) -> CmdResult<SyncResult> {
    merge::sync_now(&state.db, &configured_client(&state.db)?)
        .await
        .map_err(err)
}

#[tauri::command]
async fn push_entire_library(state: State<'_, AppState>) -> CmdResult<SyncResult> {
    configured_client(&state.db)?;
    state.db.queue_full_library().map_err(err)?;
    merge::sync_now(&state.db, &configured_client(&state.db)?)
        .await
        .map_err(err)
}

#[tauri::command]
fn detect_installed_software() -> CmdResult<Vec<DetectedEntry>> {
    detect::detect_installed_software().map_err(err)
}

#[tauri::command]
fn add_detected_entry(db: &Database, detected: DetectedEntry) -> CmdResult<bool> {
    let existing = db
        .list_catalog()
        .map_err(err)?
        .into_iter()
        .find(|e| e.name.eq_ignore_ascii_case(&detected.name));
    let created = existing.is_none();
    let now = now_rfc3339();
    let software_id = if let Some(e) = existing {
        e.id
    } else {
        let e = CatalogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            name: detected.name.clone(),
            publisher: detected.publisher.clone(),
            platforms: std::env::consts::OS.into(),
            source_type: detected.source_type.clone(),
            source_id: detected.source_id.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            ..Default::default()
        };
        let id = e.id.clone();
        db.upsert_catalog(&e, should_queue(db)).map_err(err)?;
        id
    };
    let mid = machine_id(db)?;
    let inst = InstallationEntry {
        id: format!("{software_id}-{mid}"),
        software_id,
        machine_id: mid,
        machine_label: load_settings(db).machine_label,
        installed: true,
        installed_version: detected.version,
        installed_path: detected.installed_path,
        installer_file_path: String::new(),
        last_verified_at: now.clone(),
        updated_at: now,
    };
    db.upsert_installation(&inst, should_queue(db))
        .map_err(err)?;
    Ok(created)
}

#[tauri::command]
fn add_detected_software(
    detected: DetectedEntry,
    state: State<'_, AppState>,
) -> CmdResult<CatalogBundle> {
    add_detected_entry(&state.db, detected)?;
    state.db.list_bundle().map_err(err)
}

#[tauri::command]
fn add_detected_software_bulk(
    detected: Vec<DetectedEntry>,
    state: State<'_, AppState>,
) -> CmdResult<BulkAddResult> {
    let installations_recorded = detected.len();
    let mut catalog_added = 0;
    for item in detected {
        catalog_added += usize::from(add_detected_entry(&state.db, item)?);
    }
    let bundle = state.db.list_bundle().map_err(err)?;
    Ok(BulkAddResult {
        installations_recorded,
        bundle,
        catalog_added,
    })
}

#[tauri::command]
async fn download_selected(
    ids: Vec<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<Vec<DownloadOutcome>> {
    let folder = PathBuf::from(load_settings(&state.db).download_folder);
    let mid = machine_id(&state.db)?;
    let mut outcomes = Vec::new();
    for id in ids {
        let entry = state.db.get_catalog(&id).map_err(err)?;
        match downloader::download(&state.http, &app, &entry, &folder).await {
            Ok(o) => {
                if o.checksum_ok {
                    let now = now_rfc3339();
                    let old = state
                        .db
                        .list_installations()
                        .map_err(err)?
                        .into_iter()
                        .find(|i| i.software_id == id && i.machine_id == mid);
                    let inst = InstallationEntry {
                        id: old
                            .as_ref()
                            .map(|i| i.id.clone())
                            .unwrap_or_else(|| format!("{id}-{mid}")),
                        software_id: id.clone(),
                        machine_id: mid.clone(),
                        machine_label: load_settings(&state.db).machine_label,
                        installer_file_path: o.path.clone(),
                        updated_at: now,
                        ..old.unwrap_or_default()
                    };
                    state
                        .db
                        .upsert_installation(&inst, should_queue(&state.db))
                        .map_err(err)?;
                }
                outcomes.push(o)
            }
            Err(e) => outcomes.push(DownloadOutcome {
                software_id: id,
                path: String::new(),
                checksum_ok: false,
                error: Some(e.to_string()),
            }),
        }
    }
    Ok(outcomes)
}

#[tauri::command]
async fn install_selected(
    ids: Vec<String>,
    state: State<'_, AppState>,
) -> CmdResult<Vec<InstallRun>> {
    let mid = machine_id(&state.db)?;
    let mut runs = Vec::new();
    for id in ids {
        let entry = state.db.get_catalog(&id).map_err(err)?;
        let old = state
            .db
            .list_installations()
            .map_err(err)?
            .into_iter()
            .find(|i| i.software_id == id && i.machine_id == mid);
        let path = old
            .as_ref()
            .map(|i| i.installer_file_path.as_str())
            .filter(|p| !p.is_empty());
        match plan::build_install_plan(&entry, path) {
            Ok(plan) => match executor::execute(plan).await {
                Ok(outcome) => {
                    if outcome.success && !outcome.launched {
                        let now = now_rfc3339();
                        let inst = InstallationEntry {
                            id: old
                                .as_ref()
                                .map(|i| i.id.clone())
                                .unwrap_or_else(|| format!("{id}-{mid}")),
                            software_id: id.clone(),
                            machine_id: mid.clone(),
                            machine_label: load_settings(&state.db).machine_label,
                            installed: true,
                            last_verified_at: now.clone(),
                            updated_at: now,
                            ..old.unwrap_or_default()
                        };
                        state
                            .db
                            .upsert_installation(&inst, should_queue(&state.db))
                            .map_err(err)?;
                    }
                    runs.push(InstallRun {
                        software_id: id,
                        outcome: Some(outcome),
                        error: None,
                    })
                }
                Err(e) => runs.push(InstallRun {
                    software_id: id,
                    outcome: None,
                    error: Some(e.to_string()),
                }),
            },
            Err(e) => runs.push(InstallRun {
                software_id: id,
                outcome: None,
                error: Some(e.to_string()),
            }),
        }
    }
    Ok(runs)
}

#[tauri::command]
fn find_installer_files(state: State<'_, AppState>) -> CmdResult<Vec<FileSuggestion>> {
    let settings = load_settings(&state.db);
    let mid = machine_id(&state.db)?;
    let linked: std::collections::HashSet<_> = state
        .db
        .list_installations()
        .map_err(err)?
        .into_iter()
        .filter(|i| i.machine_id == mid && !i.installer_file_path.is_empty())
        .map(|i| i.software_id)
        .collect();
    let entries: Vec<_> = state
        .db
        .list_catalog()
        .map_err(err)?
        .into_iter()
        .filter(|e| !linked.contains(&e.id))
        .collect();
    file_finder::find(
        &entries,
        &PathBuf::from(settings.download_folder),
        settings.recursive_installer_search,
    )
    .map_err(err)
}

#[tauri::command]
fn confirm_installer_file(
    software_id: String,
    path: String,
    state: State<'_, AppState>,
) -> CmdResult<CatalogBundle> {
    if !PathBuf::from(&path).is_file() {
        return Err("The selected installer file no longer exists".into());
    }
    let mid = machine_id(&state.db)?;
    let old = state
        .db
        .list_installations()
        .map_err(err)?
        .into_iter()
        .find(|i| i.software_id == software_id && i.machine_id == mid);
    let now = now_rfc3339();
    let inst = InstallationEntry {
        id: old
            .as_ref()
            .map(|i| i.id.clone())
            .unwrap_or_else(|| format!("{software_id}-{mid}")),
        software_id,
        machine_id: mid,
        machine_label: load_settings(&state.db).machine_label,
        installer_file_path: path,
        updated_at: now,
        ..old.unwrap_or_default()
    };
    state
        .db
        .upsert_installation(&inst, should_queue(&state.db))
        .map_err(err)?;
    state.db.list_bundle().map_err(err)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            let state = AppState {
                db: Database::new(data.join("installor.sqlite3"))
                    .map_err(|e| std::io::Error::other(e.to_string()))?,
                http: reqwest::Client::builder()
                    .user_agent("Installor/0.2")
                    .build()
                    .map_err(|e| std::io::Error::other(e.to_string()))?,
            };
            let _ = machine_id(&state.db);
            let background = state.clone();
            let handle = app.handle().clone();
            app.manage(state);
            if background
                .db
                .setting("apps_script_url")
                .ok()
                .flatten()
                .is_some_and(|v| !v.is_empty())
            {
                tauri::async_runtime::spawn(async move {
                    if let Ok(client) = configured_client(&background.db)
                        && merge::sync_now(&background.db, &client).await.is_ok()
                    {
                        let _ = handle.emit("sync-complete", ());
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_catalog,
            save_catalog,
            delete_catalog,
            delete_catalog_bulk,
            get_settings,
            get_sync_state,
            prepare_apps_script,
            save_settings,
            import_public_sheet,
            test_apps_script_connection,
            sync_now,
            push_entire_library,
            detect_installed_software,
            add_detected_software,
            add_detected_software_bulk,
            download_selected,
            install_selected,
            find_installer_files,
            confirm_installer_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running Installor");
}
