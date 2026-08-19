use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, Lines},
    net::TcpListener,
    process::{Child, ChildStdin, ChildStdout},
    sync::oneshot,
    time::{timeout, Duration},
};
use url::Url;

use crate::{
    credentials,
    error::{AppError, AppResult},
    local_sources,
    models::{
        AuthenticationAttempt, AuthenticationAttemptStatus, AuthenticationAvailability,
        AuthenticationDetection, AuthenticationMode, CredentialOwner,
    },
    providers,
};

const LOGIN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const START_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone, Default)]
pub struct AuthenticationState {
    attempts: Arc<Mutex<HashMap<String, StoredAttempt>>>,
}

struct StoredAttempt {
    public: AuthenticationAttempt,
    cancel: Option<oneshot::Sender<()>>,
    credential_ref: Option<String>,
}

impl AuthenticationState {
    fn insert(&self, attempt: AuthenticationAttempt, cancel: oneshot::Sender<()>) {
        self.attempts
            .lock()
            .expect("authentication state poisoned")
            .insert(
                attempt.attempt_id.clone(),
                StoredAttempt {
                    public: attempt,
                    cancel: Some(cancel),
                    credential_ref: None,
                },
            );
    }

    fn update(
        &self,
        attempt_id: &str,
        status: AuthenticationAttemptStatus,
        detection: Option<AuthenticationDetection>,
        error: Option<String>,
        credential_ref: Option<String>,
    ) {
        if let Some(attempt) = self
            .attempts
            .lock()
            .expect("authentication state poisoned")
            .get_mut(attempt_id)
        {
            if attempt.public.status == AuthenticationAttemptStatus::Cancelled {
                if let Some(reference) = credential_ref {
                    let _ = credentials::delete(&reference);
                }
                return;
            }
            attempt.public.status = status;
            attempt.public.detection = detection;
            attempt.public.error = error;
            attempt.public.authorization_url = None;
            attempt.public.user_code = None;
            attempt.cancel = None;
            attempt.credential_ref = credential_ref;
        }
    }

    fn time_out(&self, attempt_id: &str, message: &str) {
        self.update(
            attempt_id,
            AuthenticationAttemptStatus::TimedOut,
            None,
            Some(message.into()),
            None,
        );
    }

    pub fn poll(&self, attempt_id: &str) -> AppResult<AuthenticationAttempt> {
        self.attempts
            .lock()
            .expect("authentication state poisoned")
            .get(attempt_id)
            .map(|stored| stored.public.clone())
            .ok_or_else(|| AppError::InvalidRequest("authentication attempt not found".into()))
    }

    pub fn cancel(&self, attempt_id: &str) -> AppResult<()> {
        let mut attempts = self.attempts.lock().expect("authentication state poisoned");
        let stored = attempts
            .get_mut(attempt_id)
            .ok_or_else(|| AppError::InvalidRequest("authentication attempt not found".into()))?;
        if let Some(cancel) = stored.cancel.take() {
            let _ = cancel.send(());
        }
        if let Some(reference) = stored.credential_ref.take() {
            let _ = credentials::delete(&reference);
        }
        stored.public.status = AuthenticationAttemptStatus::Cancelled;
        stored.public.detection = None;
        stored.public.authorization_url = None;
        stored.public.user_code = None;
        Ok(())
    }

    pub fn take_openrouter_credential(&self, attempt_id: &str) -> AppResult<String> {
        let mut attempts = self.attempts.lock().expect("authentication state poisoned");
        let stored = attempts
            .get(attempt_id)
            .ok_or_else(|| AppError::InvalidRequest("authentication attempt not found".into()))?;
        if stored.public.provider_id != "openrouter"
            || stored.public.status != AuthenticationAttemptStatus::Completed
        {
            return Err(AppError::InvalidRequest(
                "OpenRouter authorization has not completed".into(),
            ));
        }
        let stored = attempts
            .remove(attempt_id)
            .expect("checked authentication attempt");
        stored.credential_ref.ok_or_else(|| {
            AppError::InvalidResponse("OpenRouter authorized credential is unavailable".into())
        })
    }
}

pub async fn detect(provider_id: &str) -> AppResult<AuthenticationDetection> {
    match provider_id {
        "chatgpt-codex" => local_sources::detect_codex().await,
        "google-gemini-cli" => local_sources::detect_gemini_cli().await,
        other => Err(AppError::InvalidRequest(format!(
            "authentication detection is not used for {other}"
        ))),
    }
}

