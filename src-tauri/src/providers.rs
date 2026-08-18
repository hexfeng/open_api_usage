use chrono::Utc;
use reqwest::Client;
use serde_json::Value;

use crate::{
    error::{AppError, AppResult},
    local_sources,
    models::{
        AccountType, FetchResult, FetchStrategy, HistoryBucket, Metric, ProviderCapability,
        ProviderDescriptor,
    },
};

const TIMEOUT_SECONDS: u64 = 18;

pub fn descriptors() -> Vec<ProviderDescriptor> {
    vec![
        ProviderDescriptor {
            id: "openai".into(),
            display_name: "OpenAI API".into(),
            strategy: FetchStrategy::OfficialApi,
            capabilities: vec![
                ProviderCapability::Costs,
                ProviderCapability::Tokens,
                ProviderCapability::Requests,
            ],
            official_url: "https://platform.openai.com/usage".into(),
            experimental: false,
            account_type: AccountType::ApiPlatform,
        },
        ProviderDescriptor {
            id: "deepseek".into(),
            display_name: "DeepSeek API".into(),
            strategy: FetchStrategy::OfficialApi,
            capabilities: vec![ProviderCapability::Balance],
            official_url: "https://platform.deepseek.com/usage".into(),
            experimental: false,
            account_type: AccountType::ApiPlatform,
        },
        ProviderDescriptor {
            id: "openrouter".into(),
            display_name: "OpenRouter".into(),
            strategy: FetchStrategy::OfficialApi,
            capabilities: vec![ProviderCapability::Balance, ProviderCapability::Costs],
            official_url: "https://openrouter.ai/activity".into(),
            experimental: false,
            account_type: AccountType::ApiPlatform,
        },
        ProviderDescriptor {
            id: "google-ai-studio".into(),
            display_name: "Google AI Studio API".into(),
            strategy: FetchStrategy::OfficialApi,
            capabilities: vec![ProviderCapability::Quota],
            official_url: "https://aistudio.google.com/usage".into(),
            experimental: false,
            account_type: AccountType::ApiPlatform,
        },
        ProviderDescriptor {
            id: "chatgpt-codex".into(),
            display_name: "ChatGPT / Codex".into(),
            strategy: FetchStrategy::CliOauth,
            capabilities: vec![ProviderCapability::Quota, ProviderCapability::PlanMetadata],
            official_url: "https://chatgpt.com/#settings/Subscription".into(),
            experimental: false,
            account_type: AccountType::Subscription,
        },
        ProviderDescriptor {
            id: "google-gemini-cli".into(),
            display_name: "Google / Gemini CLI".into(),
            strategy: FetchStrategy::CliOauth,
            capabilities: vec![ProviderCapability::Quota, ProviderCapability::PlanMetadata],
            official_url: "https://one.google.com/explore-plan/gemini-advanced".into(),
            experimental: true,
            account_type: AccountType::Subscription,
        },
    ]
}

fn client() -> AppResult<Client> {
    Ok(Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECONDS))
        .build()?)
}

pub async fn fetch(
    provider_id: &str,
    credential: &str,
    credential_kind: Option<&str>,
) -> AppResult<FetchResult> {
    match provider_id {
        "openai" => fetch_openai(credential).await,
        "deepseek" => fetch_deepseek(credential).await,
        "openrouter" => fetch_openrouter(credential, credential_kind).await,
        "google-ai-studio" => fetch_google_ai_studio(credential).await,
        "chatgpt-codex" => local_sources::fetch_codex().await,
        "google-gemini-cli" => local_sources::fetch_gemini_cli().await,
        other => Err(AppError::UnsupportedProvider(other.into())),
    }
}

pub fn requires_secret(provider_id: &str) -> bool {
    matches!(
        provider_id,
        "openai" | "deepseek" | "openrouter" | "google-ai-studio"
    )
}

async fn fetch_openai(admin_key: &str) -> AppResult<FetchResult> {
    let start_time = Utc::now()
        .date_naive()
        .with_day(1)
        .expect("valid first day")
        .and_hms_opt(0, 0, 0)
        .expect("valid midnight")
        .and_utc()
        .timestamp();
    let http = client()?;
    let costs = get_json(
        &http,
        &format!("https://api.openai.com/v1/organization/costs?start_time={start_time}&limit=31"),
        admin_key,
    )
    .await?;
    let usage = get_json(&http, &format!("https://api.openai.com/v1/organization/usage/completions?start_time={start_time}&bucket_width=1d&limit=31"), admin_key).await?;
    parse_openai(&costs, &usage)
}

