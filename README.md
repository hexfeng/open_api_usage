# AI Usage Dashboard

Local-first Windows desktop dashboard for monitoring personal AI API accounts and subscription quotas without proxying model traffic.

## Current MVP

- Windows 11 desktop shell: Tauri 2 + React/TypeScript + Rust.
- Dashboard, Accounts and Settings only.
- Dark/light themes and list/card provider layouts, persisted locally.
- Official API adapters for OpenAI organization usage, DeepSeek balance and OpenRouter key/account usage.
- SQLite metrics/history with balance snapshots separated from provider history buckets.
- Secrets stored in Windows Credential Manager; SQLite stores only credential references.
- Per-account scheduling, provider timeouts, exponential backoff, tray residency and start-on-login.
- Codex and Gemini subscription scopes are represented explicitly and never described as full ChatGPT or Google AI entitlement usage.

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

The application is read-only toward providers. It does not proxy API traffic, manage provider keys, rotate credentials, recharge accounts or log prompt/request content. Browser-session connectors require a separate explicit-consent implementation before they may read a domain-scoped session; raw cookies must never enter SQLite or logs.

See [Capability Matrix](docs/CAPABILITY_MATRIX.md), [Architecture](docs/ARCHITECTURE.md), and [Development Plan](docs/DEVELOPMENT.md).