pub fn identity_fingerprint(label: &str) -> String {
    let digest = Sha256::digest(label.trim().to_lowercase().as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub async fn start_codex_login(
    state: AuthenticationState,
    use_device_code: bool,
) -> AppResult<AuthenticationAttempt> {
    let executable = local_sources::codex_executable();
    let mut command = local_sources::codex_command(&executable);
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
    let login = if use_device_code {
        json!({"method":"account/login/start","id":2,"params":{"type":"chatgptDeviceCode"}})
    } else {
        json!({"method":"account/login/start","id":2,"params":{"type":"chatgpt","useHostedLoginSuccessPage":true,"appBrand":"codex"}})
    };
    local_sources::write_codex_messages(
        &mut stdin,
        &[
            local_sources::initialize_message(),
            json!({"method":"initialized"}),
            login,
        ],
    )
    .await?;
    let mut lines = BufReader::new(stdout).lines();
    let result = timeout(
        START_TIMEOUT,
        read_rpc(&mut lines, 2, "account/login/start"),
    )
    .await
    .map_err(|_| AppError::InvalidResponse("Codex sign-in start timed out".into()))??;
    let login_id = result
        .get("loginId")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidResponse("Codex login ID missing".into()))?
        .to_owned();
    let authorization_url = result
        .get(if use_device_code {
            "verificationUrl"
        } else {
            "authUrl"
        })
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| AppError::InvalidResponse("Codex authorization URL missing".into()))?;
    let attempt_id = uuid::Uuid::new_v4().to_string();
    let attempt = AuthenticationAttempt {
        attempt_id: attempt_id.clone(),
        provider_id: "chatgpt-codex".into(),
        status: AuthenticationAttemptStatus::Authorizing,
        authorization_url: Some(authorization_url),
        user_code: result
            .get("userCode")
            .and_then(Value::as_str)
            .map(str::to_owned),
        expires_at: (Utc::now()
            + chrono::Duration::from_std(LOGIN_TIMEOUT).expect("valid duration"))
        .to_rfc3339(),
        detection: None,
        error: None,
    };
    let (cancel_tx, cancel_rx) = oneshot::channel();
    state.insert(attempt.clone(), cancel_tx);
    tokio::spawn(watch_codex_login(
        state.clone(),
        attempt_id,
        login_id,
        child,
        stdin,
        lines,
        cancel_rx,
    ));
    Ok(attempt)
}

async fn watch_codex_login(
    state: AuthenticationState,
    attempt_id: String,
    login_id: String,
    mut child: Child,
    mut stdin: ChildStdin,
    mut lines: Lines<BufReader<ChildStdout>>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    enum Outcome {
        Completed,
        Cancelled,
        TimedOut,
        Failed,
    }
    let (outcome, error) = tokio::select! {
        _ = &mut cancel_rx => {
            let _ = local_sources::write_codex_messages(&mut stdin, &[json!({"method":"account/login/cancel","id":3,"params":{"loginId":login_id}})]).await;
            (Outcome::Cancelled, None)
        }
        result = timeout(LOGIN_TIMEOUT, wait_codex_completion(&mut lines, &login_id)) => match result {
            Err(_) => (Outcome::TimedOut, Some("Codex sign-in timed out. Start a new connection attempt.".into())),
            Ok(Err(_)) => (Outcome::Failed, Some("Codex sign-in did not complete.".into())),
            Ok(Ok(false)) => (Outcome::Failed, Some("Codex sign-in was cancelled or rejected.".into())),
            Ok(Ok(true)) => (Outcome::Completed, None),
        }
    };
    match outcome {
        Outcome::Completed => {
            let detection = async {
                local_sources::write_codex_messages(
                    &mut stdin,
                    &[json!({"method":"account/read","id":4,"params":{"refreshToken":false}})],
                )
                .await?;
                let value = read_rpc(&mut lines, 4, "account/read").await?;
                let account = value
                    .get("account")
                    .filter(|item| !item.is_null())
                    .ok_or_else(|| {
                        AppError::InvalidResponse("Codex account missing after sign-in".into())
                    })?;
                let mut detection = local_sources::codex_detection(account);
                detection.authentication_mode = AuthenticationMode::ProviderOauth;
                Ok::<_, AppError>(detection)
            }
            .await;
            match detection {
                Ok(value) => state.update(
                    &attempt_id,
                    AuthenticationAttemptStatus::Completed,
                    Some(value),
                    None,
                    None,
                ),
                Err(_) => state.update(
                    &attempt_id,
                    AuthenticationAttemptStatus::Failed,
                    None,
                    Some("Codex identity could not be verified after sign-in.".into()),
                    None,
                ),
            }
        }
        Outcome::Cancelled => state.update(
            &attempt_id,
            AuthenticationAttemptStatus::Cancelled,
            None,
            None,
            None,
        ),
        Outcome::TimedOut => state.time_out(
            &attempt_id,
            error.as_deref().unwrap_or("Codex sign-in timed out."),
        ),
        Outcome::Failed => state.update(
            &attempt_id,
            AuthenticationAttemptStatus::Failed,
            None,
            error,
            None,
        ),
    }
    let _ = child.kill().await;
}

