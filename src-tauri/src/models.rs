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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDescriptor {
    pub id: String,
    pub display_name: String,
    pub strategy: FetchStrategy,
    pub capabilities: Vec<ProviderCapability>,
    pub official_url: String,
    pub experimental: bool,
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
    pub observed_at: String,
    pub diagnostic: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAccountRequest {
    pub provider_id: String,
    pub display_name: String,
    pub credential: String,
    pub credential_kind: Option<String>,
}
