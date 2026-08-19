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

## Next milestone: product-grade authentication

1. Add an authentication-mode and credential-owner model without changing metric semantics.
2. Codex: detect the current account, display email/plan/scope and require explicit `Use this account` confirmation.
3. Codex: implement App Server `account/login/start` browser flow, completion notifications, cancellation and timeout.
4. Codex: add device-code fallback and keep `Remove from dashboard` separate from `account/logout`.
5. OpenRouter: implement localhost PKCE S256 for normal user-controlled keys; keep Management key as an advanced manual option.
6. API-key providers: add provider-specific setup guidance, permission descriptions and atomic test-before-replace behavior.
7. Gemini CLI: add install/login-state diagnostics and explicit confirmation while retaining the Experimental label.
8. Add authentication lifecycle tests for existing session, new login, cancellation, timeout, revoked credentials, account switch and local client removal.

The detailed contract is in [Authentication and Connection Design](AUTHENTICATION.md). These items are documented targets and are not implemented by this documentation update.

## External acceptance still required

- Reconcile each rendered metric value and time window against the user's official provider dashboard; live authentication/display is confirmed.
- Validate invalid credential, permission, rate-limit and network failures with real accounts.
- Run a seven-day private beta and retain 30 days of continuous history for long-term acceptance.
- Install and sign in to Gemini CLI, then reconcile its live per-model quota against `/stats model`.
- Reconcile Google AI Studio API usage manually because the public API-key endpoint does not expose AI Studio spend or prepaid balance.
- Complete a restart-while-offline manual check using the connected accounts to confirm cached values remain visible as Stale.
- Validate Codex browser and device-code login against personal and managed-workspace accounts once implemented.
- Validate that removing a Codex or Gemini dashboard connection does not sign out the provider's installed client.
- Validate OpenRouter PKCE callback, cancellation, replay rejection and secure local key storage once implemented.

## Release checklist

- `npm test`, `npm run test:sites`, `npm run build`.
- `cargo test`, `cargo fmt -- --check`, strict clippy.
- Tauri production build and Windows installer smoke test.
- Confirm database/logs do not contain complete credentials.
- Confirm browser callback URLs, authorization codes, PKCE verifiers and OAuth tokens do not appear in logs or crash reports.
- Keyboard navigation, focus rings, light/dark contrast and high-DPI check.
- Compare implementation and reference at normalized 1440×1024; record result in `design-qa.md`.
