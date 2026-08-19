# AI Usage Dashboard

Local-first Windows desktop dashboard for monitoring personal AI API accounts and subscription quotas without proxying model traffic.

## Current MVP

- Windows 11 desktop shell: Tauri 2 + React/TypeScript + Rust.
- Dashboard, Accounts and Settings only.
- Dark/light themes and list/card provider layouts, persisted locally.
- Official API adapters for OpenAI organization usage, DeepSeek balance, OpenRouter key/account usage and Google AI Studio API-key validation.
- Local subscription adapters for ChatGPT/Codex app-server quota and Gemini CLI OAuth model quota.
- SQLite metrics/history with balance snapshots separated from provider history buckets.
- Secrets stored in Windows Credential Manager; SQLite stores only credential references.
- Per-account scheduling, provider timeouts, exponential backoff, tray residency and start-on-login.
- Cached last-success recovery, provider/local history trends and stale diagnostics across restarts.
- Dynamic multi-currency balance, month-to-date spend and reporting-coverage summaries.
- Persisted account enablement, credential replacement and refresh/history/Windows behavior settings.
- Google AI Studio API and Gemini CLI subscription accounts are stored and displayed separately.
- Codex and Gemini subscription scopes are explicit and never described as full ChatGPT or Google AI entitlement usage.
- Provider authentication is intentionally split between pasted secrets, provider-managed browser authorization and confirmed local CLI sessions; there is no universal login flow.

## Run

```powershell
npm install
npm run dev
```

Desktop development:

```powershell
npm run tauri -- dev
```

Verification:

```powershell
npm test
npm run test:sites
npm run build
cd src-tauri
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Security boundary

The application is read-only toward providers. It does not proxy API traffic, manage provider keys, rotate credentials, recharge accounts or log prompt/request content. Codex is queried through its local read-only app-server. Gemini CLI OAuth is read only after the user explicitly adds that account; raw OAuth credentials never enter SQLite or logs.

See [Authentication and Connection Design](docs/AUTHENTICATION.md), [Capability Matrix](docs/CAPABILITY_MATRIX.md), [Architecture](docs/ARCHITECTURE.md), and [Development Plan](docs/DEVELOPMENT.md).
