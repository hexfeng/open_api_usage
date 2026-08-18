import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { FetchStrategy as NativeStrategy } from "./nativeTypes";
import type { AppSettings, MetricSource, ProviderAccount, ProviderId, Status } from "./types";

export interface NativeMetric {
  kind: string;
  value: number;
  unit: string;
  currency?: string;
  scope: string;
  window?: string;
  resetTime?: string;
  observedAt: string;
  source: NativeStrategy;
}

export interface NativeFetchResult {
  providerId: string;
  status: Status;
  metrics: NativeMetric[];
  historyBuckets: unknown[];
  observedAt: string;
  diagnostic: string;
}

interface NativeHistorySeries {
  label: string;
  basis: string;
  unit: string;
  currency?: string;
  points: { observedAt: string; value: number }[];
}

interface NativeAccountSnapshot {
  account: NativeAccount;
  result?: NativeFetchResult;
  history: NativeHistorySeries;
  lastError?: string;
}

export interface NativeAccount {
  id: string;
  providerId: ProviderId;
  displayName: string;
  credentialRef?: string;
  credentialKind?: string;
  enabled: boolean;
  source: NativeStrategy;
  scope: string;
  accountType: "apiPlatform" | "subscription";
  planName?: string;
  monthlyPrice?: string;
  renewalDate?: string;
}

export interface AccountRequest {
  providerId: ProviderId;
  displayName: string;
  credential: string;
  credentialKind?: string;
  planName?: string;
  monthlyPrice?: string;
  renewalDate?: string;
}

export interface UpdateAccountRequest {
  accountId: string;
  displayName: string;
  credential?: string;
  credentialKind?: string;
  planName?: string;
  monthlyPrice?: string;
  renewalDate?: string;
}

export const isTauri = "__TAURI_INTERNALS__" in window;

const sourceLabel = (source: NativeStrategy): MetricSource =>
  source === "cliOauth" ? "CLI/OAuth" : source === "browserSessionExperimental" ? "Browser session · Experimental" : source === "manual" ? "Manual" : "Official API";

const providerUrl: Record<ProviderId, string> = {
  openai: "https://platform.openai.com/usage",
  deepseek: "https://platform.deepseek.com/usage",
  openrouter: "https://openrouter.ai/activity",
  "google-ai-studio": "https://aistudio.google.com/usage",
  "chatgpt-codex": "https://chatgpt.com/#settings/Subscription",
  "google-gemini-cli": "https://one.google.com/explore-plan/gemini-advanced",
};

const metricLabel: Record<string, string> = {
  cost: "This month",
  tokens: "Tokens",
  requests: "Requests",
  total_balance: "Total",
  topped_up_balance: "Topped-up",
  granted_balance: "Granted",
  account_credits: "Credits",
  account_usage: "Usage",
  remaining_credits: "Remaining",
  key_usage: "Key usage",
  key_limit: "Key limit",
  key_limit_remaining: "Limit remaining",
  accessible_models: "Models",
  codex_credits: "Credits",
};

function formatMetric(metric: NativeMetric) {
  if (metric.currency) {
    const symbol = metric.currency === "CNY" ? "¥" : "$";
    return `${symbol}${metric.value.toFixed(2)}`;
  }
  if (metric.kind === "tokens") return metric.value >= 1_000_000 ? `${(metric.value / 1_000_000).toFixed(1)}M` : metric.value.toLocaleString();
  if (metric.unit === "percent_used") return `${metric.value.toFixed(0)}%`;
  return metric.value.toLocaleString();
}

function relativeTime(value: string) {
  const elapsed = Math.max(0, Date.now() - new Date(value).getTime());
  const minutes = Math.floor(elapsed / 60_000);
  if (minutes < 1) return "Just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hr ago`;
  return `${Math.floor(hours / 24)} d ago`;
}