async fn read_rpc(
    lines: &mut Lines<BufReader<ChildStdout>>,
    id: i64,
    method: &str,
) -> AppResult<Value> {
    loop {
        let line = lines.next_line().await?.ok_or_else(|| {
            AppError::InvalidResponse(format!("Codex app-server closed before {method}"))
        })?;
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if message.get("id").and_then(Value::as_i64) == Some(id) {
            return local_sources::rpc_result(message, method);
        }
    }
}

async fn wait_codex_completion(
    lines: &mut Lines<BufReader<ChildStdout>>,
    login_id: &str,
) -> AppResult<bool> {
    loop {
        let line = lines.next_line().await?.ok_or_else(|| {
            AppError::InvalidResponse("Codex app-server closed during sign-in".into())
        })?;
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(success) = codex_completion(&message, login_id) {
            return Ok(success);
        }
    }
}

fn codex_completion(message: &Value, login_id: &str) -> Option<bool> {
    if message.get("method").and_then(Value::as_str) != Some("account/login/completed") {
        return None;
    }
    let params = message.get("params")?;
    (params.get("loginId").and_then(Value::as_str) == Some(login_id)).then(|| {
        params
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}

pub async fn start_openrouter_pkce(state: AuthenticationState) -> AppResult<AuthenticationAttempt> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    let attempt_id = uuid::Uuid::new_v4().to_string();
    let callback_path = format!("/callback/{}", uuid::Uuid::new_v4());
    let callback_url = format!("http://127.0.0.1:{port}{callback_path}");
    let verifier = format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut authorization = Url::parse("https://openrouter.ai/auth")
        .map_err(|_| AppError::InvalidResponse("OpenRouter authorization URL is invalid".into()))?;
    authorization
        .query_pairs_mut()
        .append_pair("callback_url", &callback_url)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");
    let attempt = AuthenticationAttempt {
        attempt_id: attempt_id.clone(),
        provider_id: "openrouter".into(),
        status: AuthenticationAttemptStatus::Authorizing,
        authorization_url: Some(authorization.into()),
        user_code: None,
        expires_at: (Utc::now()
            + chrono::Duration::from_std(LOGIN_TIMEOUT).expect("valid duration"))
        .to_rfc3339(),
        detection: None,
        error: None,
    };
    let (cancel_tx, cancel_rx) = oneshot::channel();
    state.insert(attempt.clone(), cancel_tx);
    tokio::spawn(watch_openrouter_pkce(
        state.clone(),
        attempt_id,
        listener,
        callback_path,
        verifier,
        cancel_rx,
    ));
    Ok(attempt)
}

async fn watch_openrouter_pkce(
    state: AuthenticationState,
    attempt_id: String,
    listener: TcpListener,
    callback_path: String,
    verifier: String,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    enum Outcome {
        Completed(String),
        Cancelled,
        TimedOut,
        Failed(String),
    }
    let outcome = tokio::select! {
        _ = &mut cancel_rx => Outcome::Cancelled,
        result = timeout(LOGIN_TIMEOUT, receive_openrouter_code(listener, &callback_path)) => match result {
            Err(_) => Outcome::TimedOut,
            Ok(Err(_)) => Outcome::Failed("OpenRouter callback could not be completed.".into()),
            Ok(Ok(code)) => match exchange_openrouter_code(&code, &verifier).await {
                Ok(reference) => Outcome::Completed(reference),
                Err(error) => Outcome::Failed(error.to_string()),
            },
        }
    };
    drop(verifier);
    match outcome {
        Outcome::Completed(reference) => state.update(
            &attempt_id,
            AuthenticationAttemptStatus::Completed,
            Some(AuthenticationDetection {
                provider_id: "openrouter".into(),
                availability: AuthenticationAvailability::ExistingSession,
                authentication_mode: AuthenticationMode::ProviderOauth,
                credential_owner: CredentialOwner::Dashboard,
                identity_label: Some("OpenRouter user-controlled key".into()),
                plan_label: None,
                scope: "API key · key-level usage".into(),
                diagnostic:
                    "OpenRouter authorized a user-controlled API key through localhost PKCE S256."
                        .into(),
            }),
            None,
            Some(reference),
        ),
        Outcome::Cancelled => state.update(
            &attempt_id,
            AuthenticationAttemptStatus::Cancelled,
            None,
            None,
            None,
        ),
        Outcome::TimedOut => state.time_out(&attempt_id, "OpenRouter authorization timed out."),
        Outcome::Failed(error) => state.update(
            &attempt_id,
            AuthenticationAttemptStatus::Failed,
            None,
            Some(error),
            None,
        ),
    }
}

async fn receive_openrouter_code(listener: TcpListener, callback_path: &str) -> AppResult<String> {
    let mut guard = CallbackGuard {
        path: callback_path.to_owned(),
        consumed: false,
    };
    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut buffer = [0_u8; 8192];
        let length = stream.read(&mut buffer).await?;
        let request = String::from_utf8_lossy(&buffer[..length]);
        let target = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1));
        match target.and_then(|value| guard.accept(value)) {
            Some(code) => {
                stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<!doctype html><title>Connected</title><p>OpenRouter connected. You can return to AI Usage Dashboard.</p>").await?;
                return Ok(code);
            }
            None => {
                stream
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await?;
            }
        }
    }
}

