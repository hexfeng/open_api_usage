use std::{path::Path, sync::Mutex};

use rusqlite::{params, Connection};

use crate::{
    error::AppResult,
    models::{AccountConnection, FetchResult, FetchStrategy},
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
             );",
        )?;
        let _ = connection.execute("ALTER TABLE accounts ADD COLUMN credential_kind TEXT", []);
        Ok(Self(Mutex::new(connection)))
    }

    pub fn save_account(&self, account: &AccountConnection) -> AppResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let connection = self.0.lock().expect("database mutex poisoned");
        connection.execute(
            "INSERT INTO accounts (id, provider_id, display_name, credential_ref, credential_kind, enabled, source, scope, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
             ON CONFLICT(id) DO UPDATE SET display_name=excluded.display_name, credential_ref=excluded.credential_ref,
               credential_kind=excluded.credential_kind, enabled=excluded.enabled, source=excluded.source, scope=excluded.scope, updated_at=excluded.updated_at",
            params![account.id, account.provider_id, account.display_name, account.credential_ref, account.credential_kind, account.enabled, strategy_name(&account.source), account.scope, now],
        )?;
        Ok(())
    }

    pub fn list_accounts(&self) -> AppResult<Vec<AccountConnection>> {
        let connection = self.0.lock().expect("database mutex poisoned");
        let mut statement = connection.prepare(
            "SELECT id, provider_id, display_name, credential_ref, credential_kind, enabled, source, scope FROM accounts ORDER BY created_at",
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
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
            if metric.kind.contains("balance") {
                if let Some(currency) = &metric.currency {
                    transaction.execute(
                        "INSERT OR IGNORE INTO balance_snapshots (account_id, observed_at, value, currency, source) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![account_id, metric.observed_at, metric.value, currency, strategy_name(&metric.source)],
                    )?;
                }
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn clear(&self) -> AppResult<()> {
        let connection = self.0.lock().expect("database mutex poisoned");
        connection.execute_batch("DELETE FROM metrics; DELETE FROM provider_history_buckets; DELETE FROM balance_snapshots; DELETE FROM accounts;")?;
        Ok(())
    }

    pub fn delete_account(&self, account_id: &str) -> AppResult<Option<String>> {
        let connection = self.0.lock().expect("database mutex poisoned");
        let credential_ref = connection
            .query_row(
                "SELECT credential_ref FROM accounts WHERE id=?1",
                [account_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .unwrap_or(None);
        connection.execute("DELETE FROM accounts WHERE id=?1", [account_id])?;
        Ok(credential_ref)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
