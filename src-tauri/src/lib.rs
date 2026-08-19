mod authentication;
mod credentials;
mod error;
mod local_sources;
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
    AccountConnection, AccountSnapshot, AppSettings, AuthenticationAttempt,
    AuthenticationDetection, AuthenticationMode, CredentialOwner, FetchResult, FetchStrategy,
    ProviderDescriptor, SaveAccountRequest, UpdateAccountRequest,
};
use storage::AppDatabase;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State, WebviewWindow, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;

#[tauri::command]
fn provider_descriptors() -> Vec<ProviderDescriptor> {
    providers::descriptors()
}

#[tauri::command]
fn list_account_snapshots(database: State<'_, AppDatabase>) -> AppResult<Vec<AccountSnapshot>> {
    database.list_snapshots()
}

#[tauri::command]
async fn detect_authentication(provider_id: String) -> AppResult<AuthenticationDetection> {
    authentication::detect(&provider_id).await
}

#[tauri::command]
async fn start_codex_login(
    use_device_code: bool,
    authentication: State<'_, authentication::AuthenticationState>,
) -> AppResult<AuthenticationAttempt> {
    authentication::start_codex_login(authentication.inner().clone(), use_device_code).await
}

#[tauri::command]
async fn start_openrouter_login(
    authentication: State<'_, authentication::AuthenticationState>,
) -> AppResult<AuthenticationAttempt> {
    authentication::start_openrouter_pkce(authentication.inner().clone()).await
}

#[tauri::command]
fn poll_authentication(
    attempt_id: String,
    authentication: State<'_, authentication::AuthenticationState>,
) -> AppResult<AuthenticationAttempt> {
    authentication.poll(&attempt_id)
}

#[tauri::command]
fn cancel_authentication(
    attempt_id: String,
    authentication: State<'_, authentication::AuthenticationState>,
) -> AppResult<()> {
    authentication.cancel(&attempt_id)
}

#[tauri::command]
async fn test_provider(
    provider_id: String,
    credential: String,
    credential_kind: Option<String>,
) -> AppResult<FetchResult> {
    if providers::requires_secret(&provider_id) {
        validate_secret_input(&credential)?;
    }
    providers::fetch(&provider_id, &credential, credential_kind.as_deref()).await
}

#[tauri::command]
async fn save_account(
    request: SaveAccountRequest,
    database: State<'_, AppDatabase>,
) -> AppResult<AccountSnapshot> {
    if providers::requires_secret(&request.provider_id) {
        validate_secret_input(&request.credential)?;
    }
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
    let credential_ref = if providers::requires_secret(&request.provider_id) {
        let reference = format!("{}-{account_id}", request.provider_id);
        credentials::store(&reference, &request.credential)?;
        Some(reference)
    } else {
        None
    };
    let auth_mode = match request.provider_id.as_str() {
        "chatgpt-codex"
            if request.authentication_mode == Some(AuthenticationMode::ProviderOauth) =>
        {
            AuthenticationMode::ProviderOauth
        }
        "chatgpt-codex" => AuthenticationMode::SharedLocalSession,
        "google-gemini-cli" => AuthenticationMode::LocalCliOauth,
        _ => AuthenticationMode::PastedSecret,
    };
    let credential_owner = match request.provider_id.as_str() {
        "chatgpt-codex" => CredentialOwner::Codex,
        "google-gemini-cli" => CredentialOwner::GeminiCli,
        _ => CredentialOwner::Dashboard,
    };
    let identity_label = clean_optional(request.identity_label);
    let identity_fingerprint = identity_label
        .as_deref()
        .map(authentication::identity_fingerprint);
    let account = AccountConnection {
        id: account_id.clone(),
        provider_id: request.provider_id,
        display_name: request.display_name.trim().to_owned(),
        credential_ref: credential_ref.clone(),
        credential_kind: request.credential_kind,
        enabled: true,
        source: descriptor.strategy,
        scope: result
            .metrics
            .first()
            .map(|metric| metric.scope.clone())
            .unwrap_or_else(|| "Account".into()),
        account_type: descriptor.account_type,
        plan_name: clean_optional(request.plan_name),
        monthly_price: clean_optional(request.monthly_price),
        renewal_date: clean_optional(request.renewal_date),
        auth_mode,
        credential_owner,
        identity_label,
        identity_fingerprint,
        consented_at: chrono::Utc::now().to_rfc3339(),
        last_validated_at: Some(result.observed_at.clone()),
    };
    if let Err(error) = database
        .save_account(&account)
        .and_then(|_| database.save_fetch_result(&account_id, &result))
    {
        let _ = database.delete_account(&account_id);
        if let Some(reference) = credential_ref {
            let _ = credentials::delete(&reference);
        }
        return Err(error);
    }
    database.snapshot(&account_id)
}

