import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { disable, enable } from "@tauri-apps/plugin-autostart";
import type { FetchStrategy as NativeStrategy } from "./nativeTypes";
import type { MetricSource, ProviderAccount, Status } from "./types";

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
  observedAt: string;
  diagnostic: string;
}

export interface NativeAccount {
  id: string;
  providerId: ProviderAccount["provider"];
  displayName: string;
  credentialRef?: string;
  credentialKind?: string;
  enabled: boolean;
  source: NativeStrategy;
  scope: string;
}

export interface AccountRequest {
  providerId: ProviderAccount["provider"];
  displayName: string;
  credential: string;
  credentialKind?: string;
}

export const isTauri = "__TAURI_INTERNALS__" in window;

const sourceLabel = (source: NativeStrategy): MetricSource =>
  source === "cliOauth" ? "CLI/OAuth" : source === "browserSessionExperimental" ? "Browser session · Experimental" : source === "manual" ? "Manual" : "Official API";

const providerUrl: Record<ProviderAccount["provider"], string> = {
  openai: "https://platform.openai.com/usage",
  deepseek: "https://platform.deepseek.com/usage",
  openrouter: "https://openrouter.ai/activity",
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
  key_usage: "Key usage",
  key_limit: "Key limit",
};

function formatMetric(metric: NativeMetric) {
  if (metric.currency) {
    const symbol = metric.currency === "CNY" ? "¥" : "$";
    return `${symbol}${metric.value.toFixed(2)}`;
  }
  if (metric.kind === "tokens") return metric.value >= 1_000_000 ? `${(metric.value / 1_000_000).toFixed(1)}M` : metric.value.toLocaleString();
  return metric.value.toLocaleString();
}

export function toProviderAccount(account: NativeAccount, result?: NativeFetchResult, failed = false): ProviderAccount {
  const source = sourceLabel(account.source);
  const observedAt = result?.observedAt ?? new Date().toISOString();
  return {
    id: account.id,
    provider: account.providerId,
    name: account.displayName,
    type: "API Platform",
    status: failed ? "Stale" : result?.status ?? "Refreshing",
    source,
    scope: account.scope,
    updatedLabel: failed ? "Refresh failed · last success kept" : result ? "Just now" : "Connecting",
    updatedAt: observedAt,
    officialUrl: providerUrl[account.providerId],
    credentialHint: account.credentialRef ? "Windows Credential Manager" : "Credential required",
    enabled: account.enabled,
    history: [],
    metrics: (result?.metrics ?? []).map((metric, index) => ({
      id: `${account.id}-${metric.kind}-${index}`,
      label: metricLabel[metric.kind] ?? metric.kind,
      value: formatMetric(metric),
      kind: metric.kind.includes("token") ? "tokens" : metric.kind.includes("request") ? "requests" : "money",
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
  const accounts = await invoke<NativeAccount[]>("list_accounts");
  const results = await Promise.all(accounts.map(async (account) => {
    try {
      const result = await invoke<NativeFetchResult>("refresh_account", { accountId: account.id });
      return toProviderAccount(account, result);
    } catch {
      return toProviderAccount(account, undefined, true);
    }
  }));
  return results;
}

const nativeSource = (source: MetricSource): NativeStrategy =>
  source === "CLI/OAuth" ? "cliOauth" : source === "Browser session · Experimental" ? "browserSessionExperimental" : source === "Manual" ? "manual" : "officialApi";

export async function refreshConnectedAccount(account: ProviderAccount): Promise<ProviderAccount> {
  const native: NativeAccount = {
    id: account.id,
    providerId: account.provider,
    displayName: account.name,
    enabled: account.enabled,
    source: nativeSource(account.source),
    scope: account.scope,
  };
  try {
    const result = await invoke<NativeFetchResult>("refresh_account", { accountId: account.id });
    return toProviderAccount(native, result);
  } catch {
    return { ...account, status: "Stale", updatedLabel: "Refresh failed · last success kept" };
  }
}

export async function testConnection(request: AccountRequest): Promise<NativeFetchResult> {
  return invoke("test_provider", { providerId: request.providerId, credential: request.credential, credentialKind: request.credentialKind });
}

export async function saveConnectedAccount(request: AccountRequest): Promise<ProviderAccount> {
  const account = await invoke<NativeAccount>("save_account", { request });
  const result = await invoke<NativeFetchResult>("refresh_account", { accountId: account.id });
  return toProviderAccount(account, result);
}

export async function removeConnectedAccount(accountId: string) {
  if (isTauri) await invoke("delete_account", { accountId });
}

export async function deleteAllLocalData() {
  if (isTauri) await invoke("delete_local_data");
}

export async function setStartOnLogin(value: boolean) {
  if (!isTauri) return;
  if (value) await enable();
  else await disable();
}

export async function openOfficialUrl(url: string) {
  if (isTauri) await openUrl(url);
}
