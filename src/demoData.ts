import type { AppSettings, ProviderAccount, SubscriptionAccount } from "./types";

const observedAt = "2026-08-17T14:28:00-04:00";

export const demoProviders: ProviderAccount[] = [
  {
    id: "openai-primary",
    provider: "openai",
    name: "OpenAI API",
    type: "API Platform",
    status: "Live",
    source: "Official API",
    scope: "Organization",
    updatedLabel: "2 min ago",
    updatedAt: observedAt,
    officialUrl: "https://platform.openai.com/usage",
    credentialHint: "Admin key ·•••• 7K2Q",
    enabled: true,
    history: [11, 17, 13, 22, 18, 26, 24],
    metrics: [
      { id: "openai-cost", label: "This month", value: "$18.42", kind: "money", currency: "USD", scope: "Organization", window: "Aug 1–17", observedAt, source: "Official API" },
      { id: "openai-tokens", label: "Tokens", value: "4.2M", kind: "tokens", unit: "tokens", scope: "Organization", window: "Aug 1–17", observedAt, source: "Official API" },
      { id: "openai-requests", label: "Requests", value: "1,284", kind: "requests", unit: "requests", scope: "Organization", window: "Aug 1–17", observedAt, source: "Official API" },
    ],
  },
  {
    id: "deepseek-primary",
    provider: "deepseek",
    name: "DeepSeek API",
    type: "API Platform",
    status: "Live",
    source: "Official API",
    scope: "Account balance",
    updatedLabel: "3 min ago",
    updatedAt: "2026-08-17T14:27:00-04:00",
    officialUrl: "https://platform.deepseek.com/usage",
    credentialHint: "API key ·•••• 19AF",
    enabled: true,
    history: [78, 79, 80, 80, 81, 82, 82],
    metrics: [
      { id: "deepseek-total", label: "Total", value: "¥82.31", kind: "money", currency: "CNY", scope: "Account balance", observedAt, source: "Official API" },
      { id: "deepseek-topup", label: "Topped-up", value: "¥70.00", kind: "money", currency: "CNY", scope: "Account balance", observedAt, source: "Official API" },
      { id: "deepseek-granted", label: "Granted", value: "¥12.31", kind: "money", currency: "CNY", scope: "Account balance", observedAt, source: "Official API" },
    ],
  },
  {
    id: "openrouter-primary",
    provider: "openrouter",
    name: "OpenRouter",
    type: "API Platform",
    status: "Stale",
    source: "Official API",
    scope: "Account credits",
    updatedLabel: "34 min ago",
    updatedAt: "2026-08-17T13:56:00-04:00",
    officialUrl: "https://openrouter.ai/activity",
    credentialHint: "Management key ·•••• D81C",
    enabled: true,
    history: [8, 12, 9, 14, 19, 23, 26],
    metrics: [
      { id: "openrouter-credits", label: "Credits", value: "$100.50", kind: "money", currency: "USD", scope: "Account credits", observedAt, source: "Official API" },
      { id: "openrouter-usage", label: "Usage", value: "$25.75", kind: "money", currency: "USD", scope: "Account credits", window: "Lifetime", observedAt, source: "Official API" },
    ],
  },
];

export const demoSubscriptions: SubscriptionAccount[] = [
  {
    id: "chatgpt-plus",
    provider: "chatgpt",
    name: "ChatGPT Plus",
    plan: "Codex quota",
    status: "Live",
    source: "CLI/OAuth",
    scope: "Codex only",
    updatedLabel: "8 min ago",
    officialUrl: "https://chatgpt.com/#settings/Subscription",
    monthlyPrice: "$20 / month",
    renewalDate: "Renews Sep 3",
    metrics: [
      { id: "codex-5h", label: "5-hour", value: "42%", kind: "quota", unit: "% used", scope: "Codex only", window: "Rolling 5 hours", resetTime: "2h 18m", observedAt, source: "CLI/OAuth" },
      { id: "codex-weekly", label: "Weekly", value: "61%", kind: "quota", unit: "% used", scope: "Codex only", window: "Weekly", observedAt, source: "CLI/OAuth" },
    ],
  },
  {
    id: "google-ai-pro",
    provider: "gemini",
    name: "Google AI Pro",
    plan: "Gemini CLI quota",
    status: "Experimental",
    source: "Browser session · Experimental",
    scope: "Gemini CLI only",
    updatedLabel: "18 min ago",
    officialUrl: "https://one.google.com/explore-plan/gemini-advanced",
    monthlyPrice: "$19.99 / month",
    renewalDate: "Renews Sep 12",
    metrics: [
      { id: "gemini-quota", label: "Quota", value: "72%", kind: "quota", unit: "% used", scope: "Gemini CLI only", window: "Current window", resetTime: "1h 12m", observedAt, source: "Browser session · Experimental" },
    ],
  },
];

export const defaultSettings: AppSettings = {
  apiRefreshMinutes: 15,
  localRefreshMinutes: 30,
  startOnLogin: true,
  minimizeToTray: true,
  historyRetentionDays: 90,
};