struct CallbackGuard {
    path: String,
    consumed: bool,
}

impl CallbackGuard {
    fn accept(&mut self, target: &str) -> Option<String> {
        if self.consumed {
            return None;
        }
        let code = callback_code(target, &self.path)?;
        self.consumed = true;
        Some(code)
    }
}

fn callback_code(target: &str, callback_path: &str) -> Option<String> {
    let url = Url::parse(&format!("http://localhost{target}")).ok()?;
    if url.path() != callback_path {
        return None;
    }
    url.query_pairs()
        .find_map(|(name, value)| (name == "code" && !value.is_empty()).then(|| value.into_owned()))
}

async fn exchange_openrouter_code(code: &str, verifier: &str) -> AppResult<String> {
    let response = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?
        .post("https://openrouter.ai/api/v1/auth/keys")
        .json(&json!({"code":code,"code_verifier":verifier,"code_challenge_method":"S256"}))
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::InvalidResponse(match status.as_u16() {
            403 => "Authentication required: OpenRouter code is invalid, expired, or already used"
                .into(),
            429 => "Rate limited: OpenRouter temporarily rejected the key exchange".into(),
            _ => format!("OpenRouter key exchange returned HTTP {status}"),
        }));
    }
    let body: Value = response.json().await?;
    let key = body.get("key").and_then(Value::as_str).ok_or_else(|| {
        AppError::InvalidResponse("OpenRouter key exchange returned no API key".into())
    })?;
    providers::fetch("openrouter", key, Some("api_key")).await?;
    let reference = format!("openrouter-pending-{}", uuid::Uuid::new_v4());
    credentials::store(&reference, key)?;
    Ok(reference)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_requires_the_random_path_and_nonempty_code() {
        assert_eq!(
            callback_code("/callback/random?code=once", "/callback/random").as_deref(),
            Some("once")
        );
        assert!(callback_code("/callback/other?code=once", "/callback/random").is_none());
        assert!(callback_code("/callback/random?code=", "/callback/random").is_none());
    }

    #[test]
    fn pkce_challenge_is_s256_base64url_without_padding() {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(b"verifier"));
        assert_eq!(challenge.len(), 43);
        assert!(!challenge.contains('='));
        assert_ne!(challenge, "verifier");
    }

    #[test]
    fn identity_fingerprint_is_stable_and_not_the_identity() {
        let one = identity_fingerprint(" User@Example.com ");
        let two = identity_fingerprint("user@example.com");
        assert_eq!(one, two);
        assert_eq!(one.len(), 64);
        assert!(!one.contains("example"));
    }

    #[test]
    fn callback_code_is_single_use() {
        let mut guard = CallbackGuard {
            path: "/callback/random".into(),
            consumed: false,
        };
        assert_eq!(
            guard.accept("/callback/random?code=once").as_deref(),
            Some("once")
        );
        assert!(guard.accept("/callback/random?code=replay").is_none());
    }

    #[test]
    fn codex_completion_is_bound_to_its_login_id() {
        let completed = json!({"method":"account/login/completed","params":{"loginId":"expected","success":true}});
        assert_eq!(codex_completion(&completed, "expected"), Some(true));
        assert_eq!(codex_completion(&completed, "other"), None);
        assert_eq!(
            codex_completion(&json!({"method":"account/updated"}), "expected"),
            None
        );
    }

    #[test]
    fn codex_failed_completion_is_not_treated_as_success() {
        let completed = json!({"method":"account/login/completed","params":{"loginId":"expected","success":false,"error":"cancelled"}});
        assert_eq!(codex_completion(&completed, "expected"), Some(false));
    }

    #[test]
    fn late_completion_cannot_revive_a_cancelled_attempt() {
        let state = AuthenticationState::default();
        let (cancel, _receiver) = oneshot::channel();
        let attempt = AuthenticationAttempt {
            attempt_id: "cancelled".into(),
            provider_id: "openrouter".into(),
            status: AuthenticationAttemptStatus::Authorizing,
            authorization_url: Some("https://openrouter.ai/auth".into()),
            user_code: None,
            expires_at: "2026-08-19T00:00:00Z".into(),
            detection: None,
            error: None,
        };
        state.insert(attempt, cancel);
        state.cancel("cancelled").unwrap();
        state.update(
            "cancelled",
            AuthenticationAttemptStatus::Completed,
            Some(AuthenticationDetection {
                provider_id: "openrouter".into(),
                availability: AuthenticationAvailability::ExistingSession,
                authentication_mode: AuthenticationMode::ProviderOauth,
                credential_owner: CredentialOwner::Dashboard,
                identity_label: None,
                plan_label: None,
                scope: "API key".into(),
                diagnostic: "late".into(),
            }),
            None,
            None,
        );
        let stored = state.poll("cancelled").unwrap();
        assert_eq!(stored.status, AuthenticationAttemptStatus::Cancelled);
        assert!(stored.detection.is_none());
    }

    #[test]
    fn premature_oauth_save_does_not_orphan_the_live_attempt() {
        let state = AuthenticationState::default();
        let (cancel, _receiver) = oneshot::channel();
        state.insert(
            AuthenticationAttempt {
                attempt_id: "pending".into(),
                provider_id: "openrouter".into(),
                status: AuthenticationAttemptStatus::Authorizing,
                authorization_url: None,
                user_code: None,
                expires_at: "2026-08-19T00:00:00Z".into(),
                detection: None,
                error: None,
            },
            cancel,
        );
        assert!(state.take_openrouter_credential("pending").is_err());
        assert_eq!(
            state.poll("pending").unwrap().status,
            AuthenticationAttemptStatus::Authorizing
        );
    }

    #[test]
    fn timeout_has_a_distinct_terminal_state() {
        let state = AuthenticationState::default();
        let (cancel, _receiver) = oneshot::channel();
        state.insert(
            AuthenticationAttempt {
                attempt_id: "timeout".into(),
                provider_id: "chatgpt-codex".into(),
                status: AuthenticationAttemptStatus::Authorizing,
                authorization_url: None,
                user_code: None,
                expires_at: "2026-08-19T00:00:00Z".into(),
                detection: None,
                error: None,
            },
            cancel,
        );
        state.time_out("timeout", "Codex sign-in timed out.");
        let stored = state.poll("timeout").unwrap();
        assert_eq!(stored.status, AuthenticationAttemptStatus::TimedOut);
        assert_eq!(stored.error.as_deref(), Some("Codex sign-in timed out."));
    }
}
