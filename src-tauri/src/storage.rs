use std::{collections::BTreeMap, path::Path, sync::Mutex};

use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    error::AppResult,
    models::{
        AccountConnection, AccountSnapshot, AccountType, AppSettings, AuthenticationMode,
        CredentialOwner, FetchResult, FetchStrategy, HistoryPoint, HistorySeries, Metric,
    },
};

pub struct AppDatabase(pub Mutex<Connection>);

impl AppDatabase {
    pub fn open(path: &Path) -> AppResult<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS accounts (
               id TEXT PRIMARY KEY,
               provider_id TEXT NOT NULL,
               display_name TEXT NOT NULL,
               credential_ref TEXT,
               credential_kind TEXT,
               enabled INTEGER NOT NULL DEFAULT 1,
               source TEXT NOT NULL,
               scope TEXT NOT NULL,
               account_type TEXT NOT NULL DEFAULT 'api_platform',
               plan_name TEXT,
               monthly_price TEXT,
               renewal_date TEXT,
               auth_mode TEXT NOT NULL DEFAULT 'pasted_secret',
               credential_owner TEXT NOT NULL DEFAULT 'dashboard',
               identity_label TEXT,
               identity_fingerprint TEXT,
               consented_at TEXT NOT NULL DEFAULT '',
               last_validated_at TEXT,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS metrics (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               account_id TEXT NOT NULL,
               kind TEXT NOT NULL,
               value REAL NOT NULL,
               unit TEXT NOT NULL,
               currency TEXT,
               scope TEXT NOT NULL,
               window_label TEXT,
               reset_time TEXT,
               observed_at TEXT NOT NULL,
               source TEXT NOT NULL,
               FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
             );
             CREATE TABLE IF NOT EXISTS provider_history_buckets (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               account_id TEXT NOT NULL,
               metric_kind TEXT NOT NULL,
               bucket_start TEXT NOT NULL,
               bucket_end TEXT NOT NULL,
               value REAL NOT NULL,
               unit TEXT NOT NULL,
               currency TEXT,
               source TEXT NOT NULL,
               UNIQUE(account_id, metric_kind, bucket_start, bucket_end)
             );
             CREATE TABLE IF NOT EXISTS balance_snapshots (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               account_id TEXT NOT NULL,
               observed_at TEXT NOT NULL,
               value REAL NOT NULL,
               currency TEXT NOT NULL,
               source TEXT NOT NULL,
               UNIQUE(account_id, observed_at, currency)
             );
             CREATE TABLE IF NOT EXISTS account_state (
               account_id TEXT PRIMARY KEY,
               last_status TEXT,
               last_observed_at TEXT,
               diagnostic TEXT,
               last_error TEXT,
               last_error_at TEXT,
               FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
             );
             CREATE TABLE IF NOT EXISTS app_settings (
               id INTEGER PRIMARY KEY CHECK (id = 1),
               api_refresh_minutes INTEGER NOT NULL,
               local_refresh_minutes INTEGER NOT NULL,
               start_on_login INTEGER NOT NULL,
               minimize_to_tray INTEGER NOT NULL,
               history_retention_days INTEGER NOT NULL
             );",
        )?;
        let _ = connection.execute("ALTER TABLE accounts ADD COLUMN credential_kind TEXT", []);
        let _ = connection.execute(
            "ALTER TABLE accounts ADD COLUMN account_type TEXT NOT NULL DEFAULT 'api_platform'",
            [],
        );
        let _ = connection.execute("ALTER TABLE accounts ADD COLUMN plan_name TEXT", []);
        let _ = connection.execute("ALTER TABLE accounts ADD COLUMN monthly_price TEXT", []);
        let _ = connection.execute("ALTER TABLE accounts ADD COLUMN renewal_date TEXT", []);
        let _ = connection.execute(
            "ALTER TABLE accounts ADD COLUMN auth_mode TEXT NOT NULL DEFAULT 'pasted_secret'",
            [],
        );
        let _ = connection.execute(
            "ALTER TABLE accounts ADD COLUMN credential_owner TEXT NOT NULL DEFAULT 'dashboard'",
            [],
        );
        let _ = connection.execute("ALTER TABLE accounts ADD COLUMN identity_label TEXT", []);
        let _ = connection.execute(
            "ALTER TABLE accounts ADD COLUMN identity_fingerprint TEXT",
            [],
        );
        let _ = connection.execute(
            "ALTER TABLE accounts ADD COLUMN consented_at TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = connection.execute("ALTER TABLE accounts ADD COLUMN last_validated_at TEXT", []);
        connection.execute(
            "UPDATE accounts SET auth_mode='shared_local_session', credential_owner='codex' WHERE provider_id='chatgpt-codex' AND auth_mode='pasted_secret'",
            [],
        )?;
        connection.execute(
            "UPDATE accounts SET auth_mode='local_cli_oauth', credential_owner='gemini_cli' WHERE provider_id='google-gemini-cli' AND auth_mode='pasted_secret'",
            [],
        )?;
        connection.execute(
            "UPDATE accounts SET consented_at=updated_at WHERE consented_at=''",
            [],
        )?;
        let _ = connection.execute(
            "ALTER TABLE provider_history_buckets ADD COLUMN currency TEXT",
            [],
        );
        let defaults = AppSettings::default();
        connection.execute(
            "INSERT OR IGNORE INTO app_settings (id, api_refresh_minutes, local_refresh_minutes, start_on_login, minimize_to_tray, history_retention_days)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![defaults.api_refresh_minutes, defaults.local_refresh_minutes, defaults.start_on_login, defaults.minimize_to_tray, defaults.history_retention_days],
        )?;
        connection.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_metrics_account_observed ON metrics(account_id, observed_at);
             CREATE INDEX IF NOT EXISTS idx_history_account_bucket ON provider_history_buckets(account_id, bucket_start);
             CREATE INDEX IF NOT EXISTS idx_balance_account_observed ON balance_snapshots(account_id, observed_at);
             PRAGMA optimize;",
        )?;
        Ok(Self(Mutex::new(connection)))
    }

    pub fn save_account(&self, account: &AccountConnection) -> AppResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let connection = self.0.lock().expect("database mutex poisoned");
        connection.execute(
            "INSERT INTO accounts (id, provider_id, display_name, credential_ref, credential_kind, enabled, source, scope, account_type, plan_name, monthly_price, renewal_date, auth_mode, credential_owner, identity_label, identity_fingerprint, consented_at, last_validated_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?19)
             ON CONFLICT(id) DO UPDATE SET display_name=excluded.display_name, credential_ref=excluded.credential_ref,
               credential_kind=excluded.credential_kind, enabled=excluded.enabled, source=excluded.source, scope=excluded.scope,
               account_type=excluded.account_type, plan_name=excluded.plan_name, monthly_price=excluded.monthly_price,
               renewal_date=excluded.renewal_date, auth_mode=excluded.auth_mode, credential_owner=excluded.credential_owner,
               identity_label=excluded.identity_label, identity_fingerprint=excluded.identity_fingerprint,
               consented_at=excluded.consented_at, last_validated_at=excluded.last_validated_at, updated_at=excluded.updated_at",
            params![account.id, account.provider_id, account.display_name, account.credential_ref, account.credential_kind, account.enabled, strategy_name(&account.source), account.scope, account_type_name(&account.account_type), account.plan_name, account.monthly_price, account.renewal_date, auth_mode_name(&account.auth_mode), credential_owner_name(&account.credential_owner), account.identity_label, account.identity_fingerprint, account.consented_at, account.last_validated_at, now],
        )?;
        Ok(())
    }

    pub fn list_accounts(&self) -> AppResult<Vec<AccountConnection>> {
        let connection = self.0.lock().expect("database mutex poisoned");
        let mut statement = connection.prepare(
            "SELECT id, provider_id, display_name, credential_ref, credential_kind, enabled, source, scope, account_type, plan_name, monthly_price, renewal_date, auth_mode, credential_owner, identity_label, identity_fingerprint, consented_at, last_validated_at FROM accounts ORDER BY created_at",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(AccountConnection {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                display_name: row.get(2)?,
                credential_ref: row.get(3)?,
                credential_kind: row.get(4)?,
                enabled: row.get(5)?,
                source: parse_strategy(&row.get::<_, String>(6)?),
                scope: row.get(7)?,
                account_type: parse_account_type(&row.get::<_, String>(8)?),
                plan_name: row.get(9)?,
                monthly_price: row.get(10)?,
                renewal_date: row.get(11)?,
                auth_mode: parse_auth_mode(&row.get::<_, String>(12)?),
                credential_owner: parse_credential_owner(&row.get::<_, String>(13)?),
                identity_label: row.get(14)?,
                identity_fingerprint: row.get(15)?,
                consented_at: row.get(16)?,
                last_validated_at: row.get(17)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_account(&self, account_id: &str) -> AppResult<Option<AccountConnection>> {
        let connection = self.0.lock().expect("database mutex poisoned");
        Ok(connection
            .query_row(
                "SELECT id, provider_id, display_name, credential_ref, credential_kind, enabled, source, scope, account_type, plan_name, monthly_price, renewal_date, auth_mode, credential_owner, identity_label, identity_fingerprint, consented_at, last_validated_at FROM accounts WHERE id=?1",
                [account_id],
                account_from_row,
            )
            .optional()?)
    }

    pub fn set_account_enabled(
        &self,
        account_id: &str,
        enabled: bool,
    ) -> AppResult<AccountConnection> {
        let connection = self.0.lock().expect("database mutex poisoned");
        connection.execute(
            "UPDATE accounts SET enabled=?2, updated_at=?3 WHERE id=?1",
            params![account_id, enabled, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(connection.query_row(
            "SELECT id, provider_id, display_name, credential_ref, credential_kind, enabled, source, scope, account_type, plan_name, monthly_price, renewal_date, auth_mode, credential_owner, identity_label, identity_fingerprint, consented_at, last_validated_at FROM accounts WHERE id=?1",
            [account_id],
            account_from_row,
        )?)
    }

    pub fn save_fetch_result(&self, account_id: &str, result: &FetchResult) -> AppResult<()> {
        let mut connection = self.0.lock().expect("database mutex poisoned");
        let transaction = connection.transaction()?;
        for metric in &result.metrics {
            transaction.execute(
                "INSERT INTO metrics (account_id, kind, value, unit, currency, scope, window_label, reset_time, observed_at, source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![account_id, metric.kind, metric.value, metric.unit, metric.currency, metric.scope, metric.window, metric.reset_time, metric.observed_at, strategy_name(&metric.source)],
            )?;
            if metric.kind == "total_balance" {
                if let Some(currency) = &metric.currency {
                    transaction.execute(
                        "INSERT OR IGNORE INTO balance_snapshots (account_id, observed_at, value, currency, source) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![account_id, metric.observed_at, metric.value, currency, strategy_name(&metric.source)],
                    )?;
                }
            }
        }
        for bucket in &result.history_buckets {
            transaction.execute(
                "INSERT INTO provider_history_buckets (account_id, metric_kind, bucket_start, bucket_end, value, unit, currency, source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(account_id, metric_kind, bucket_start, bucket_end) DO UPDATE SET value=excluded.value, unit=excluded.unit, currency=excluded.currency, source=excluded.source",
                params![account_id, bucket.metric_kind, bucket.bucket_start, bucket.bucket_end, bucket.value, bucket.unit, bucket.currency, strategy_name(&bucket.source)],
            )?;
        }
        transaction.execute(
            "INSERT INTO account_state (account_id, last_status, last_observed_at, diagnostic, last_error, last_error_at)
             VALUES (?1, ?2, ?3, ?4, NULL, NULL)
             ON CONFLICT(account_id) DO UPDATE SET last_status=excluded.last_status, last_observed_at=excluded.last_observed_at,
               diagnostic=excluded.diagnostic, last_error=NULL, last_error_at=NULL",
            params![account_id, result.status, result.observed_at, result.diagnostic],
        )?;
        transaction.execute(
            "UPDATE accounts SET last_validated_at=?2, updated_at=?2 WHERE id=?1",
            params![account_id, result.observed_at],
        )?;
        transaction.commit()?;
        drop(connection);
        let retention = self.get_settings()?.history_retention_days;
        self.prune_history(retention)?;
        Ok(())
    }

    pub fn record_refresh_failure(&self, account_id: &str, error: &str) -> AppResult<()> {
        let connection = self.0.lock().expect("database mutex poisoned");
        connection.execute(
            "INSERT INTO account_state (account_id, last_error, last_error_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(account_id) DO UPDATE SET last_error=excluded.last_error, last_error_at=excluded.last_error_at",
            params![account_id, error, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn list_snapshots(&self) -> AppResult<Vec<AccountSnapshot>> {
        self.list_accounts()?
            .into_iter()
            .map(|account| self.snapshot_for(account))
            .collect()
    }

    pub fn snapshot(&self, account_id: &str) -> AppResult<AccountSnapshot> {
        let account = self
            .get_account(account_id)?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
        self.snapshot_for(account)
    }

    fn snapshot_for(&self, account: AccountConnection) -> AppResult<AccountSnapshot> {
        let connection = self.0.lock().expect("database mutex poisoned");
        let state = connection
            .query_row(
                "SELECT last_status, last_observed_at, diagnostic, last_error FROM account_state WHERE account_id=?1",
                [&account.id],
                |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, Option<String>>(3)?)),
            )
            .optional()?;
        let observed_at = state
            .as_ref()
            .and_then(|(_, observed, _, _)| observed.clone())
            .or_else(|| {
                connection
                    .query_row(
                        "SELECT MAX(observed_at) FROM metrics WHERE account_id=?1",
                        [&account.id],
                        |row| row.get::<_, Option<String>>(0),
                    )
                    .ok()
                    .flatten()
            });
        let result = if let Some(observed_at) = observed_at {
            let mut statement = connection.prepare(
                "SELECT kind, value, unit, currency, scope, window_label, reset_time, observed_at, source
                 FROM metrics WHERE account_id=?1 AND observed_at=?2 ORDER BY id",
            )?;
            let metrics = statement
                .query_map(params![account.id, observed_at], |row| {
                    Ok(Metric {
                        kind: row.get(0)?,
                        value: row.get(1)?,
                        unit: row.get(2)?,
                        currency: row.get(3)?,
                        scope: row.get(4)?,
                        window: row.get(5)?,
                        reset_time: row.get(6)?,
                        observed_at: row.get(7)?,
                        source: parse_strategy(&row.get::<_, String>(8)?),
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            let (status, _, diagnostic, _) = state.clone().unwrap_or_default();
            Some(FetchResult {
                provider_id: account.provider_id.clone(),
                status: status.unwrap_or_else(|| "Live".into()),
                metrics,
                history_buckets: Vec::new(),
                observed_at,
                diagnostic: diagnostic.unwrap_or_else(|| "Restored from local history".into()),
            })
        } else {
            None
        };
        let last_error = state.and_then(|(_, _, _, error)| error);
        let history = history_for(&connection, &account)?;
        Ok(AccountSnapshot {
            account,
            result,
            history,
            last_error,
        })
    }

    pub fn get_settings(&self) -> AppResult<AppSettings> {
        let connection = self.0.lock().expect("database mutex poisoned");
        Ok(connection.query_row(
            "SELECT api_refresh_minutes, local_refresh_minutes, start_on_login, minimize_to_tray, history_retention_days FROM app_settings WHERE id=1",
            [],
            |row| Ok(AppSettings { api_refresh_minutes: row.get(0)?, local_refresh_minutes: row.get(1)?, start_on_login: row.get(2)?, minimize_to_tray: row.get(3)?, history_retention_days: row.get(4)? }),
        )?)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> AppResult<()> {
        let connection = self.0.lock().expect("database mutex poisoned");
        connection.execute(
            "UPDATE app_settings SET api_refresh_minutes=?1, local_refresh_minutes=?2, start_on_login=?3, minimize_to_tray=?4, history_retention_days=?5 WHERE id=1",
            params![settings.api_refresh_minutes, settings.local_refresh_minutes, settings.start_on_login, settings.minimize_to_tray, settings.history_retention_days],
        )?;
        Ok(())
    }

    pub fn prune_history(&self, retention_days: u32) -> AppResult<()> {
        let cutoff = format!("-{retention_days} days");
        let connection = self.0.lock().expect("database mutex poisoned");
        connection.execute(
            "DELETE FROM metrics WHERE substr(observed_at, 1, 10) < date('now', ?1)",
            [&cutoff],
        )?;
        connection.execute(
            "DELETE FROM provider_history_buckets WHERE substr(bucket_start, 1, 10) < date('now', ?1)",
            [&cutoff],
        )?;
        connection.execute(
            "DELETE FROM balance_snapshots WHERE substr(observed_at, 1, 10) < date('now', ?1)",
            [&cutoff],
        )?;
        Ok(())
    }

    pub fn clear(&self) -> AppResult<()> {
        let connection = self.0.lock().expect("database mutex poisoned");
        connection.execute_batch("DELETE FROM metrics; DELETE FROM provider_history_buckets; DELETE FROM balance_snapshots; DELETE FROM account_state; DELETE FROM accounts;")?;
        Ok(())
    }

    pub fn delete_account(&self, account_id: &str) -> AppResult<Option<String>> {
        let mut connection = self.0.lock().expect("database mutex poisoned");
        let transaction = connection.transaction()?;
        let credential_ref = transaction
            .query_row(
                "SELECT credential_ref FROM accounts WHERE id=?1",
                [account_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .unwrap_or(None);
        transaction.execute("DELETE FROM metrics WHERE account_id=?1", [account_id])?;
        transaction.execute(
            "DELETE FROM provider_history_buckets WHERE account_id=?1",
            [account_id],
        )?;
        transaction.execute(
            "DELETE FROM balance_snapshots WHERE account_id=?1",
            [account_id],
        )?;
        transaction.execute(
            "DELETE FROM account_state WHERE account_id=?1",
            [account_id],
        )?;
        transaction.execute("DELETE FROM accounts WHERE id=?1", [account_id])?;
        transaction.commit()?;
        Ok(credential_ref)
    }
}

fn account_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccountConnection> {
    Ok(AccountConnection {
        id: row.get(0)?,
        provider_id: row.get(1)?,
        display_name: row.get(2)?,
        credential_ref: row.get(3)?,
        credential_kind: row.get(4)?,
        enabled: row.get(5)?,
        source: parse_strategy(&row.get::<_, String>(6)?),
        scope: row.get(7)?,
        account_type: parse_account_type(&row.get::<_, String>(8)?),
        plan_name: row.get(9)?,
        monthly_price: row.get(10)?,
        renewal_date: row.get(11)?,
        auth_mode: parse_auth_mode(&row.get::<_, String>(12)?),
        credential_owner: parse_credential_owner(&row.get::<_, String>(13)?),
        identity_label: row.get(14)?,
        identity_fingerprint: row.get(15)?,
        consented_at: row.get(16)?,
        last_validated_at: row.get(17)?,
    })
}

fn history_for(connection: &Connection, account: &AccountConnection) -> AppResult<HistorySeries> {
    let (label, basis, kind, currency) = match account.provider_id.as_str() {
        "openai" => (
            "Daily spend",
            "Provider history buckets",
            "cost",
            Some("USD".into()),
        ),
        "deepseek" => (
            "Total balance",
            "Local balance snapshots",
            "total_balance",
            None,
        ),
        "openrouter" if account.credential_kind.as_deref() == Some("management") => (
            "Total usage",
            "Local account snapshots",
            "account_usage",
            Some("USD".into()),
        ),
        "openrouter" => (
            "Key usage",
            "Local key snapshots",
            "key_usage",
            Some("USD".into()),
        ),
        "chatgpt-codex" => (
            "Codex quota usage",
            "Local Codex quota snapshots",
            "codex_quota_primary",
            None,
        ),
        _ => ("History", "Local snapshots", "", None),
    };
    let mut points = if account.provider_id == "openai" {
        history_rows(
            connection,
            "SELECT bucket_start, value FROM provider_history_buckets WHERE account_id=?1 AND metric_kind=?2 AND substr(bucket_start, 1, 10) >= date('now', '-7 days') ORDER BY bucket_start",
            &account.id,
            kind,
        )?
    } else if account.provider_id == "deepseek" {
        history_rows(
            connection,
            "SELECT observed_at, value FROM balance_snapshots WHERE account_id=?1 AND ?2 != '' AND substr(observed_at, 1, 10) >= date('now', '-7 days') ORDER BY observed_at",
            &account.id,
            kind,
        )?
    } else {
        history_rows(
            connection,
            "SELECT observed_at, value FROM metrics WHERE account_id=?1 AND kind=?2 AND substr(observed_at, 1, 10) >= date('now', '-7 days') ORDER BY observed_at",
            &account.id,
            kind,
        )?
    };
    if points.is_empty() && account.provider_id == "openai" {
        points = history_rows(
            connection,
            "SELECT observed_at, value FROM metrics WHERE account_id=?1 AND kind=?2 AND substr(observed_at, 1, 10) >= date('now', '-7 days') ORDER BY observed_at",
            &account.id,
            kind,
        )?;
    }
    let mut daily = BTreeMap::new();
    for point in points {
        daily.insert(
            point.observed_at.chars().take(10).collect::<String>(),
            point,
        );
    }
    Ok(HistorySeries {
        label: label.into(),
        basis: basis.into(),
        unit: if currency.is_some() {
            "currency"
        } else {
            "percent"
        }
        .into(),
        currency,
        points: daily.into_values().collect(),
    })
}

fn history_rows(
    connection: &Connection,
    sql: &str,
    account_id: &str,
    kind: &str,
) -> AppResult<Vec<HistoryPoint>> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map(params![account_id, kind], |row| {
        Ok(HistoryPoint {
            observed_at: row.get(0)?,
            value: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn strategy_name(strategy: &FetchStrategy) -> &'static str {
    match strategy {
        FetchStrategy::OfficialApi => "official_api",
        FetchStrategy::CliOauth => "cli_oauth",
        FetchStrategy::BrowserSessionExperimental => "browser_session_experimental",
        FetchStrategy::Manual => "manual",
    }
}

fn parse_strategy(value: &str) -> FetchStrategy {
    match value {
        "cli_oauth" => FetchStrategy::CliOauth,
        "browser_session_experimental" => FetchStrategy::BrowserSessionExperimental,
        "manual" => FetchStrategy::Manual,
        _ => FetchStrategy::OfficialApi,
    }
}

fn account_type_name(account_type: &AccountType) -> &'static str {
    match account_type {
        AccountType::ApiPlatform => "api_platform",
        AccountType::Subscription => "subscription",
    }
}

fn parse_account_type(value: &str) -> AccountType {
    if value == "subscription" {
        AccountType::Subscription
    } else {
        AccountType::ApiPlatform
    }
}

fn auth_mode_name(mode: &AuthenticationMode) -> &'static str {
    match mode {
        AuthenticationMode::PastedSecret => "pasted_secret",
        AuthenticationMode::ProviderOauth => "provider_oauth",
        AuthenticationMode::SharedLocalSession => "shared_local_session",
        AuthenticationMode::LocalCliOauth => "local_cli_oauth",
    }
}

fn parse_auth_mode(value: &str) -> AuthenticationMode {
    match value {
        "provider_oauth" => AuthenticationMode::ProviderOauth,
        "shared_local_session" => AuthenticationMode::SharedLocalSession,
        "local_cli_oauth" => AuthenticationMode::LocalCliOauth,
        _ => AuthenticationMode::PastedSecret,
    }
}

fn credential_owner_name(owner: &CredentialOwner) -> &'static str {
    match owner {
        CredentialOwner::Dashboard => "dashboard",
        CredentialOwner::Codex => "codex",
        CredentialOwner::GeminiCli => "gemini_cli",
        CredentialOwner::Provider => "provider",
    }
}

fn parse_credential_owner(value: &str) -> CredentialOwner {
    match value {
        "codex" => CredentialOwner::Codex,
        "gemini_cli" => CredentialOwner::GeminiCli,
        "provider" => CredentialOwner::Provider,
        _ => CredentialOwner::Dashboard,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_result(account: &AccountConnection, value: f64) -> FetchResult {
        let observed_at = chrono::Utc::now().to_rfc3339();
        FetchResult {
            provider_id: account.provider_id.clone(),
            status: "Live".into(),
            metrics: vec![Metric {
                kind: "cost".into(),
                value,
                unit: "currency".into(),
                currency: Some("USD".into()),
                scope: "Organization".into(),
                window: Some("Month to date".into()),
                reset_time: None,
                observed_at: observed_at.clone(),
                source: FetchStrategy::OfficialApi,
            }],
            history_buckets: Vec::new(),
            observed_at,
            diagnostic: "test result".into(),
        }
    }

    #[test]
    fn stores_only_credential_reference() {
        let database = AppDatabase::open(Path::new(":memory:")).unwrap();
        let account = AccountConnection {
            id: "one".into(),
            provider_id: "openai".into(),
            display_name: "OpenAI".into(),
            credential_ref: Some("credential-ref".into()),
            credential_kind: Some("management".into()),
            enabled: true,
            source: FetchStrategy::OfficialApi,
            scope: "Organization".into(),
            account_type: AccountType::ApiPlatform,
            plan_name: None,
            monthly_price: None,
            renewal_date: None,
            auth_mode: AuthenticationMode::PastedSecret,
            credential_owner: CredentialOwner::Dashboard,
            identity_label: None,
            identity_fingerprint: None,
            consented_at: "2026-08-19T00:00:00Z".into(),
            last_validated_at: None,
        };
        database.save_account(&account).unwrap();
        let raw: String = database
            .0
            .lock()
            .unwrap()
            .query_row("SELECT credential_ref FROM accounts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(raw, "credential-ref");
    }

    #[test]
    fn restores_last_success_after_a_failed_refresh() {
        let database = AppDatabase::open(Path::new(":memory:")).unwrap();
        let account = AccountConnection {
            id: "one".into(),
            provider_id: "openai".into(),
            display_name: "OpenAI".into(),
            credential_ref: Some("credential-ref".into()),
            credential_kind: Some("management".into()),
            enabled: true,
            source: FetchStrategy::OfficialApi,
            scope: "Organization".into(),
            account_type: AccountType::ApiPlatform,
            plan_name: None,
            monthly_price: None,
            renewal_date: None,
            auth_mode: AuthenticationMode::PastedSecret,
            credential_owner: CredentialOwner::Dashboard,
            identity_label: None,
            identity_fingerprint: None,
            consented_at: "2026-08-19T00:00:00Z".into(),
            last_validated_at: None,
        };
        database.save_account(&account).unwrap();
        database
            .save_fetch_result(&account.id, &sample_result(&account, 12.5))
            .unwrap();
        database
            .record_refresh_failure(&account.id, "provider returned HTTP 401")
            .unwrap();

        let snapshot = database.snapshot(&account.id).unwrap();
        assert_eq!(snapshot.result.unwrap().metrics[0].value, 12.5);
        assert_eq!(
            snapshot.last_error.as_deref(),
            Some("provider returned HTTP 401")
        );
    }

    #[test]
    fn persists_account_and_scheduler_settings() {
        let database = AppDatabase::open(Path::new(":memory:")).unwrap();
        let account = AccountConnection {
            id: "one".into(),
            provider_id: "deepseek".into(),
            display_name: "DeepSeek".into(),
            credential_ref: Some("credential-ref".into()),
            credential_kind: Some("api_key".into()),
            enabled: true,
            source: FetchStrategy::OfficialApi,
            scope: "Account balance".into(),
            account_type: AccountType::ApiPlatform,
            plan_name: None,
            monthly_price: None,
            renewal_date: None,
            auth_mode: AuthenticationMode::PastedSecret,
            credential_owner: CredentialOwner::Dashboard,
            identity_label: None,
            identity_fingerprint: None,
            consented_at: "2026-08-19T00:00:00Z".into(),
            last_validated_at: None,
        };
        database.save_account(&account).unwrap();
        assert!(
            !database
                .set_account_enabled(&account.id, false)
                .unwrap()
                .enabled
        );

        let settings = AppSettings {
            api_refresh_minutes: 30,
            local_refresh_minutes: 60,
            start_on_login: false,
            minimize_to_tray: false,
            history_retention_days: 60,
        };
        database.save_settings(&settings).unwrap();
        let restored = database.get_settings().unwrap();
        assert_eq!(restored.api_refresh_minutes, 30);
        assert!(!restored.minimize_to_tray);
    }

    #[test]
    fn balance_history_uses_total_balance_only() {
        let database = AppDatabase::open(Path::new(":memory:")).unwrap();
        let account = AccountConnection {
            id: "one".into(),
            provider_id: "deepseek".into(),
            display_name: "DeepSeek".into(),
            credential_ref: Some("credential-ref".into()),
            credential_kind: Some("api_key".into()),
            enabled: true,
            source: FetchStrategy::OfficialApi,
            scope: "Account balance".into(),
            account_type: AccountType::ApiPlatform,
            plan_name: None,
            monthly_price: None,
            renewal_date: None,
            auth_mode: AuthenticationMode::PastedSecret,
            credential_owner: CredentialOwner::Dashboard,
            identity_label: None,
            identity_fingerprint: None,
            consented_at: "2026-08-19T00:00:00Z".into(),
            last_validated_at: None,
        };
        database.save_account(&account).unwrap();
        let observed_at = chrono::Utc::now().to_rfc3339();
        let metrics = [
            ("total_balance", 82.31),
            ("topped_up_balance", 70.0),
            ("granted_balance", 12.31),
        ]
        .into_iter()
        .map(|(kind, value)| Metric {
            kind: kind.into(),
            value,
            unit: "currency".into(),
            currency: Some("CNY".into()),
            scope: "Account balance".into(),
            window: None,
            reset_time: None,
            observed_at: observed_at.clone(),
            source: FetchStrategy::OfficialApi,
        })
        .collect();
        database
            .save_fetch_result(
                &account.id,
                &FetchResult {
                    provider_id: "deepseek".into(),
                    status: "Live".into(),
                    metrics,
                    history_buckets: Vec::new(),
                    observed_at,
                    diagnostic: "test result".into(),
                },
            )
            .unwrap();
        let connection = database.0.lock().unwrap();
        let (count, value): (i64, f64) = connection
            .query_row(
                "SELECT COUNT(*), MAX(value) FROM balance_snapshots",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(value, 82.31);
    }

    #[test]
    fn persists_subscription_type_and_manual_plan_metadata() {
        let database = AppDatabase::open(Path::new(":memory:")).unwrap();
        let account = AccountConnection {
            id: "codex".into(),
            provider_id: "chatgpt-codex".into(),
            display_name: "ChatGPT Plus".into(),
            credential_ref: None,
            credential_kind: Some("local_oauth".into()),
            enabled: true,
            source: FetchStrategy::CliOauth,
            scope: "Codex only".into(),
            account_type: AccountType::Subscription,
            plan_name: Some("ChatGPT Plus".into()),
            monthly_price: Some("$20 / month".into()),
            renewal_date: Some("Renews Sep 3".into()),
            auth_mode: AuthenticationMode::SharedLocalSession,
            credential_owner: CredentialOwner::Codex,
            identity_label: Some("user@example.com".into()),
            identity_fingerprint: None,
            consented_at: "2026-08-19T00:00:00Z".into(),
            last_validated_at: None,
        };
        database.save_account(&account).unwrap();
        let restored = database.get_account(&account.id).unwrap().unwrap();
        assert_eq!(restored.account_type, AccountType::Subscription);
        assert_eq!(restored.plan_name.as_deref(), Some("ChatGPT Plus"));
        assert!(restored.credential_ref.is_none());
    }

    #[test]
    fn migrates_existing_accounts_to_explicit_authentication_metadata() {
        let path = std::env::temp_dir().join(format!(
            "aud-auth-migration-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE accounts (
                id TEXT PRIMARY KEY, provider_id TEXT NOT NULL, display_name TEXT NOT NULL,
                credential_ref TEXT, credential_kind TEXT, enabled INTEGER NOT NULL,
                source TEXT NOT NULL, scope TEXT NOT NULL, account_type TEXT NOT NULL,
                plan_name TEXT, monthly_price TEXT, renewal_date TEXT,
                created_at TEXT NOT NULL, updated_at TEXT NOT NULL
             );
             INSERT INTO accounts VALUES (
                'codex', 'chatgpt-codex', 'Codex', NULL, 'local_oauth', 1,
                'cli_oauth', 'Codex only', 'subscription', NULL, NULL, NULL,
                '2026-08-18T00:00:00Z', '2026-08-19T00:00:00Z'
             );",
            )
            .unwrap();
        drop(connection);
        let database = AppDatabase::open(&path).unwrap();
        let account = database.get_account("codex").unwrap().unwrap();
        assert_eq!(account.auth_mode, AuthenticationMode::SharedLocalSession);
        assert_eq!(account.credential_owner, CredentialOwner::Codex);
        assert_eq!(account.consented_at, "2026-08-19T00:00:00Z");
        drop(database);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite3-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite3-shm"));
    }

    #[test]
    fn sqlite_schema_has_no_raw_secret_or_oauth_material_columns() {
        let database = AppDatabase::open(Path::new(":memory:")).unwrap();
        let connection = database.0.lock().unwrap();
        let mut statement = connection
            .prepare("SELECT name FROM pragma_table_info('accounts')")
            .unwrap();
        let columns = statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        for forbidden in [
            "credential",
            "api_key",
            "token",
            "authorization_code",
            "code_verifier",
            "cookie",
        ] {
            assert!(!columns.iter().any(|column| column == forbidden));
        }
        assert!(columns.contains(&"credential_ref".to_owned()));
    }
}