async fn fetch_deepseek(api_key: &str) -> AppResult<FetchResult> {
    let json = get_json(&client()?, "https://api.deepseek.com/user/balance", api_key).await?;
    parse_deepseek(&json)
}

async fn fetch_openrouter(api_key: &str, credential_kind: Option<&str>) -> AppResult<FetchResult> {
    let endpoint = if credential_kind == Some("management") {
        "https://openrouter.ai/api/v1/credits"
    } else {
        "https://openrouter.ai/api/v1/key"
    };
    let json = get_json(&client()?, endpoint, api_key).await?;
    parse_openrouter(&json, credential_kind == Some("management"))
}

async fn fetch_google_ai_studio(api_key: &str) -> AppResult<FetchResult> {
    let response = client()?
        .get("https://generativelanguage.googleapis.com/v1beta/models")
        .header("x-goog-api-key", api_key)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::InvalidResponse(format!(
            "Google AI Studio returned HTTP {status}"
        )));
    }
    parse_google_ai_studio(&response.json().await?)
}

async fn get_json(http: &Client, url: &str, credential: &str) -> AppResult<Value> {
    let response = http.get(url).bearer_auth(credential).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::InvalidResponse(format!(
            "provider returned HTTP {status}"
        )));
    }
    Ok(response.json().await?)
}

fn observed() -> String {
    Utc::now().to_rfc3339()
}

fn parse_openai(costs: &Value, usage: &Value) -> AppResult<FetchResult> {
    let cost = sum_nested_number(costs, &["results", "amount", "value"]);
    let tokens = sum_fields(usage, &["input_tokens", "output_tokens"]);
    let requests = sum_field(usage, "num_model_requests");
    let observed_at = observed();
    Ok(FetchResult {
        provider_id: "openai".into(),
        status: "Live".into(),
        observed_at: observed_at.clone(),
        history_buckets: openai_cost_history(costs),
        diagnostic: "Organization-level costs and completions usage. Requires an OpenAI Admin Key."
            .into(),
        metrics: vec![
            metric(
                "cost",
                cost,
                "currency",
                Some("USD"),
                "Organization",
                Some("Month to date"),
                &observed_at,
            ),
            metric(
                "tokens",
                tokens,
                "tokens",
                None,
                "Organization",
                Some("Month to date"),
                &observed_at,
            ),
            metric(
                "requests",
                requests,
                "requests",
                None,
                "Organization",
                Some("Month to date"),
                &observed_at,
            ),
        ],
    })
}

fn parse_deepseek(json: &Value) -> AppResult<FetchResult> {
    let balances = json
        .get("balance_infos")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidResponse("DeepSeek balance_infos missing".into()))?;
    let mut metrics = Vec::new();
    let observed_at = observed();
    for balance in balances {
        let currency = balance
            .get("currency")
            .and_then(Value::as_str)
            .unwrap_or("CNY");
        let total = number(balance.get("total_balance"));
        let topped_up = number(balance.get("topped_up_balance"));
        let granted = number(balance.get("granted_balance"));
        metrics.push(metric(
            "total_balance",
            total,
            "currency",
            Some(currency),
            "Account balance",
            None,
            &observed_at,
        ));
        metrics.push(metric(
            "topped_up_balance",
            topped_up,
            "currency",
            Some(currency),
            "Account balance",
            None,
            &observed_at,
        ));
        metrics.push(metric(
            "granted_balance",
            granted,
            "currency",
            Some(currency),
            "Account balance",
            None,
            &observed_at,
        ));
    }
    Ok(FetchResult {
        provider_id: "deepseek".into(),
        status: if json
            .get("is_available")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "Live"
        } else {
            "Unavailable"
        }
        .into(),
        metrics,
        history_buckets: Vec::new(),
        observed_at,
        diagnostic:
            "Balance components are stored as snapshots and never converted into inferred spend."
                .into(),
    })
}

