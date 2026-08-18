# Development and Acceptance Plan

## Completed engineering slices

1. Confirmed visual baseline copied into `design/`.
2. React design tokens and four theme/layout combinations implemented.
3. Dashboard, Accounts, Settings, account detail and add/test connection flows implemented.
4. SQLite schema, Windows credential references and official API adapters implemented.
5. Independent scheduler, exponential backoff, tray and autostart implemented.
6. Unit, component, static-host and browser interaction checks implemented.
7. Dynamic summaries, cached last-success recovery and real seven-day history reads implemented.
8. Account edit/persistent disable and SQLite-backed refresh, tray and retention settings implemented.
9. ChatGPT/Codex app-server subscription quota implemented and live-validated on Windows.
10. Google split into Google AI Studio API and Gemini CLI subscription accounts, with separate credentials, scope and diagnostics.

## External acceptance still required

- Reconcile each rendered metric value and time window against the user's official provider dashboard; live authentication/display is confirmed.
- Validate invalid credential, permission, rate-limit and network failures with real accounts.
- Run a seven-day private beta and retain 30 days of continuous history for long-term acceptance.
- Install and sign in to Gemini CLI, then reconcile its live per-model quota against `/stats model`.
- Reconcile Google AI Studio API usage manually because the public API-key endpoint does not expose AI Studio spend or prepaid balance.
- Complete a restart-while-offline manual check using the connected accounts to confirm cached values remain visible as Stale.

## Release checklist

- `npm test`, `npm run test:sites`, `npm run build`.
- `cargo test`, `cargo fmt -- --check`, strict clippy.
- Tauri production build and Windows installer smoke test.
- Confirm database/logs do not contain complete credentials.
- Keyboard navigation, focus rings, light/dark contrast and high-DPI check.
- Compare implementation and reference at normalized 1440×1024; record result in `design-qa.md`.
