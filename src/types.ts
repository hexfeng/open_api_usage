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

export interface Metric {
  id: string;
  label: string;
  value: string;
  kind: "money" | "tokens" | "requests" | "quota";
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
  provider: "openai" | "deepseek" | "openrouter";
  name: string;
  type: "API Platform";
  status: Status;
  source: MetricSource;
  scope: string;
  updatedLabel: string;
  updatedAt: string;
  officialUrl: string;
  credentialHint: string;
  enabled: boolean;
  metrics: Metric[];
  history: number[];
}

export interface SubscriptionAccount {
  id: string;
  provider: "chatgpt" | "gemini";
  name: string;
  plan: string;
  status: Status;
  source: MetricSource;
  scope: string;
  updatedLabel: string;
  officialUrl: string;
  monthlyPrice: string;
  renewalDate: string;
  metrics: Metric[];
}

export interface AppSettings {
  apiRefreshMinutes: number;
  localRefreshMinutes: number;
  startOnLogin: boolean;
  minimizeToTray: boolean;
  historyRetentionDays: number;
}