fn parse_openrouter(json: &Value, management: bool) -> AppResult<FetchResult> {
    let data = json.get("data").unwrap_or(json);
    let observed_at = observed();
    let (metrics, diagnostic) = if management {
        let total = number(data.get("total_credits"));
        let usage = number(data.get("total_usage"));
        let remaining = total - usage;
        (
            vec![
                metric(
                    "account_credits",
                    total,
                    "currency",
                    Some("USD"),
                    "Account credits",
                    Some("Lifetime purchased"),
                    &observed_at,
                ),
                metric(
                    "account_usage",
                    usage,
                    "currency",
                    Some("USD"),
                    "Account credits",
                    Some("Lifetime"),
                    &observed_at,
                ),
                metric(
                    "remaining_credits",
                    remaining,
                    "currency",
                    Some("USD"),
                    "Account credits",
                    Some("Calculated from purchased credits minus total usage"),
                    &observed_at,
                ),
            ],
            "Account-level credits from a management credential. Remaining credits are calculated from official purchased-credit and total-usage fields.",
        )
    } else {
        let usage = number(data.get("usage"));
        let mut metrics = vec![metric(
            "key_usage",
            usage,
            "currency",
            Some("USD"),
            "API key",
            Some("Lifetime"),
            &observed_at,
        )];
        if let Some(limit) = optional_number(data.get("limit")) {
            metrics.push(metric(
                "key_limit",
                limit,
                "currency",
                Some("USD"),
                "API key",
                data.get("limit_reset").and_then(Value::as_str),
                &observed_at,
            ));
        }
        if let Some(remaining) = optional_number(data.get("limit_remaining")) {
            metrics.push(metric(
                "key_limit_remaining",
                remaining,
                "currency",
                Some("USD"),
                "API key",
                data.get("limit_reset").and_then(Value::as_str),
                &observed_at,
            ));
        }
        (
            metrics,
            "Key-level usage and limit. This is not account-level credit balance.",
        )
    };
    Ok(FetchResult {
        provider_id: "openrouter".into(),
        status: "Live".into(),
        metrics,
        history_buckets: Vec::new(),
        observed_at,
        diagnostic: diagnostic.into(),
    })
}

fn parse_google_ai_studio(json: &Value) -> AppResult<FetchResult> {
    let models = json
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidResponse("Google AI Studio models missing".into()))?;
    let generate_models = models
        .iter()
        .filter(|model| {
            model
                .get("supportedGenerationMethods")
                .and_then(Value::as_array)
                .is_some_and(|methods| {
                    methods
                        .iter()
                        .any(|method| method.as_str() == Some("generateContent"))
                })
        })
        .count();
    let observed_at = observed();
    Ok(FetchResult {
        provider_id: "google-ai-studio".into(),
        status: "Live".into(),
        metrics: vec![metric(
            "accessible_models",
            generate_models as f64,
            "models",
            None,
            "Google Cloud project API access",
            None,
            &observed_at,
        )],
        history_buckets: Vec::new(),
        observed_at,
        diagnostic: "API key validation and accessible generateContent models. Gemini API quotas are project-level, not key-level; balance, spend and usage remain available only in Google AI Studio.".into(),
    })
}

fn metric(
    kind: &str,
    value: f64,
    unit: &str,
    currency: Option<&str>,
    scope: &str,
    window: Option<&str>,
    observed_at: &str,
) -> Metric {
    Metric {
        kind: kind.into(),
        value,
        unit: unit.into(),
        currency: currency.map(str::to_owned),
        scope: scope.into(),
        window: window.map(str::to_owned),
        reset_time: None,
        observed_at: observed_at.into(),
        source: FetchStrategy::OfficialApi,
    }
}

fn openai_cost_history(costs: &Value) -> Vec<HistoryBucket> {
    costs
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|bucket| {
            let start = bucket.get("start_time")?.as_i64()?;
            let end = bucket.get("end_time")?.as_i64()?;
            let value = bucket
                .get("results")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|result| number(result.get("amount").and_then(|amount| amount.get("value"))))
                .sum();
            let currency = bucket
                .get("results")
                .and_then(Value::as_array)
                .and_then(|results| results.first())
                .and_then(|result| result.get("amount"))
                .and_then(|amount| amount.get("currency"))
                .and_then(Value::as_str)
                .unwrap_or("USD");
            Some(HistoryBucket {
                metric_kind: "cost".into(),
                bucket_start: chrono::DateTime::from_timestamp(start, 0)?.to_rfc3339(),
                bucket_end: chrono::DateTime::from_timestamp(end, 0)?.to_rfc3339(),
                value,
                unit: "currency".into(),
                currency: Some(currency.into()),
                source: FetchStrategy::OfficialApi,
            })
        })
        .collect()
}

