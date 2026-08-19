use std::{collections::HashSet, path::PathBuf, process::Stdio, time::SystemTime};

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    time::{timeout, Duration},
};

use crate::{
    credentials,
    error::{AppError, AppResult},
    models::{
        AuthenticationAvailability, AuthenticationDetection, AuthenticationMode, CredentialOwner,
        FetchResult, FetchStrategy, Metric,
    },
};

const LOCAL_SOURCE_TIMEOUT: Duration = Duration::from_secs(20);

pub async fn detect_codex() -> AppResult<AuthenticationDetection> {
    let account = codex_account_read().await?;
    let Some(account) = account.get("account").filter(|value| !value.is_null()) else {
        return Ok(AuthenticationDetection {
            provider_id: "chatgpt-codex".into(),
            availability: AuthenticationAvailability::AuthenticationRequired,
            authentication_mode: AuthenticationMode::SharedLocalSession,
            credential_owner: CredentialOwner::Codex,
            identity_label: None,
            plan_label: None,
            scope: "Codex only".into(),
            diagnostic: "Codex is installed but no ChatGPT subscription session is active.".into(),
        });
    };
    if account.get("type").and_then(Value::as_str) != Some("chatgpt") {
        return Ok(AuthenticationDetection {
            provider_id: "chatgpt-codex".into(),
            availability: AuthenticationAvailability::AuthenticationRequired,
            authentication_mode: AuthenticationMode::SharedLocalSession,
            credential_owner: CredentialOwner::Codex,
            identity_label: None,
            plan_label: None,
            scope: "Codex only".into(),
            diagnostic: "Codex is not signed in with a ChatGPT subscription account.".into(),
        });
    }
    Ok(codex_detection(account))
}

pub(crate) fn codex_detection(account: &Value) -> AuthenticationDetection {
    AuthenticationDetection {
        provider_id: "chatgpt-codex".into(),
        availability: AuthenticationAvailability::ExistingSession,
        authentication_mode: AuthenticationMode::SharedLocalSession,
        credential_owner: CredentialOwner::Codex,
        identity_label: account.get("email").and_then(Value::as_str).map(str::to_owned),
        plan_label: account
            .get("planType")
            .and_then(Value::as_str)
            .map(str::to_owned),
        scope: "Codex only".into(),
        diagnostic: "Shared Codex session detected. Confirm this identity before monitoring; removing it from the dashboard will not sign out Codex.".into(),
    }
}

async fn codex_account_read() -> AppResult<Value> {
    let executable = codex_executable();
    let mut command = codex_command(&executable);
    let mut child = command.spawn().map_err(|error| {
        AppError::InvalidRequest(format!(
            "Codex CLI could not be started from {}: {error}",
            executable.display()
        ))
    })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| AppError::InvalidResponse("Codex app-server stdin missing".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::InvalidResponse("Codex app-server stdout missing".into()))?;
    let exchange = timeout(LOCAL_SOURCE_TIMEOUT, async {
        write_codex_messages(
            &mut stdin,
            &[
                initialize_message(),
                json!({"method":"initialized"}),
                json!({"method":"account/read","id":2,"params":{"refreshToken":false}}),
            ],
        )
        .await?;
        let mut lines = BufReader::new(stdout).lines();
        loop {
            let line = lines.next_line().await?.ok_or_else(|| {
                AppError::InvalidResponse("Codex app-server closed before account/read".into())
            })?;
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if message.get("id").and_then(Value::as_i64) == Some(2) {
                break rpc_result(message, "account/read");
            }
        }
    })
    .await;
    let _ = child.kill().await;
    exchange.map_err(|_| AppError::InvalidResponse("Codex account detection timed out".into()))?
}

