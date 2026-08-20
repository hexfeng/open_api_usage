# Prototype Instructions

## Durable product authentication decisions

- Do not model every provider connection as a generic login. Distinguish pasted API/Admin/Management keys, provider-managed browser OAuth, confirmed shared local sessions and local CLI OAuth reuse.
- A detected Codex or Gemini local session must be shown to the user and explicitly confirmed before connection in the product flow, even when no new browser login is technically required.
- Codex product login should use the official App Server browser flow with device-code fallback; do not construct OpenAI OAuth URLs or read Codex token files.
- OpenRouter should prefer official PKCE for normal user-controlled keys while keeping Management keys as a separate advanced path.
- Removing a dashboard account must not automatically revoke a provider key or sign out a shared Codex/Gemini client.
- Keep Gemini CLI quota integration Experimental until its local/internal contract is proven stable enough for release.
- Keep Dashboard as the only primary product page. Accounts and Settings are lightweight, low-frequency management surfaces and should be opened from compact header icon controls as overlays rather than treated as peer top-level navigation pages.
- Dashboard must expose a direct Add account action, and clicking an account card or row must open that account's detail/manage state without routing through a separate Accounts page.
- Add account is a progressive overlay flow: choose API Platform or Subscription, choose a provider from a visible provider list, then complete the provider-specific connection. Do not use one all-provider dropdown as the primary selection interaction.

Run the local server yourself and open the preview in the browser available to this environment. Do not give the user server-start instructions when you can run it.

Before making substantial visual changes, use the Product Design plugin's `get-context` skill when the visual source is unclear or no longer matches the current goal. When the user gives durable prototype-specific design feedback, preferences, or decisions, record them in `AGENTS.md`.

When implementing from a selected generated mock, treat that image as the source of truth for layout, component anatomy, density, spacing, color, typography, visible content, and hierarchy.

Build app UI in `src/`. Keep `.openai/hosting.json`, `worker/index.js`, `scripts/prepare-sites-build.mjs`, and `tests/sites-worker.test.mjs` intact so the same local prototype can be handed to Sites. Before a Sites handoff, run `npm run build` and `npm run test:sites`; the build must leave `dist/client/index.html`, `dist/server/index.js`, and `dist/.openai/hosting.json`.
