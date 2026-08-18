# Development and Acceptance Plan

## Completed engineering slices

1. Confirmed visual baseline copied into `design/`.
2. React design tokens and four theme/layout combinations implemented.
3. Dashboard, Accounts, Settings, account detail and add/test connection flows implemented.
4. SQLite schema, Windows credential references and official API adapters implemented.
5. Independent scheduler, exponential backoff, tray and autostart implemented.
6. Unit, component, static-host and browser interaction checks implemented.

## External acceptance still required

- Compare each official adapter against the user's real provider dashboard.
- Validate invalid credential, permission, rate-limit and network failures with real accounts.
- Run a seven-day private beta and retain 30 days of continuous history for long-term acceptance.
- Approve a Windows-safe Codex quota connector.
- Approve explicit browser-session consent and domain allowlist behavior before enabling Gemini experimental reads.

## Release checklist

- `npm test`, `npm run test:sites`, `npm run build`.
- `cargo test`, `cargo fmt -- --check`, strict clippy.
- Tauri production build and Windows installer smoke test.
- Confirm database/logs do not contain complete credentials.
- Keyboard navigation, focus rings, light/dark contrast and high-DPI check.
- Compare implementation and reference at normalized 1440×1024; record result in `design-qa.md`.
