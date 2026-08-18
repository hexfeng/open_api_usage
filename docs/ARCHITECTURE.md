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
          └─ balance_snapshots
```

The browser build uses deterministic 2026-08-17 demo data for visual review. The Tauri runtime does not seed those records: it loads only locally connected accounts.

## Contracts

- `ProviderDescriptor`: labels, official URL, strategy, capabilities and experimental status.
- `AccountConnection`: local account configuration, source/scope, enabled state and credential reference.
- `FetchStrategy`: Official API, CLI/OAuth, browser session experimental or manual.
- `FetchResult`: provider status, diagnostic and timestamped metrics.
- `Metric`: kind, value, unit/currency, scope, window/reset time, observed time and source.

## Provider isolation

Each account is refreshed in its own async task. Official APIs refresh every 15 minutes; local/OAuth/browser sources use 30 minutes. Failure schedules exponential backoff up to 15 minutes, retains stored metrics and is surfaced as `Stale` by the UI.

## Credentials

`keyring` uses the Windows native credential store. The SQLite `credential_ref` is a generated opaque identifier. Provider HTTP errors never include the submitted credential, and the log filter rejects credential/cookie-targeted records.

## Data semantics

- OpenAI requires an Admin Key and uses organization cost/completions usage endpoints.
- DeepSeek saves total, topped-up and granted balances as independent snapshots; it never derives spend from a balance delta.
- OpenRouter persists whether the connection is a normal key or management credential so scheduled refreshes preserve key-level versus account-level scope.
- Provider history buckets and balance snapshots are separate tables.

## Desktop behavior

The main window defaults to 1440×1024 with 960×700 minimum dimensions. Closing hides it to the tray. The tray can reopen or fully quit the app. Start-on-login is enabled on first launch and can be changed from Settings.
