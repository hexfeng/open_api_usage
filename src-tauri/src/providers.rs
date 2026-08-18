use chrono::Utc;
use reqwest::Client;
use serde_json::Value;

use crate::{
    error::{AppError, AppResult},
    models::{FetchResult, FetchStrategy, Metric, ProviderCapability, ProviderDescriptor},
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
        },
        ProviderDescriptor {
            id: "deepseek".into(),
            display_name: "DeepSeek API".into(),
            strategy: FetchStrategy::OfficialApi,
            capabilities: vec![ProviderCapability::Balance],
            official_url: "https://platform.deepseek.com/usage".into(),
            experimental: false,
        },
        ProviderDescriptor {
            id: "openrouter".into(),
            display_name: "OpenRouter".into(),
            strategy: FetchStrategy::OfficialApi,
            capabilities: vec![ProviderCapability::Balance, ProviderCapability::Costs],
            official_url: "https://openrouter.ai/activity".into(),
            experimental: false,
        },
        ProviderDescriptor {
            id: "chatgpt-codex".into(),
            display_name: "ChatGPT / Codex".into(),
            strategy: FetchStrategy::CliOauth,
            capabilities: vec![ProviderCapability::Quota, ProviderCapability::PlanMetadata],
            official_url: "https://chatgpt.com/#settings/Subscription".into(),
            experimental: false,
        },
        ProviderDescriptor {
            id: "google-gemini-cli".into(),
            display_name: "Google AI / Gemini CLI".into(),
            strategy: FetchStrategy::BrowserSessionExperimental,
            capabilities: vec![ProviderCapability::Quota, ProviderCapability::PlanMetadata],
            official_url: "https://one.google.com/explore-plan/gemini-advanced".into(),
            experimental: true,
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
        "chatgpt-codex" => Err(AppError::InvalidRequest("Codex quota uses the local CLI/OAuth connector; no API key is accepted".into())),
        "google-gemini-cli" => Err(AppError::InvalidRequest("Gemini CLI quota requires explicit browser-session consent; raw cookies are never accepted by this command".into())),
        other => Err(AppError::UnsupportedProvider(other.into())),
    }
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
        diagnostic: "Organization-level costs and completions usage. Requires an OpenAI Admin Key."
            .into(),
        metrics: vec![
            metric(
                "cost",
                cost,
                "currency",
                Some("USD"),
                "Organization",
                &observed_at,
            ),
            metric(
                "tokens",
                tokens,
                "tokens",
                None,
                "Organization",
                &observed_at,
            ),
            metric(
                "requests",
                requests,
                "requests",
                None,
                "Organization",
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
            &observed_at,
        ));
        metrics.push(metric(
            "topped_up_balance",
            topped_up,
            "currency",
            Some(currency),
            "Account balance",
            &observed_at,
        ));
        metrics.push(metric(
            "granted_balance",
            granted,
            "currency",
            Some(currency),
            "Account balance",
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
        (
            vec![
                metric(
                    "account_credits",
                    total,
                    "currency",
                    Some("USD"),
                    "Account credits",
                    &observed_at,
                ),
                metric(
                    "account_usage",
                    usage,
                    "currency",
                    Some("USD"),
                    "Account credits",
                    &observed_at,
                ),
            ],
            "Account-level credits from a management credential.",
        )
    } else {
        let usage = number(data.get("usage"));
        let limit = number(data.get("limit"));
        (
            vec![
                metric(
                    "key_usage",
                    usage,
                    "currency",
                    Some("USD"),
                    "API key",
                    &observed_at,
                ),
                metric(
                    "key_limit",
                    limit,
                    "currency",
                    Some("USD"),
                    "API key",
                    &observed_at,
                ),
            ],
            "Key-level usage and limit. This is not account-level credit balance.",
        )
    };
    Ok(FetchResult {
        provider_id: "openrouter".into(),
        status: "Live".into(),
        metrics,
        observed_at,
        diagnostic: diagnostic.into(),
    })
}

fn metric(
    kind: &str,
    value: f64,
    unit: &str,
    currency: Option<&str>,
    scope: &str,
    observed_at: &str,
) -> Metric {
    Metric {
        kind: kind.into(),
        value,
        unit: unit.into(),
        currency: currency.map(str::to_owned),
        scope: scope.into(),
        window: Some("Current month".into()),
        reset_time: None,
        observed_at: observed_at.into(),
        source: FetchStrategy::OfficialApi,
    }
}

fn number(value: Option<&Value>) -> f64 {
    value
        .and_then(|item| item.as_f64().or_else(|| item.as_str()?.parse::<f64>().ok()))
        .unwrap_or(0.0)
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
        let key = parse_openrouter(&json!({"data":{"usage":25.75,"limit":100.5}}), false).unwrap();
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
    }

    #[test]
    fn openai_aggregates_costs_tokens_and_requests() {
        let costs =
            json!({"data":[{"results":[{"amount":{"value":1.25}},{"amount":{"value":2.75}}]}]});
        let usage = json!({"data":[{"results":[{"input_tokens":100,"output_tokens":40,"num_model_requests":3}]}]});
        let result = parse_openai(&costs, &usage).unwrap();
        assert_eq!(result.metrics[0].value, 4.0);
        assert_eq!(result.metrics[1].value, 140.0);
        assert_eq!(result.metrics[2].value, 3.0);
    }
}
