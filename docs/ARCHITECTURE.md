# Architecture

## Runtime

```text
React UI
  └─ Tauri invoke bridge
      ├─ Provider descriptors and adapters
      ├─ Per-account scheduler
      ├─ Windows Credential Manager
      └─ SQLite
          ├─ accounts (credential references only)
          ├─ metrics
          ├─ provider_history_buckets
          ├─ balance_snapshots
          ├─ account_state (last success and last failure diagnostic)
          └─ app_settings
```

The browser build uses deterministic 2026-08-17 demo data for visual review. The Tauri runtime does not seed those records: it loads only locally connected accounts.

## Contracts

- `ProviderDescriptor`: labels, official URL, strategy, capabilities and experimental status.
- `AccountConnection`: local account category, source/scope, enabled state, optional credential reference and manual subscription metadata.
- `FetchStrategy`: Official API, CLI/OAuth, browser session experimental or manual.
- `FetchResult`: provider status, diagnostic and timestamped metrics.
- `Metric`: kind, value, unit/currency, scope, window/reset time, observed time and source.

## Provider isolation

Each account is refreshed in its own async task. Official APIs refresh every 15 minutes; local/OAuth/browser sources use 30 minutes. Failure schedules exponential backoff up to 15 minutes, retains stored metrics and is surfaced as `Stale` by the UI.

The configured intervals are read from SQLite on each successful scheduler cycle. The UI restores cached snapshots first, then refreshes enabled accounts in parallel. Scheduler results are emitted to the open window so tray-time background refreshes update the visible dashboard without a reload.

## Credentials

`keyring` uses the Windows native credential store. The SQLite `credential_ref` is a generated opaque identifier. Provider HTTP errors never include the submitted credential, and the log filter rejects credential/cookie-targeted records.

Authentication is provider-specific:

- OpenAI API, DeepSeek, Google AI Studio and the current OpenRouter implementation use test-before-save secrets owned by the dashboard.
- Codex and Gemini reuse credentials owned by their installed local clients; the dashboard stores no copied OAuth token.
- Product-grade Codex connection will use the official App Server browser or device-code flow when login is required, while still asking the user to confirm an already detected account.
- Product-grade OpenRouter connection should prefer the provider's PKCE flow for a normal user-controlled key; Management keys remain an explicit advanced credential.
- Removing an account from the dashboard does not revoke provider keys or sign out a shared local client.

The complete current/target state machines, ownership rules and provider flows are defined in [Authentication and Connection Design](AUTHENTICATION.md).

## Data semantics

- OpenAI requires an Admin Key and uses organization cost/completions usage endpoints.
- DeepSeek saves total, topped-up and granted balances as independent snapshots; it never derives spend from a balance delta.
- OpenRouter persists whether the connection is a normal key or management credential so scheduled refreshes preserve key-level versus account-level scope.
- Google AI Studio validates API access with `x-goog-api-key` and model listing. Its public API-key surface is not treated as a billing balance endpoint.
- ChatGPT/Codex currently launches the installed Codex app-server in read-only/untrusted mode and calls `account/read` plus `account/rateLimits/read`; it never reads Codex token files. The current reused session is an MVP implementation, not the final consent flow.
- Gemini CLI reads the existing local OAuth record from Windows Credential Manager (legacy file fallback), uses the installed official CLI package's OAuth client configuration to refresh an expired access token in memory, and calls `loadCodeAssist` plus `retrieveUserQuota`. No Google OAuth client credential is copied into this repository. The connector remains Experimental.
- Provider history buckets and balance snapshots are separate tables.
- OpenAI charts use provider daily cost buckets; DeepSeek charts use local total-balance snapshots; OpenRouter charts use local account- or key-usage snapshots; Codex charts use local primary-quota snapshots.
- Top-level balance totals keep USD and CNY separate. Month-to-date spend only includes metrics carrying the matching time window.

## Desktop behavior

The main window defaults to 1440×1024 with 960×700 minimum dimensions. Closing hides it to the tray. The tray can reopen or fully quit the app. Start-on-login is enabled on first launch and can be changed from Settings.

Account enablement, refresh intervals, tray-close behavior, autostart preference and history retention are persisted. Disabling an account removes it from scheduler work without deleting its cached metrics or credential reference.
