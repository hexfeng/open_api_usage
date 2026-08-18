export type Theme = "light" | "dark";
export type LayoutMode = "list" | "cards";
export type Page = "dashboard" | "accounts" | "settings";
export type Status =
  | "Live"
  | "Refreshing"
  | "Stale"
  | "Authentication required"
  | "Unavailable"
  | "Experimental";

export type MetricSource =
  | "Official API"
  | "CLI/OAuth"
  | "Browser session · Experimental"
  | "Manual";

export type ProviderId =
  | "openai"
  | "deepseek"
  | "openrouter"
  | "google-ai-studio"
  | "chatgpt-codex"
  | "google-gemini-cli";

export interface Metric {
  id: string;
  label: string;
  value: string;
  numericValue: number;
  metricKind: string;
  kind: "money" | "tokens" | "requests" | "quota" | "count";
  unit?: string;
  currency?: "USD" | "CNY";
  scope: string;
  window?: string;
  resetTime?: string;
  observedAt: string;
  source: MetricSource;
}

export interface ProviderAccount {
  id: string;
  provider: ProviderId;
  name: string;
  type: "API Platform" | "Subscription";
  status: Status;
  source: MetricSource;
  scope: string;
  updatedLabel: string;
  updatedAt: string;
  officialUrl: string;
  credentialHint: string;
  credentialKind?: string;
  enabled: boolean;
  metrics: Metric[];
  history: number[];
  historyLabel: string;
  historyBasis: string;
  lastError?: string;
  diagnostic?: string;
  plan?: string;
  monthlyPrice?: string;
  renewalDate?: string;
}

export type SubscriptionAccount = ProviderAccount;

export interface AppSettings {
  apiRefreshMinutes: number;
  localRefreshMinutes: number;
  startOnLogin: boolean;
  minimizeToTray: boolean;
  historyRetentionDays: number;
}
