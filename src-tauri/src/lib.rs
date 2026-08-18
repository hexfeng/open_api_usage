mod credentials;
mod error;
mod models;
mod providers;
mod storage;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use error::{AppError, AppResult};
use models::{
    AccountConnection, FetchResult, FetchStrategy, ProviderDescriptor, SaveAccountRequest,
};
use storage::AppDatabase;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, State, WebviewWindow, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;

#[tauri::command]
fn provider_descriptors() -> Vec<ProviderDescriptor> {
    providers::descriptors()
}

#[tauri::command]
fn list_accounts(database: State<'_, AppDatabase>) -> AppResult<Vec<AccountConnection>> {
    database.list_accounts()
}

#[tauri::command]
async fn test_provider(
    provider_id: String,
    credential: String,
    credential_kind: Option<String>,
) -> AppResult<FetchResult> {
    validate_secret_input(&credential)?;
    providers::fetch(&provider_id, &credential, credential_kind.as_deref()).await
}

#[tauri::command]
async fn save_account(
    request: SaveAccountRequest,
    database: State<'_, AppDatabase>,
) -> AppResult<AccountConnection> {
    validate_secret_input(&request.credential)?;
    if request.display_name.trim().is_empty() {
        return Err(AppError::InvalidRequest("account name is required".into()));
    }
    let descriptor = providers::descriptors()
        .into_iter()
        .find(|item| item.id == request.provider_id)
        .ok_or_else(|| AppError::UnsupportedProvider(request.provider_id.clone()))?;
    let result = providers::fetch(
        &request.provider_id,
        &request.credential,
        request.credential_kind.as_deref(),
    )
    .await?;
    let account_id = uuid::Uuid::new_v4().to_string();
    let credential_ref = format!("{}-{account_id}", request.provider_id);
    credentials::store(&credential_ref, &request.credential)?;
    let account = AccountConnection {
        id: account_id.clone(),
        provider_id: request.provider_id,
        display_name: request.display_name.trim().to_owned(),
        credential_ref: Some(credential_ref.clone()),
        credential_kind: request.credential_kind,
        enabled: true,
        source: descriptor.strategy,
        scope: result
            .metrics
            .first()
            .map(|metric| metric.scope.clone())
            .unwrap_or_else(|| "Account".into()),
    };
    if let Err(error) = database
        .save_account(&account)
        .and_then(|_| database.save_fetch_result(&account_id, &result))
    {
        let _ = credentials::delete(&credential_ref);
        return Err(error);
    }
    Ok(account)
}

#[tauri::command]
async fn refresh_account(account_id: String, app: AppHandle) -> AppResult<FetchResult> {
    refresh_one(&app, &account_id).await
}

#[tauri::command]
fn delete_account(account_id: String, database: State<'_, AppDatabase>) -> AppResult<()> {
    if let Some(reference) = database.delete_account(&account_id)? {
        credentials::delete(&reference)?;
    }
    Ok(())
}

#[tauri::command]
fn delete_local_data(database: State<'_, AppDatabase>) -> AppResult<()> {
    for account in database.list_accounts()? {
        if let Some(reference) = account.credential_ref {
            credentials::delete(&reference)?;
        }
    }
    database.clear()
}

fn validate_secret_input(value: &str) -> AppResult<()> {
    if value.trim().len() < 8 {
        return Err(AppError::InvalidRequest("credential is too short".into()));
    }
    Ok(())
}

async fn refresh_one(app: &AppHandle, account_id: &str) -> AppResult<FetchResult> {
    let database = app.state::<AppDatabase>();
    let account = database
        .list_accounts()?
        .into_iter()
        .find(|item| item.id == account_id)
        .ok_or_else(|| AppError::InvalidRequest("account not found".into()))?;
    let reference = account
        .credential_ref
        .as_deref()
        .ok_or_else(|| AppError::InvalidRequest("credential reference missing".into()))?;
    let secret = credentials::read(reference)?;
    let result = providers::fetch(
        &account.provider_id,
        &secret,
        account.credential_kind.as_deref(),
    )
    .await;
    drop(secret);
    if let Ok(value) = &result {
        database.save_fetch_result(account_id, value)?;
    }
    result
}

fn configure_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItem::with_id(app, "open", "Open dashboard", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().expect("default icon").clone())
        .menu(&menu)
        .tooltip("AI Usage Dashboard")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn minimize_to_tray(window: &WebviewWindow) {
    let close_window = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = close_window.hide();
        }
    });
}

fn start_scheduler(app: AppHandle) {
    let schedule: Arc<Mutex<HashMap<String, (Instant, u32)>>> =
        Arc::new(Mutex::new(HashMap::new()));
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let accounts = match app.state::<AppDatabase>().list_accounts() {
                Ok(accounts) => accounts,
                Err(error) => {
                    log::warn!("scheduler could not list accounts: {error}");
                    continue;
                }
            };
            for account in accounts.into_iter().filter(|account| account.enabled) {
                let is_due = schedule
                    .lock()
                    .expect("schedule mutex poisoned")
                    .get(&account.id)
                    .map(|(due, _)| Instant::now() >= *due)
                    .unwrap_or(true);
                if !is_due {
                    continue;
                }
                let task_app = app.clone();
                let task_schedule = schedule.clone();
                tauri::async_runtime::spawn(async move {
                    let result = refresh_one(&task_app, &account.id).await;
                    let mut schedule = task_schedule.lock().expect("schedule mutex poisoned");
                    let previous_attempts = schedule
                        .get(&account.id)
                        .map(|(_, attempts)| *attempts)
                        .unwrap_or(0);
                    let (delay, attempts) = if result.is_ok() {
                        let minutes = match account.source {
                            FetchStrategy::OfficialApi => 15,
                            _ => 30,
                        };
                        (Duration::from_secs(minutes * 60), 0)
                    } else {
                        let attempts = previous_attempts.saturating_add(1);
                        let seconds = 60_u64.saturating_mul(2_u64.saturating_pow(attempts.min(4)));
                        (Duration::from_secs(seconds.min(15 * 60)), attempts)
                    };
                    schedule.insert(account.id, (Instant::now() + delay, attempts));
                });
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--background"])
                .build(),
        )
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .filter(|metadata| {
                    !metadata.target().contains("credential")
                        && !metadata.target().contains("cookie")
                })
                .build(),
        )
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            app.manage(AppDatabase::open(
                &data_dir.join("ai-usage-dashboard.sqlite3"),
            )?);
            configure_tray(app)?;
            if !app.autolaunch().is_enabled()? {
                app.autolaunch().enable()?;
            }
            if let Some(window) = app.get_webview_window("main") {
                minimize_to_tray(&window);
                if std::env::args().any(|argument| argument == "--background") {
                    let _ = window.hide();
                }
            }
            start_scheduler(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            provider_descriptors,
            list_accounts,
            test_provider,
            save_account,
            refresh_account,
            delete_account,
            delete_local_data
        ])
        .run(tauri::generate_context!())
        .expect("error while running AI Usage Dashboard");
}
