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

## Completed milestone: product-grade authentication

1. Authentication mode, credential owner, identity label/fingerprint, consent time and validation time are persisted without changing metric/history semantics.
2. Codex detects and displays the current email/plan/Codex-only scope and requires explicit `Use this account` confirmation.
3. Codex App Server `account/login/start` supports system-browser completion notifications, cancellation, timeout and account switching.
4. Codex device-code fallback is available; dashboard removal never invokes `account/logout`.
5. OpenRouter prefers localhost PKCE S256 with random port/path and single-use callback; manual normal keys and advanced Management keys remain separate.
6. API-key providers have provider-specific setup/scope guidance, test-before-save and rollback-safe credential replacement.
7. Gemini CLI distinguishes missing install, signed out, invalid credential and signed-in states, requires identity confirmation and remains Experimental.
8. Authentication lifecycle/security tests cover identity confirmation UI, completion correlation, failed completion, cancellation/late-completion races, timeout state, PKCE callback path/single use, schema migration, cached authentication failures and error classification.

The detailed implementation contract is in [Authentication and Connection Design](AUTHENTICATION.md).

## Implemented frontend interaction direction

The next frontend pass is defined in [Frontend Interaction Design](INTERACTION_DESIGN.md):

- Dashboard becomes the only primary page.
- Connections and Settings move to compact header icon controls and lightweight overlays.
- Dashboard exposes Add account directly and existing account cards/rows open account detail directly.
- Add account progresses through account type, visible provider selection and provider-specific connection; it no longer starts with one all-provider dropdown.
- Account list, detail, add, edit, credential replacement and removal share one Connections dialog shell instead of stacking pages and modals.

These interaction decisions are implemented in the current frontend working tree. Automated tests cover the single-page shell, overlays, direct detail/edit entry, progressive category/provider selection, explicit Codex confirmation and OpenRouter PKCE preference. Provider-backed authentication remains subject to the external acceptance below.

## External acceptance still required

- Reconcile each rendered metric value and time window against the user's official provider dashboard; live authentication/display is confirmed.
- Validate invalid credential, permission, rate-limit and network failures with real accounts.
- Run a seven-day private beta and retain 30 days of continuous history for long-term acceptance.
- Install and sign in to Gemini CLI, then reconcile its live per-model quota against `/stats model`.
- Reconcile Google AI Studio API usage manually because the public API-key endpoint does not expose AI Studio spend or prepaid balance.
- Complete a restart-while-offline manual check using the connected accounts to confirm cached values remain visible as Stale.
- Validate Codex browser and device-code login against real personal and managed-workspace accounts.
- Validate that removing a Codex or Gemini dashboard connection does not sign out the provider's installed client.
- Validate OpenRouter PKCE callback, cancellation and secure Windows Credential Manager storage against a real OpenRouter account; replay rejection is automated.

## Release checklist

- `npm test`, `npm run test:sites`, `npm run build`.
- `cargo test`, `cargo fmt -- --check`, strict clippy.
- Tauri production build and Windows installer smoke test.
- Confirm database/logs do not contain complete credentials.
- Confirm browser callback URLs, authorization codes, PKCE verifiers and OAuth tokens do not appear in logs or crash reports.
- Keyboard navigation, focus rings, light/dark contrast and high-DPI check.
- Compare implementation and reference at normalized 1440×1024; record result in `design-qa.md`.
