import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { toProviderAccount, type NativeAccountSnapshot } from "./bridge";

describe("AI Usage Dashboard", () => {
  beforeEach(() => localStorage.clear());
  afterEach(cleanup);

  it("switches between list and card layouts", () => {
    render(<App />);
    const cards = screen.getByRole("button", { name: "Card layout" });
    fireEvent.click(cards);
    expect(cards).toHaveAttribute("aria-pressed", "true");
    expect(localStorage.getItem("aud-layout")).toBe("cards");
  });

  it("applies and remembers the light theme", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Light/i }));
    expect(document.documentElement).toHaveAttribute("data-theme", "light");
    expect(localStorage.getItem("aud-theme")).toBe("light");
  });

  it("navigates to account and settings views", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Accounts" }));
    expect(screen.getByRole("heading", { name: "Accounts" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
  });

  it("keeps API and subscription scope labels explicit", () => {
    render(<App />);
    expect(document.body.textContent).toContain("Organization");
    expect(document.body.textContent).toContain("Codex only");
    expect(document.body.textContent).toContain("Gemini CLI only");
  });

  it("renders subscription bars as remaining quota", () => {
    render(<App />);
    const value = screen.getByText("58%");
    const quota = value.closest(".quota");
    expect(quota).toHaveTextContent("5-hour remaining");
    expect(quota?.querySelector(".progress-track > span")).toHaveStyle({ width: "58%" });
  });

  it("derives account summaries from provider metrics", () => {
    render(<App />);
    expect(screen.getByText("$74.75 · ¥82.31")).toBeInTheDocument();
    expect(screen.getAllByText("$18.42")).toHaveLength(2);
    expect(screen.getByText("4/4")).toBeInTheDocument();
    expect(document.body.textContent).not.toContain("$42.18");
  });

  it("shows the stored history basis in account details", () => {
    render(<App />);
    fireEvent.click(screen.getByText("OpenAI API").closest("article")!);
    expect(screen.getByRole("heading", { name: "Daily spend" })).toBeInTheDocument();
    expect(screen.getByText("Provider history buckets")).toBeInTheDocument();
  });

  it("edits an account name without replacing its credential", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Accounts" }));
    fireEvent.click(screen.getByRole("button", { name: "Edit OpenAI API" }));
    const name = screen.getByLabelText("Account name");
    fireEvent.change(name, { target: { value: "Primary OpenAI" } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    expect(screen.getByRole("heading", { name: "Primary OpenAI" })).toBeInTheDocument();
  });

  it("keeps Google AI Studio API and Gemini CLI as separate account types", () => {
    render(<App />);
    expect(screen.getByText("Google AI Studio API")).toBeInTheDocument();
    expect(screen.getByText("Google AI Pro")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Accounts" }));
    expect(screen.getByRole("table", { name: "API platform accounts" })).toHaveTextContent("Google AI Studio API");
    expect(screen.getByRole("table", { name: "Subscription accounts" })).toHaveTextContent("Google AI Pro");
  });

  it("edits manual subscription metadata independently of automatic quota", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Accounts" }));
    fireEvent.click(screen.getByRole("button", { name: "Edit ChatGPT Plus" }));
    fireEvent.change(screen.getByLabelText("Plan name · Manual"), { target: { value: "ChatGPT Pro" } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    expect(screen.getByRole("heading", { name: "ChatGPT Plus" })).toBeInTheDocument();
    expect(document.body.textContent).toContain("ChatGPT Pro");
  });

  it("requires explicit confirmation for a detected Codex identity", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Accounts" }));
    fireEvent.click(screen.getByRole("button", { name: /Add account/i }));
    fireEvent.change(screen.getByLabelText("Provider"), { target: { value: "chatgpt-codex" } });
    fireEvent.click(screen.getByRole("button", { name: "Detect account" }));
    expect(await screen.findByText("user@example.com")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Sign in with another account" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Use this account" })).toBeEnabled();
    expect(document.body.textContent).toContain("Codex only");
  });

  it("makes OpenRouter PKCE the preferred path without inventing state validation", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Accounts" }));
    fireEvent.click(screen.getByRole("button", { name: /Add account/i }));
    fireEvent.change(screen.getByLabelText("Provider"), { target: { value: "openrouter" } });
    expect(screen.getByLabelText("Connection method")).toHaveValue("oauth");
    expect(screen.getByRole("button", { name: "Connect with OpenRouter" })).toBeInTheDocument();
    expect(document.body.textContent).toContain("does not define OAuth state");
    expect(screen.queryByLabelText("API Key")).not.toBeInTheDocument();
  });

  it("converts stored Codex used percentages to remaining percentages for display", () => {
    const snapshot: NativeAccountSnapshot = {
      account: {
        id: "codex", providerId: "chatgpt-codex", displayName: "Codex", enabled: true,
        source: "cliOauth", scope: "Codex only", accountType: "subscription", authMode: "sharedLocalSession", credentialOwner: "codex",
        consentedAt: "2026-08-19T00:00:00Z", lastValidatedAt: "2026-08-19T00:00:00Z",
      },
      result: {
        providerId: "chatgpt-codex", status: "Live", observedAt: "2026-08-19T00:00:00Z", diagnostic: "Codex quota", historyBuckets: [],
        metrics: [{ kind: "codex_quota_primary", value: 67, unit: "percent_used", scope: "Codex only", window: "7 days", observedAt: "2026-08-19T00:00:00Z", source: "cliOauth" }],
      },
      history: {
        label: "Codex quota usage", basis: "Local Codex quota snapshots", unit: "percent",
        points: [{ observedAt: "2026-08-18T00:00:00Z", value: 65 }, { observedAt: "2026-08-19T00:00:00Z", value: 67 }],
      },
    };
    const account = toProviderAccount(snapshot);
    expect(account.metrics[0]).toMatchObject({ label: "7 days remaining", value: "33%", numericValue: 33, unit: "percent_remaining" });
    expect(account.history).toEqual([35, 33]);
    expect(account.historyLabel).toBe("Codex quota remaining");
  });

  it("keeps cached data when authentication expires", () => {
    const snapshot: NativeAccountSnapshot = {
      account: {
        id: "cached", providerId: "deepseek", displayName: "DeepSeek", credentialRef: "opaque-ref", credentialKind: "api_key", enabled: true,
        source: "officialApi", scope: "Account balance", accountType: "apiPlatform", authMode: "pastedSecret", credentialOwner: "dashboard",
        consentedAt: "2026-08-19T00:00:00Z", lastValidatedAt: "2026-08-19T00:00:00Z",
      },
      result: {
        providerId: "deepseek", status: "Live", observedAt: "2026-08-19T00:00:00Z", diagnostic: "last success", historyBuckets: [],
        metrics: [{ kind: "total_balance", value: 10, unit: "currency", currency: "CNY", scope: "Account balance", observedAt: "2026-08-19T00:00:00Z", source: "officialApi" }],
      },
      history: { label: "Balance", basis: "Local snapshots", unit: "currency", points: [] },
      lastError: "Authentication required: DeepSeek rejected or revoked this credential",
    };
    const account = toProviderAccount(snapshot);
    expect(account.status).toBe("Authentication required");
    expect(account.metrics[0].value).toBe("¥10.00");
    expect(account.updatedAt).toBe("2026-08-19T00:00:00Z");
  });
});
