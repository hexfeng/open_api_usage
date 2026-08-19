use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FetchStrategy {
    OfficialApi,
    CliOauth,
    BrowserSessionExperimental,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProviderCapability {
    Balance,
    Costs,
    Tokens,
    Requests,
    Quota,
    PlanMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AccountType {
    ApiPlatform,
    Subscription,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AuthenticationMode {
    PastedSecret,
    ProviderOauth,
    SharedLocalSession,
    LocalCliOauth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CredentialOwner {
    Dashboard,
    Codex,
    GeminiCli,
    Provider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDescriptor {
    pub id: String,
    pub display_name: String,
    pub strategy: FetchStrategy,
    pub capabilities: Vec<ProviderCapability>,
    pub official_url: String,
    pub experimental: bool,
    pub account_type: AccountType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountConnection {
    pub id: String,
    pub provider_id: String,
    pub display_name: String,
    pub credential_ref: Option<String>,
    pub credential_kind: Option<String>,
    pub enabled: bool,
    pub source: FetchStrategy,
    pub scope: String,
    pub account_type: AccountType,
    pub plan_name: Option<String>,
    pub monthly_price: Option<String>,
    pub renewal_date: Option<String>,
    pub auth_mode: AuthenticationMode,
    pub credential_owner: CredentialOwner,
    pub identity_label: Option<String>,
    pub identity_fingerprint: Option<String>,
    pub consented_at: String,
    pub last_validated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AuthenticationAvailability {
    ExistingSession,
    AuthenticationRequired,
    NotInstalled,
    CredentialInvalid,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationDetection {
    pub provider_id: String,
    pub availability: AuthenticationAvailability,
    pub authentication_mode: AuthenticationMode,
    pub credential_owner: CredentialOwner,
    pub identity_label: Option<String>,
    pub plan_label: Option<String>,
    pub scope: String,
    pub diagnostic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AuthenticationAttemptStatus {
    Authorizing,
    Completed,
    Cancelled,
    TimedOut,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationAttempt {
    pub attempt_id: String,
    pub provider_id: String,
    pub status: AuthenticationAttemptStatus,
    pub authorization_url: Option<String>,
    pub user_code: Option<String>,
    pub expires_at: String,
    pub detection: Option<AuthenticationDetection>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metric {
    pub kind: String,
    pub value: f64,
    pub unit: String,
    pub currency: Option<String>,
    pub scope: String,
    pub window: Option<String>,
    pub reset_time: Option<String>,
    pub observed_at: String,
    pub source: FetchStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResult {
    pub provider_id: String,
    pub status: String,
    pub metrics: Vec<Metric>,
    pub history_buckets: Vec<HistoryBucket>,
    pub observed_at: String,
    pub diagnostic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryBucket {
    pub metric_kind: String,
    pub bucket_start: String,
    pub bucket_end: String,
    pub value: f64,
    pub unit: String,
    pub currency: Option<String>,
    pub source: FetchStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    pub observed_at: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySeries {
    pub label: String,
    pub basis: String,
    pub unit: String,
    pub currency: Option<String>,
    pub points: Vec<HistoryPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSnapshot {
    pub account: AccountConnection,
    pub result: Option<FetchResult>,
    pub history: HistorySeries,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub api_refresh_minutes: u32,
    pub local_refresh_minutes: u32,
    pub start_on_login: bool,
    pub minimize_to_tray: bool,
    pub history_retention_days: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_refresh_minutes: 15,
            local_refresh_minutes: 30,
            start_on_login: true,
            minimize_to_tray: true,
            history_retention_days: 90,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAccountRequest {
    pub provider_id: String,
    pub display_name: String,
    pub credential: String,
    pub credential_kind: Option<String>,
    pub plan_name: Option<String>,
    pub monthly_price: Option<String>,
    pub renewal_date: Option<String>,
    pub identity_label: Option<String>,
    pub authentication_mode: Option<AuthenticationMode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountRequest {
    pub account_id: String,
    pub display_name: String,
    pub credential: Option<String>,
    pub credential_kind: Option<String>,
    pub plan_name: Option<String>,
    pub monthly_price: Option<String>,
    pub renewal_date: Option<String>,
}