#[tauri::command]
async fn save_openrouter_oauth_account(
    attempt_id: String,
    display_name: String,
    authentication: State<'_, authentication::AuthenticationState>,
    database: State<'_, AppDatabase>,
) -> AppResult<AccountSnapshot> {
    if display_name.trim().is_empty() {
        return Err(AppError::InvalidRequest("account name is required".into()));
    }
    let pending_reference = authentication.take_openrouter_credential(&attempt_id)?;
    let secret = match credentials::read(&pending_reference) {
        Ok(value) => value,
        Err(error) => {
            let _ = credentials::delete(&pending_reference);
            return Err(error);
        }
    };
    let result = match providers::fetch("openrouter", &secret, Some("api_key")).await {
        Ok(value) => value,
        Err(error) => {
            drop(secret);
            let _ = credentials::delete(&pending_reference);
            return Err(error);
        }
    };
    let account_id = uuid::Uuid::new_v4().to_string();
    let final_reference = format!("openrouter-{account_id}");
    if let Err(error) = credentials::store(&final_reference, &secret) {
        drop(secret);
        let _ = credentials::delete(&pending_reference);
        return Err(error);
    }
    drop(secret);
    let account = AccountConnection {
        id: account_id.clone(),
        provider_id: "openrouter".into(),
        display_name: display_name.trim().into(),
        credential_ref: Some(final_reference.clone()),
        credential_kind: Some("api_key".into()),
        enabled: true,
        source: FetchStrategy::OfficialApi,
        scope: result
            .metrics
            .first()
            .map(|metric| metric.scope.clone())
            .unwrap_or_else(|| "API key".into()),
        account_type: models::AccountType::ApiPlatform,
        plan_name: None,
        monthly_price: None,
        renewal_date: None,
        auth_mode: AuthenticationMode::ProviderOauth,
        credential_owner: CredentialOwner::Dashboard,
        identity_label: Some("OpenRouter user-controlled key".into()),
        identity_fingerprint: None,
        consented_at: chrono::Utc::now().to_rfc3339(),
        last_validated_at: Some(result.observed_at.clone()),
    };
    let saved = database
        .save_account(&account)
        .and_then(|_| database.save_fetch_result(&account_id, &result));
    if let Err(error) = saved {
        let _ = database.delete_account(&account_id);
        let _ = credentials::delete(&final_reference);
        let _ = credentials::delete(&pending_reference);
        return Err(error);
    }
    let _ = credentials::delete(&pending_reference);
    database.snapshot(&account_id)
}

#[tauri::command]
async fn refresh_account(account_id: String, app: AppHandle) -> AppResult<AccountSnapshot> {
    let _ = refresh_one(&app, &account_id).await;
    app.state::<AppDatabase>().snapshot(&account_id)
}

#[tauri::command]
fn set_account_enabled(
    account_id: String,
    enabled: bool,
    database: State<'_, AppDatabase>,
) -> AppResult<AccountSnapshot> {
    database.set_account_enabled(&account_id, enabled)?;
    database.snapshot(&account_id)
}

#[tauri::command]
async fn update_account(
    request: UpdateAccountRequest,
    app: AppHandle,
) -> AppResult<AccountSnapshot> {
    if request.display_name.trim().is_empty() {
        return Err(AppError::InvalidRequest("account name is required".into()));
    }
    let database = app.state::<AppDatabase>();
    let mut account = database
        .get_account(&request.account_id)?
        .ok_or_else(|| AppError::InvalidRequest("account not found".into()))?;
    let original_account = account.clone();
    let credential = request
        .credential
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let requires_secret = providers::requires_secret(&account.provider_id);
    let credential_kind_changed =
        requires_secret && request.credential_kind != account.credential_kind;
    if credential_kind_changed && credential.is_none() {
        return Err(AppError::InvalidRequest(
            "a new credential is required when changing credential scope".into(),
        ));
    }
    if requires_secret {
        if let Some(value) = credential {
            validate_secret_input(value)?;
        }
    } else if credential.is_some() {
        return Err(AppError::InvalidRequest(
            "local CLI/OAuth accounts do not accept pasted credentials".into(),
        ));
    }

    let result = if let Some(value) = credential {
        Some(
            providers::fetch(
                &account.provider_id,
                value,
                request.credential_kind.as_deref(),
            )
            .await?,
        )
    } else {
        None
    };
    let old_reference = account.credential_ref.clone();
    let new_reference = credential.map(|value| {
        let reference = format!("{}-{}", account.provider_id, uuid::Uuid::new_v4());
        (reference, value)
    });
    if let Some((reference, value)) = &new_reference {
        credentials::store(reference, value)?;
        account.credential_ref = Some(reference.clone());
    }
    account.display_name = request.display_name.trim().into();
    account.credential_kind = request.credential_kind;
    account.plan_name = clean_optional(request.plan_name);
    account.monthly_price = clean_optional(request.monthly_price);
    account.renewal_date = clean_optional(request.renewal_date);
    if let Some(result) = &result {
        account.scope = result
            .metrics
            .first()
            .map(|metric| metric.scope.clone())
            .unwrap_or_else(|| "Account".into());
    }
    let saved = database.save_account(&account).and_then(|_| match &result {
        Some(value) => database.save_fetch_result(&account.id, value),
        None => Ok(()),
    });
    if let Err(error) = saved {
        let _ = database.save_account(&original_account);
        if let Some((reference, _)) = &new_reference {
            let _ = credentials::delete(reference);
        }
        return Err(error);
    }
    if new_reference.is_some() {
        if let Some(reference) = old_reference {
            let _ = credentials::delete(&reference);
        }
    }
    database.snapshot(&account.id)
}