pub(crate) fn codex_command(executable: &PathBuf) -> Command {
    let mut command = Command::new(executable);
    command
        .args(["-s", "read-only", "-a", "untrusted", "app-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    command
}

pub(crate) fn initialize_message() -> Value {
    json!({"method":"initialize","id":1,"params":{"clientInfo":{"name":"ai_usage_dashboard","title":"AI Usage Dashboard","version":env!("CARGO_PKG_VERSION")}}})
}

pub(crate) async fn write_codex_messages(
    stdin: &mut tokio::process::ChildStdin,
    messages: &[Value],
) -> AppResult<()> {
    for message in messages {
        stdin.write_all(message.to_string().as_bytes()).await?;
        stdin.write_all(b"\n").await?;
    }
    stdin.flush().await?;
    Ok(())
}

pub async fn fetch_codex() -> AppResult<FetchResult> {
    let executable = codex_executable();
    let mut command = codex_command(&executable);

    let mut child = command.spawn().map_err(|error| {
        AppError::InvalidRequest(format!(
            "Codex CLI could not be started from {}: {error}",
            executable.display()
        ))
    })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| AppError::InvalidResponse("Codex app-server stdin missing".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::InvalidResponse("Codex app-server stdout missing".into()))?;

    let exchange = timeout(LOCAL_SOURCE_TIMEOUT, async {
        write_codex_messages(
            &mut stdin,
            &[
                initialize_message(),
                json!({"method":"initialized"}),
                json!({"method":"account/read","id":2,"params":{"refreshToken":false}}),
                json!({"method":"account/rateLimits/read","id":3}),
            ],
        )
        .await?;

        let mut account = None;
        let mut limits = None;
        let mut lines = BufReader::new(stdout).lines();
        while account.is_none() || limits.is_none() {
            let line = lines.next_line().await?.ok_or_else(|| {
                AppError::InvalidResponse(
                    "Codex app-server closed before returning quota data".into(),
                )
            })?;
            let message: Value = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(_) => continue,
            };
            match message.get("id").and_then(Value::as_i64) {
                Some(2) => account = Some(rpc_result(message, "account/read")?),
                Some(3) => limits = Some(rpc_result(message, "account/rateLimits/read")?),
                _ => {}
            }
        }
        Ok::<_, AppError>((
            account.expect("account response"),
            limits.expect("limits response"),
        ))
    })
    .await;
    let _ = child.kill().await;
    let (account, limits) = exchange
        .map_err(|_| AppError::InvalidResponse("Codex app-server quota read timed out".into()))??;
    parse_codex(&account, &limits)
}

pub(crate) fn rpc_result(message: Value, method: &str) -> AppResult<Value> {
    if let Some(error) = message.get("error") {
        let detail = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown app-server error");
        return Err(AppError::InvalidResponse(format!("{method}: {detail}")));
    }
    message
        .get("result")
        .cloned()
        .ok_or_else(|| AppError::InvalidResponse(format!("{method} result missing")))
}

fn parse_codex(account: &Value, response: &Value) -> AppResult<FetchResult> {
    let account = account.get("account").ok_or_else(|| {
        AppError::InvalidResponse("Codex is not signed in with a ChatGPT account".into())
    })?;
    if account.get("type").and_then(Value::as_str) != Some("chatgpt") {
        return Err(AppError::InvalidResponse(
            "Codex is not using a ChatGPT subscription login".into(),
        ));
    }
    let plan = account
        .get("planType")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let observed_at = Utc::now().to_rfc3339();
    let mut metrics = Vec::new();
    let mut seen = HashSet::new();
    let mut limits = Vec::new();
    if let Some(value) = response.get("rateLimits") {
        limits.push(value);
    }
    if let Some(values) = response
        .get("rateLimitsByLimitId")
        .and_then(Value::as_object)
    {
        limits.extend(values.values());
    }
    for limit in limits {
        for window in [limit.get("primary"), limit.get("secondary")]
            .into_iter()
            .flatten()
        {
            let used = window.get("usedPercent").and_then(Value::as_f64);
            let duration = window.get("windowDurationMins").and_then(Value::as_i64);
            let reset = window.get("resetsAt").and_then(Value::as_i64);
            let Some((used, duration)) = used.zip(duration) else {
                continue;
            };
            let identity = format!("{duration}:{}:{used}", reset.unwrap_or_default());
            if !seen.insert(identity) {
                continue;
            }
            let index = metrics.len();
            metrics.push(Metric {
                kind: if index == 0 {
                    "codex_quota_primary".into()
                } else {
                    format!("codex_quota_{}", index + 1)
                },
                value: used,
                unit: "percent_used".into(),
                currency: None,
                scope: "Codex only".into(),
                window: Some(format_window(duration)),
                reset_time: reset.and_then(epoch_rfc3339),
                observed_at: observed_at.clone(),
                source: FetchStrategy::CliOauth,
            });
        }
    }
    if let Some(balance) = response
        .pointer("/rateLimits/credits/balance")
        .and_then(Value::as_f64)
    {
        metrics.push(Metric {
            kind: "codex_credits".into(),
            value: balance,
            unit: "credits".into(),
            currency: None,
            scope: "Codex credits".into(),
            window: None,
            reset_time: None,
            observed_at: observed_at.clone(),
            source: FetchStrategy::CliOauth,
        });
    }
    if metrics.is_empty() {
        return Err(AppError::InvalidResponse(
            "Codex returned no subscription quota windows".into(),
        ));
    }
    Ok(FetchResult {
        provider_id: "chatgpt-codex".into(),
        status: "Live".into(),
        metrics,
        history_buckets: Vec::new(),
        observed_at,
        diagnostic: format!(
            "Codex-only quota from the local app-server account/rateLimits/read endpoint. ChatGPT plan: {plan}."
        ),
    })
}

pub async fn detect_gemini_cli() -> AppResult<AuthenticationDetection> {
    if !gemini_cli_installed().await {
        return Ok(AuthenticationDetection {
            provider_id: "google-gemini-cli".into(),
            availability: AuthenticationAvailability::NotInstalled,
            authentication_mode: AuthenticationMode::LocalCliOauth,
            credential_owner: CredentialOwner::GeminiCli,
            identity_label: None,
            plan_label: None,
            scope: "Gemini CLI only".into(),
            diagnostic: "Gemini CLI is not installed. Install the official CLI, run gemini, and choose Sign in with Google.".into(),
        });
    }
    if gemini_credential_text()?.is_none() {
        return Ok(AuthenticationDetection {
            provider_id: "google-gemini-cli".into(),
            availability: AuthenticationAvailability::AuthenticationRequired,
            authentication_mode: AuthenticationMode::LocalCliOauth,
            credential_owner: CredentialOwner::GeminiCli,
            identity_label: None,
            plan_label: None,
            scope: "Gemini CLI only".into(),
            diagnostic: "Gemini CLI is installed but signed out. Run gemini and choose Sign in with Google, then check again.".into(),
        });
    }
    match gemini_session().await {
        Ok((load, _)) => Ok(AuthenticationDetection {
            provider_id: "google-gemini-cli".into(),
            availability: AuthenticationAvailability::ExistingSession,
            authentication_mode: AuthenticationMode::LocalCliOauth,
            credential_owner: CredentialOwner::GeminiCli,
            identity_label: gemini_identity(&load),
            plan_label: load
                .pointer("/paidTier/name")
                .or_else(|| load.pointer("/currentTier/name"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            scope: "Gemini CLI only · Experimental".into(),
            diagnostic: "Gemini CLI session detected. Confirm this identity before monitoring; the dashboard does not copy its OAuth token or sign out the CLI when removed.".into(),
        }),
        Err(error) => {
            let detail = error.to_string();
            let invalid = detail.contains("HTTP 401")
                || detail.contains("HTTP 403")
                || detail.contains("refresh token missing")
                || detail.contains("access token missing")
                || detail.contains("credential file is invalid");
            Ok(AuthenticationDetection {
                provider_id: "google-gemini-cli".into(),
                availability: if invalid {
                    AuthenticationAvailability::CredentialInvalid
                } else {
                    AuthenticationAvailability::Unavailable
                },
                authentication_mode: AuthenticationMode::LocalCliOauth,
                credential_owner: CredentialOwner::GeminiCli,
                identity_label: None,
                plan_label: None,
                scope: "Gemini CLI only · Experimental".into(),
                diagnostic: if invalid {
                    "Gemini CLI credentials are present but invalid or no longer accepted. Sign in again in the official CLI.".into()
                } else {
                    "Gemini CLI is installed, but its session could not be validated because the provider or Experimental quota contract is unavailable.".into()
                },
            })
        }
    }
}

pub async fn fetch_gemini_cli() -> AppResult<FetchResult> {
    let (load, quota) = gemini_session().await?;
    parse_gemini_cli(&load, &quota)
}

async fn gemini_session() -> AppResult<(Value, Value)> {
    let credential_path = gemini_oauth_path()?;
    let credential_text = gemini_credential_text()?.ok_or_else(|| {
            AppError::InvalidRequest(format!(
                "Gemini CLI OAuth credentials were not found in Windows Credential Manager or at {}; install Gemini CLI and sign in with Google first",
                credential_path.display()
            ))
        })?;
    let credentials: Value = serde_json::from_str(&credential_text).map_err(|_| {
        AppError::InvalidResponse("Gemini CLI OAuth credential file is invalid".into())
    })?;
    drop(credential_text);
    let access_token = gemini_access_token(&credentials).await?;
    let http = Client::builder().timeout(LOCAL_SOURCE_TIMEOUT).build()?;
    let metadata = json!({
        "ideType":"GEMINI_CLI",
        "platform":"WINDOWS_AMD64",
        "pluginType":"GEMINI"
    });
    let load = google_post(
        &http,
        "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist",
        &access_token,
        json!({"metadata":metadata}),
    )
    .await?;
    let project = load
        .get("cloudaicompanionProject")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| std::env::var("GOOGLE_CLOUD_PROJECT").ok())
        .ok_or_else(|| {
            AppError::InvalidResponse(
                "Gemini CLI project was not returned; complete Gemini CLI onboarding first".into(),
            )
        })?;
    let quota = google_post(
        &http,
        "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota",
        &access_token,
        json!({"project":project}),
    )
    .await?;
    drop(access_token);
    Ok((load, quota))
}

fn gemini_credential_text() -> AppResult<Option<String>> {
    if let Some(value) = credentials::read_external("gemini-cli-oauth", "main-account")? {
        return Ok(Some(value));
    }
    let path = gemini_oauth_path()?;
    match std::fs::read_to_string(path) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

async fn gemini_cli_installed() -> bool {
    let executable = if cfg!(windows) {
        "gemini.cmd"
    } else {
        "gemini"
    };
    if matches!(
        timeout(Duration::from_secs(5), Command::new(executable).arg("--version").output()).await,
        Ok(Ok(output)) if output.status.success()
    ) {
        return true;
    }
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let Ok(Ok(output)) = timeout(
        Duration::from_secs(5),
        Command::new(npm).args(["root", "-g"]).output(),
    )
    .await
    else {
        return false;
    };
    output.status.success()
        && PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
            .join("@google")
            .join("gemini-cli")
            .join("package.json")
            .is_file()
}

fn gemini_identity(load: &Value) -> Option<String> {
    [
        "/email",
        "/user/email",
        "/account/email",
        "/currentUser/email",
    ]
    .into_iter()
    .find_map(|path| {
        load.pointer(path)
            .and_then(Value::as_str)
            .map(str::to_owned)
    })
    .or_else(|| {
        load.get("cloudaicompanionProject")
            .and_then(Value::as_str)
            .map(str::to_owned)
    })
}

async fn gemini_access_token(credentials: &Value) -> AppResult<String> {
    let token = credentials.get("token").unwrap_or(credentials);
    let now_ms = Utc::now().timestamp_millis();
    let expiry_ms = token
        .get("expiresAt")
        .or_else(|| token.get("expiry_date"))
        .and_then(Value::as_i64)
        .unwrap_or_default();
    if expiry_ms > now_ms + 60_000 {
        return token
            .get("accessToken")
            .or_else(|| token.get("access_token"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| AppError::InvalidResponse("Gemini CLI access token missing".into()));
    }
    let refresh_token = token
        .get("refreshToken")
        .or_else(|| token.get("refresh_token"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::InvalidResponse("Gemini CLI refresh token missing; sign in again".into())
        })?;
    let (client_id, client_secret) = gemini_oauth_client().await?;
    let response = Client::builder()
        .timeout(LOCAL_SOURCE_TIMEOUT)
        .build()?
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(AppError::InvalidResponse(format!(
            "Gemini CLI OAuth refresh returned HTTP {}",
            response.status()
        )));
    }
    let body: Value = response.json().await?;
    body.get("access_token")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            AppError::InvalidResponse("Gemini CLI OAuth refresh returned no access token".into())
        })
}

async fn gemini_oauth_client() -> AppResult<(String, String)> {
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let output = timeout(
        Duration::from_secs(5),
        Command::new(npm).args(["root", "-g"]).output(),
    )
    .await
    .map_err(|_| AppError::InvalidResponse("Locating Gemini CLI timed out".into()))?
    .map_err(|_| {
        AppError::InvalidRequest(
            "Gemini CLI installation could not be located; install it and sign in first".into(),
        )
    })?;
    if !output.status.success() {
        return Err(AppError::InvalidRequest(
            "Gemini CLI installation could not be located; install it and sign in first".into(),
        ));
    }
    let root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let relative = ["dist", "src", "code_assist", "oauth2.js"];
    let candidates = [
        root.join("@google")
            .join("gemini-cli-core")
            .join(relative.iter().collect::<PathBuf>()),
        root.join("@google")
            .join("gemini-cli")
            .join("node_modules")
            .join("@google")
            .join("gemini-cli-core")
            .join(relative.iter().collect::<PathBuf>()),
    ];
    for path in candidates {
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        let Some(client_id) = javascript_const(&source, "OAUTH_CLIENT_ID") else {
            continue;
        };
        let Some(client_secret) = javascript_const(&source, "OAUTH_CLIENT_SECRET") else {
            continue;
        };
        return Ok((client_id, client_secret));
    }
    Err(AppError::InvalidResponse(
        "Gemini CLI OAuth configuration was not found in the installed official package; update or reinstall Gemini CLI".into(),
    ))
}

fn javascript_const(source: &str, name: &str) -> Option<String> {
    let declaration = format!("const {name}");
    let tail = source.get(source.find(&declaration)? + declaration.len()..)?;
    let assignment = tail.get(tail.find('=')? + 1..)?.trim_start();
    let quote = assignment.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let value = assignment.get(quote.len_utf8()..)?;
    Some(value.get(..value.find(quote)?)?.to_owned())
}

async fn google_post(http: &Client, url: &str, token: &str, body: Value) -> AppResult<Value> {
    let response = http.post(url).bearer_auth(token).json(&body).send().await?;
    if !response.status().is_success() {
        return Err(AppError::InvalidResponse(format!(
            "Gemini CLI quota service returned HTTP {}",
            response.status()
        )));
    }
    Ok(response.json().await?)
}

fn parse_gemini_cli(load: &Value, quota: &Value) -> AppResult<FetchResult> {
    let observed_at = Utc::now().to_rfc3339();
    let buckets = quota
        .get("buckets")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidResponse("Gemini CLI quota buckets missing".into()))?;
    let mut metrics = Vec::new();
    for (index, bucket) in buckets.iter().enumerate() {
        let Some(remaining) = bucket.get("remainingFraction").and_then(Value::as_f64) else {
            continue;
        };
        let model = bucket
            .get("modelId")
            .and_then(Value::as_str)
            .unwrap_or("Gemini model");
        metrics.push(Metric {
            kind: format!("gemini_quota_{}", index + 1),
            value: ((1.0 - remaining.clamp(0.0, 1.0)) * 100.0).round(),
            unit: "percent_used".into(),
            currency: None,
            scope: format!("Gemini CLI · {model}"),
            window: bucket
                .get("tokenType")
                .and_then(Value::as_str)
                .map(str::to_owned),
            reset_time: bucket
                .get("resetTime")
                .and_then(Value::as_str)
                .map(str::to_owned),
            observed_at: observed_at.clone(),
            source: FetchStrategy::CliOauth,
        });
    }
    if metrics.is_empty() {
        return Err(AppError::InvalidResponse(
            "Gemini CLI returned no readable model quota buckets".into(),
        ));
    }
    let plan = load
        .pointer("/paidTier/name")
        .or_else(|| load.pointer("/currentTier/name"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    Ok(FetchResult {
        provider_id: "google-gemini-cli".into(),
        status: "Experimental".into(),
        metrics,
        history_buckets: Vec::new(),
        observed_at,
        diagnostic: format!(
            "Gemini CLI model quota from the local OAuth session and retrieveUserQuota. Plan: {plan}. This does not represent all Google AI subscription benefits."
        ),
    })
}

pub(crate) fn codex_executable() -> PathBuf {
    if let Some(path) = std::env::var_os("CODEX_CLI_PATH").map(PathBuf::from) {
        if path.is_file() {
            return path;
        }
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let bin = PathBuf::from(local)
            .join("OpenAI")
            .join("Codex")
            .join("bin");
        if let Ok(entries) = std::fs::read_dir(bin) {
            let mut candidates = entries
                .flatten()
                .map(|entry| entry.path().join("codex.exe"))
                .filter(|path| path.is_file())
                .collect::<Vec<_>>();
            candidates.sort_by_key(|path| {
                path.metadata()
                    .and_then(|metadata| metadata.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH)
            });
            if let Some(path) = candidates.pop() {
                return path;
            }
        }
    }
    PathBuf::from("codex")
}

fn gemini_oauth_path() -> AppResult<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .map(|home| home.join(".gemini").join("oauth_creds.json"))
        .ok_or_else(|| AppError::InvalidRequest("Windows user profile path is unavailable".into()))
}

fn format_window(minutes: i64) -> String {
    if minutes % 10_080 == 0 {
        format!("{} days", minutes / 1_440)
    } else if minutes % 60 == 0 {
        format!("{} hours", minutes / 60)
    } else {
        format!("{minutes} minutes")
    }
}

fn epoch_rfc3339(value: i64) -> Option<String> {
    DateTime::from_timestamp(value, 0).map(|time| time.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_codex_windows_without_duplicates() {
        let account = json!({"account":{"type":"chatgpt","planType":"plus"}});
        let limits = json!({
            "rateLimits":{"primary":{"usedPercent":44.0,"windowDurationMins":10080,"resetsAt":1787277647}},
            "rateLimitsByLimitId":{"codex":{"primary":{"usedPercent":44.0,"windowDurationMins":10080,"resetsAt":1787277647}}}
        });
        let result = parse_codex(&account, &limits).unwrap();
        assert_eq!(result.metrics.len(), 1);
        assert_eq!(result.metrics[0].scope, "Codex only");
        assert_eq!(result.metrics[0].window.as_deref(), Some("7 days"));
    }

    #[test]
    fn parses_gemini_remaining_fraction_as_used_percent() {
        let result = parse_gemini_cli(
            &json!({"currentTier":{"name":"Google AI Pro"}}),
            &json!({"buckets":[{"modelId":"gemini-pro","remainingFraction":0.72,"resetTime":"2026-08-19T00:00:00Z"}]}),
        )
        .unwrap();
        assert_eq!(result.metrics[0].value, 28.0);
        assert_eq!(result.metrics[0].source, FetchStrategy::CliOauth);
        assert_eq!(result.status, "Experimental");
    }

    #[test]
    fn reads_oauth_client_values_from_installed_javascript() {
        let source = r#"
            const OAUTH_CLIENT_ID = 'example-client-id';
            const OAUTH_CLIENT_SECRET = "example-client-secret";
        "#;
        assert_eq!(
            javascript_const(source, "OAUTH_CLIENT_ID").as_deref(),
            Some("example-client-id")
        );
        assert_eq!(
            javascript_const(source, "OAUTH_CLIENT_SECRET").as_deref(),
            Some("example-client-secret")
        );
    }

    #[test]
    #[ignore = "requires a locally signed-in Codex installation"]
    fn reads_live_codex_subscription() {
        let result = tauri::async_runtime::block_on(fetch_codex()).unwrap();
        assert_eq!(result.provider_id, "chatgpt-codex");
        assert!(result
            .metrics
            .iter()
            .any(|metric| metric.kind == "codex_quota_primary"));
    }

    #[test]
    #[ignore = "requires a locally signed-in Codex installation"]
    fn detects_live_codex_identity_before_consent() {
        let detection = tauri::async_runtime::block_on(detect_codex()).unwrap();
        assert_eq!(
            detection.availability,
            AuthenticationAvailability::ExistingSession
        );
        assert_eq!(detection.scope, "Codex only");
        assert_eq!(detection.credential_owner, CredentialOwner::Codex);
        assert!(detection.identity_label.is_some());
        assert!(detection.plan_label.is_some());
    }
}