fn number(value: Option<&Value>) -> f64 {
    value
        .and_then(|item| item.as_f64().or_else(|| item.as_str()?.parse::<f64>().ok()))
        .unwrap_or(0.0)
}

fn optional_number(value: Option<&Value>) -> Option<f64> {
    value.and_then(|item| item.as_f64().or_else(|| item.as_str()?.parse::<f64>().ok()))
}

fn sum_field(root: &Value, field: &str) -> f64 {
    root.get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|bucket| {
            bucket
                .get("results")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .map(|result| number(result.get(field)))
        .sum()
}

fn sum_fields(root: &Value, fields: &[&str]) -> f64 {
    fields.iter().map(|field| sum_field(root, field)).sum()
}

fn sum_nested_number(root: &Value, path: &[&str]) -> f64 {
    root.get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|bucket| {
            bucket
                .get(path[0])
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .map(|value| number(value.get(path[1]).and_then(|v| v.get(path[2]))))
        .sum()
}

trait DateExt {
    fn with_day(self, day: u32) -> Option<Self>
    where
        Self: Sized;
}
impl DateExt for chrono::NaiveDate {
    fn with_day(self, day: u32) -> Option<Self> {
        chrono::Datelike::with_day(&self, day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deepseek_keeps_balance_components_separate() {
        let result = parse_deepseek(&json!({"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"82.31","topped_up_balance":"70.00","granted_balance":"12.31"}]})).unwrap();
        assert_eq!(result.metrics.len(), 3);
        assert!(!result
            .metrics
            .iter()
            .any(|metric| metric.kind.contains("spend")));
    }

    #[test]
    fn openrouter_distinguishes_key_and_account_scope() {
        let key = parse_openrouter(&json!({"data":{"usage":25.75,"limit":100.5,"limit_remaining":74.75,"limit_reset":"monthly"}}), false).unwrap();
        let account = parse_openrouter(
            &json!({"data":{"total_credits":100.5,"total_usage":25.75}}),
            true,
        )
        .unwrap();
        assert!(key.metrics.iter().all(|metric| metric.scope == "API key"));
        assert!(account
            .metrics
            .iter()
            .all(|metric| metric.scope == "Account credits"));
        assert_eq!(
            account
                .metrics
                .iter()
                .find(|metric| metric.kind == "remaining_credits")
                .unwrap()
                .value,
            74.75
        );
        assert_eq!(key.metrics.len(), 3);
    }

    #[test]
    fn openai_aggregates_costs_tokens_and_requests() {
        let costs = json!({"data":[{"start_time":1786924800,"end_time":1787011200,"results":[{"amount":{"value":1.25,"currency":"USD"}},{"amount":{"value":2.75,"currency":"USD"}}]}]});
        let usage = json!({"data":[{"results":[{"input_tokens":100,"output_tokens":40,"num_model_requests":3}]}]});
        let result = parse_openai(&costs, &usage).unwrap();
        assert_eq!(result.metrics[0].value, 4.0);
        assert_eq!(result.metrics[1].value, 140.0);
        assert_eq!(result.metrics[2].value, 3.0);
        assert_eq!(result.metrics[0].window.as_deref(), Some("Month to date"));
        assert_eq!(result.history_buckets.len(), 1);
        assert_eq!(result.history_buckets[0].value, 4.0);
    }

    #[test]
    fn google_ai_studio_reports_access_without_inventing_usage() {
        let result = parse_google_ai_studio(&serde_json::json!({
            "models": [
                {"name":"models/gemini-pro","supportedGenerationMethods":["generateContent"]},
                {"name":"models/embedding","supportedGenerationMethods":["embedContent"]}
            ]
        }))
        .unwrap();
        assert_eq!(result.metrics[0].kind, "accessible_models");
        assert_eq!(result.metrics[0].value, 1.0);
        assert!(!result
            .metrics
            .iter()
            .any(|metric| { metric.kind.contains("balance") || metric.kind.contains("spend") }));
    }
}