#[tauri::command]
fn get_app_settings(app: AppHandle, database: State<'_, AppDatabase>) -> AppResult<AppSettings> {
    let mut settings = database.get_settings()?;
    settings.start_on_login = app
        .autolaunch()
        .is_enabled()
        .map_err(|error| AppError::InvalidRequest(format!("autostart error: {error}")))?;
    database.save_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
fn update_app_settings(
    settings: AppSettings,
    app: AppHandle,
    database: State<'_, AppDatabase>,
) -> AppResult<AppSettings> {
    validate_settings(&settings)?;
    let autolaunch = app.autolaunch();
    if settings.start_on_login {
        autolaunch
            .enable()
            .map_err(|error| AppError::InvalidRequest(format!("autostart error: {error}")))?;
    } else {
        autolaunch
            .disable()
            .map_err(|error| AppError::InvalidRequest(format!("autostart error: {error}")))?;
    }
    database.save_settings(&settings)?;
    database.prune_history(settings.history_retention_days)?;
    Ok(settings)
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

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
}

fn validate_settings(settings: &AppSettings) -> AppResult<()> {
    if ![5, 15, 30, 60].contains(&settings.api_refresh_minutes)
        || ![15, 30, 60, 120].contains(&settings.local_refresh_minutes)
        || ![30, 60, 90, 180].contains(&settings.history_retention_days)
    {
        return Err(AppError::InvalidRequest(
            "unsupported settings value".into(),
        ));
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
    let result = async {
        if providers::requires_secret(&account.provider_id) {
            let reference = account
                .credential_ref
                .as_deref()
                .ok_or_else(|| AppError::InvalidRequest("credential reference missing".into()))?;
            let secret = credentials::read(reference)?;
            let value = providers::fetch(
                &account.provider_id,
                &secret,
                account.credential_kind.as_deref(),
            )
            .await;
            drop(secret);
            value
        } else {
            providers::fetch(&account.provider_id, "", account.credential_kind.as_deref()).await
        }
    }
    .await;
    match &result {
        Ok(value) => database.save_fetch_result(account_id, value)?,
        Err(error) => database.record_refresh_failure(account_id, &error.to_string())?,
    }
    if let Ok(snapshot) = database.snapshot(account_id) {
        let _ = app.emit("account-updated", snapshot);
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
            let minimize = close_window
                .app_handle()
                .state::<AppDatabase>()
                .get_settings()
                .map(|settings| settings.minimize_to_tray)
                .unwrap_or(true);
            api.prevent_close();
            if minimize {
                let _ = close_window.hide();
            } else {
                close_window.app_handle().exit(0);
            }
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
                        let settings = task_app
                            .state::<AppDatabase>()
                            .get_settings()
                            .unwrap_or_default();
                        let minutes = match account.source {
                            FetchStrategy::OfficialApi => settings.api_refresh_minutes,
                            _ => settings.local_refresh_minutes,
                        };
                        (Duration::from_secs(u64::from(minutes) * 60), 0)
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
            app.manage(authentication::AuthenticationState::default());
            configure_tray(app)?;
            let settings = app.state::<AppDatabase>().get_settings()?;
            if settings.start_on_login && !app.autolaunch().is_enabled()? {
                app.autolaunch().enable()?;
            } else if !settings.start_on_login && app.autolaunch().is_enabled()? {
                app.autolaunch().disable()?;
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
            list_account_snapshots,
            detect_authentication,
            start_codex_login,
            start_openrouter_login,
            poll_authentication,
            cancel_authentication,
            test_provider,
            save_account,
            save_openrouter_oauth_account,
            refresh_account,
            set_account_enabled,
            update_account,
            get_app_settings,
            update_app_settings,
            delete_account,
            delete_local_data
        ])
        .run(tauri::generate_context!())
        .expect("error while running AI Usage Dashboard");
}