export function toProviderAccount(snapshot: NativeAccountSnapshot): ProviderAccount {
  const { account, result, history, lastError } = snapshot;
  const source = sourceLabel(account.source);
  const observedAt = result?.observedAt ?? new Date().toISOString();
  const hasCachedMetrics = Boolean(result?.metrics.length);
  const authenticationFailure = /HTTP (401|403)|not signed in|OAuth credentials|refresh token|authentication/i.test(lastError ?? "");
  const failedStatus: Status = authenticationFailure
    ? "Authentication required"
    : "Unavailable";
  return {
    id: account.id,
    provider: account.providerId,
    name: account.displayName,
    type: account.accountType === "subscription" ? "Subscription" : "API Platform",
    status: !account.enabled ? "Unavailable" : lastError && hasCachedMetrics ? "Stale" : lastError ? failedStatus : result?.status ?? "Refreshing",
    source,
    scope: account.scope,
    updatedLabel: lastError && hasCachedMetrics ? `${relativeTime(observedAt)} · last success` : result ? relativeTime(observedAt) : "No successful refresh",
    updatedAt: observedAt,
    officialUrl: providerUrl[account.providerId],
    credentialHint: account.credentialRef
      ? `${account.credentialKind === "management" ? "Management key" : account.providerId === "openai" ? "Admin key" : "API key"} · Windows Credential Manager`
      : account.providerId === "chatgpt-codex"
        ? "Local Codex app-server"
        : account.providerId === "google-gemini-cli"
          ? "Local Gemini CLI OAuth"
          : "Credential required",
    credentialKind: account.credentialKind,
    enabled: account.enabled,
    history: history.points.map((point) => point.value),
    historyLabel: history.label,
    historyBasis: history.basis,
    lastError,
    diagnostic: result?.diagnostic,
    plan: account.planName,
    monthlyPrice: account.monthlyPrice,
    renewalDate: account.renewalDate,
    metrics: (result?.metrics ?? []).map((metric, index) => ({
      id: `${account.id}-${metric.kind}-${index}`,
      label: metric.kind.startsWith("codex_quota")
        ? metric.window ?? "Codex quota"
        : metric.kind.startsWith("gemini_quota")
          ? metric.scope.split(" · ").at(-1) ?? "Gemini quota"
          : metricLabel[metric.kind] ?? metric.kind,
      value: formatMetric(metric),
      numericValue: metric.value,
      metricKind: metric.kind,
      kind: metric.unit === "percent_used" ? "quota" : metric.kind.includes("token") ? "tokens" : metric.kind.includes("request") ? "requests" : metric.unit === "models" ? "count" : "money",
      unit: metric.unit,
      currency: metric.currency === "CNY" ? "CNY" : metric.currency ? "USD" : undefined,
      scope: metric.scope,
      window: metric.window,
      resetTime: metric.resetTime,
      observedAt: metric.observedAt,
      source,
    })),
  };
}

export async function loadConnectedAccounts(): Promise<ProviderAccount[]> {
  if (!isTauri) return [];
  const snapshots = await invoke<NativeAccountSnapshot[]>("list_account_snapshots");
  return snapshots.map(toProviderAccount);
}

export async function refreshConnectedAccount(account: ProviderAccount): Promise<ProviderAccount> {
  try {
    const snapshot = await invoke<NativeAccountSnapshot>("refresh_account", { accountId: account.id });
    return toProviderAccount(snapshot);
  } catch {
    return { ...account, status: "Stale", updatedLabel: "Refresh failed · last success kept" };
  }
}

export async function testConnection(request: AccountRequest): Promise<NativeFetchResult> {
  return invoke("test_provider", { providerId: request.providerId, credential: request.credential, credentialKind: request.credentialKind });
}

export async function saveConnectedAccount(request: AccountRequest): Promise<ProviderAccount> {
  const snapshot = await invoke<NativeAccountSnapshot>("save_account", { request });
  return toProviderAccount(snapshot);
}

export async function updateConnectedAccount(request: UpdateAccountRequest): Promise<ProviderAccount> {
  const snapshot = await invoke<NativeAccountSnapshot>("update_account", { request });
  return toProviderAccount(snapshot);
}

export async function setConnectedAccountEnabled(accountId: string, enabled: boolean): Promise<ProviderAccount> {
  const snapshot = await invoke<NativeAccountSnapshot>("set_account_enabled", { accountId, enabled });
  return toProviderAccount(snapshot);
}

export async function removeConnectedAccount(accountId: string) {
  if (isTauri) await invoke("delete_account", { accountId });
}

export async function deleteAllLocalData() {
  if (isTauri) await invoke("delete_local_data");
}

export async function loadAppSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_app_settings");
}

export async function saveAppSettings(settings: AppSettings): Promise<AppSettings> {
  return invoke<AppSettings>("update_app_settings", { settings });
}

export async function listenForAccountUpdates(onUpdate: (account: ProviderAccount) => void): Promise<UnlistenFn> {
  return listen<NativeAccountSnapshot>("account-updated", (event) => onUpdate(toProviderAccount(event.payload)));
}

export async function openOfficialUrl(url: string) {
  if (isTauri) await openUrl(url);
}
