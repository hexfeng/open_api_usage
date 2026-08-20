# Frontend Interaction Design

Status: implemented in the current frontend working tree. Updated 2026-08-20.

This document defines the next frontend interaction structure for AI Usage Dashboard. It intentionally replaces the current three-peer-page navigation model while preserving the existing visual language, provider authentication boundaries and local-first behavior.

## 1. Product interaction goal

Dashboard is the product. A user should be able to inspect current usage, add an account, open an account, recover a failed connection and edit account details without leaving the dashboard context.

The interface should optimize for these frequent actions:

1. Read balance, spend, remaining quota, freshness and account health.
2. Add either an API Platform or Subscription account directly from Dashboard.
3. Click an existing account to inspect or manage it.
4. Refresh or recover an account without navigating through a separate management page.

Accounts and Settings are lightweight management surfaces, not peer product destinations.

## 2. Information architecture

```text
Dashboard
├─ Add account -> Connections dialog / Add flow
├─ Account card or row -> Connections dialog / Account detail
├─ Connections icon -> Connections dialog / Account list
├─ Settings icon -> Settings dialog
└─ Theme control -> Immediate light/dark switch
```

The persistent header contains:

- Brand at the left.
- Compact Connections and Settings icon buttons at the right.
- The existing light/dark segmented control beside those icons.

Dashboard is the only primary page. Remove Dashboard, Accounts and Settings as peer navigation tabs. Connections and Settings icons need visible tooltips, accessible names and the same compact visual treatment as the theme control.

## 3. Dashboard actions and account interaction

### 3.1 Direct Add account entry

Dashboard must always expose `Add account` in the page-heading action area beside `Refresh all`.

- `Add account` opens the Connections dialog directly in the first Add flow step.
- `Refresh all` remains a separate secondary action.
- When no accounts exist, the empty state repeats `Add account` as its primary action.
- Adding another account must never require opening the Connections list first.

### 3.2 Existing account interaction

Clicking a provider card, provider row or subscription card opens the same Connections dialog directly in that account's detail state.

- Card or row body: open account detail.
- `Open provider`: open the external provider page without opening account detail.
- Recovery action on Stale or Authentication required: open the same account detail focused on recovery.
- Edit is entered from account detail; Dashboard does not route through a separate Accounts page.

Cards and rows must be keyboard-focusable interactive elements rather than click-only articles.
The detail target and any secondary action must remain sibling interactive elements; do not wrap `Open provider` or recovery buttons inside one card-wide button.

## 4. One Connections dialog

All account-related tasks share one overlay shell instead of opening dialogs on top of dialogs.

```text
ConnectionsDialogState
├─ list
├─ detail(accountId)
├─ add.category
├─ add.provider(category)
├─ add.connect(providerId)
├─ edit.details(accountId)
├─ edit.credential(accountId)
└─ remove.confirm(accountId)
```

Entry behavior:

| Entry | Initial dialog state |
|---|---|
| Header Connections icon | Account list |
| Dashboard Add account | Add / Choose account type |
| Dashboard account card or row | Account detail |
| Account recovery action | Account detail / Recovery |

The dialog uses a Back control for internal navigation and one Close control for leaving the overlay. It must not stack an Add, Edit or confirmation modal above the Connections dialog.

Recommended desktop geometry:

- Width appropriate for visible account/provider lists, approximately 720–840 px.
- `max-height: calc(100vh - 48px)` or equivalent.
- Scrollable body with stable header and action area.
- Single-column reflow at the 960×700 minimum window and Windows high-DPI scaling.

## 5. Add account flow

The primary Add flow must not start with one provider dropdown. Provider selection is progressive and visible.

```text
Step 1: Choose account type
        API Platform | Subscription
                    ↓
Step 2: Choose provider
        Visible provider cards filtered by account type
                    ↓
Step 3: Connect and confirm
        Provider-specific authentication, test and consent
```

The dialog header shows the current step and offers Back and Close. If the user has entered a credential or started authorization, Back or changing provider requires an explicit discard/cancel decision. Secrets and authorization state are never carried into another provider flow.

### 5.1 Step 1 — Choose account type

Show two large selection cards:

#### API Platform

- Description: API spend, account balance, key usage or project access.
- Expected connection: API/Admin/Management key or provider browser authorization.
- Next: API provider selection.

#### Subscription

- Description: plan quota collected through a confirmed Codex or Gemini CLI session.
- Expected connection: detected local session or provider-owned login.
- Next: Subscription provider selection.

The categories are product concepts, not generic authentication modes.

### 5.2 Step 2 — Choose provider

Show providers as a visible list or responsive card grid. Do not use a single select control as the primary provider picker.

API Platform providers:

- OpenAI API
- DeepSeek API
- OpenRouter
- Google AI Studio API

Subscription providers:

- ChatGPT / Codex
- Google / Gemini CLI · Experimental

Each provider option shows:

- Provider icon and name.
- What data the connection can expose.
- Primary connection method.
- Experimental status when applicable.

The provider area scrolls vertically when it grows beyond the available dialog height. New providers can be added as visible items without making a long, hard-to-scan dropdown. Search or filtering is not required for the current provider count.

### 5.3 Step 3 — Connect and confirm

Only fields required for the connection appear here. Account name can default from the provider and remain editable. Optional plan price and renewal metadata do not block connection and should be added later from account details.

#### Pasted-key API providers

1. Explain the exact credential type, permission and metric scope.
2. Accept the credential.
3. `Test connection`.
4. Display what was validated.
5. Enable `Save account` only after a successful test.

#### OpenRouter

After choosing OpenRouter, show three visible connection-method choices:

1. Connect with OpenRouter / PKCE — recommended.
2. Normal API Key — key-level usage.
3. Management Key — advanced account-credit scope.

Management Key remains visually separated as an advanced path. It is not a silent scope change inside the ordinary edit form.

#### ChatGPT / Codex

1. Detect the current local Codex session.
2. Show detected identity, plan and `Codex only` scope.
3. Label the system state `Account detected`; do not claim user confirmation automatically.
4. Offer `Connect this account` as the explicit consent action.
5. Offer `Use another account`; prefer official browser login with device code as fallback.

#### Google / Gemini CLI

Show missing installation, signed out, invalid credential and signed-in states separately. A signed-in identity must be displayed and explicitly connected. `Gemini CLI only` and `Experimental` remain visible throughout the flow.

### 5.4 Completion

Successful connection transitions to the new account's detail state in the same dialog. The user can close the dialog or optionally add plan name, monthly price and renewal metadata there.

## 6. Account detail and editing

Account detail is the canonical account surface, whether opened from Dashboard or the Connections list.

It contains:

- Account identity, provider and data scope.
- Health and freshness.
- Experimental maturity as a separate label.
- Current metrics and last successful update.
- Failure explanation and recovery action when relevant.
- Connection method, credential owner and credential hint.
- `Edit details`, `Replace credential` or `Reconnect`, and `Remove from dashboard`.

Editing is split by intent:

- `Edit details`: account name and optional manual subscription metadata.
- `Replace credential`: validate the new credential before replacing the old local credential.
- `Reconnect`: provider browser login or confirmed local-session flow.
- Changing OpenRouter between normal and Management scope uses a connection-method change flow, not a simple metadata dropdown.

`Remove from dashboard` changes the dialog body to a confirmation state naming the account and explaining the exact effect. Confirmation removes the local dashboard connection and its locally stored credential, then returns to Dashboard; it does not revoke a provider key or sign out a shared Codex or Gemini CLI session.

## 7. Settings dialog

Settings opens from a compact header Gear icon and remains separate from Connections.

It contains the existing lightweight groups:

- Refresh schedule.
- Windows behavior.
- History retention.
- Local data.

Settings save immediately with local success or error feedback. Delete local data changes the same dialog body to a clear confirmation state; it does not open another nested modal.

## 8. Overlay interaction rules

- Only one overlay is open at a time.
- Escape closes the overlay unless an in-progress provider authorization requires an explicit cancel path.
- Closing a dirty edit state asks whether to discard changes.
- Focus moves into the overlay when it opens and returns to the triggering icon, button, card or row when it closes.
- Back changes internal dialog state without losing the Dashboard beneath it.
- External provider links are the only account actions that navigate away from the app surface.
- No route change is required for Connections or Settings.

## 9. Acceptance criteria for the approved direction

- Dashboard is the only peer-level page.
- Connections and Settings are compact header icon controls beside the theme control.
- Dashboard exposes Add account without first opening the Connections list.
- Dashboard cards and rows open account detail directly.
- Add account starts with API Platform versus Subscription.
- Provider selection uses visible provider items, not one all-provider dropdown.
- The selected category determines the provider choices shown in the next step.
- Provider-specific connection, test and consent behavior remains intact.
- Add, detail, edit, credential replacement and removal use one Connections dialog shell without nested dialogs.
- Back, provider changes and closing never discard entered credentials or in-progress authorization without an explicit decision.
- Removing an account clearly distinguishes local dashboard removal from provider revocation or shared-session logout.
- The complete flow remains reachable at 960×700 and Windows high-DPI scaling.
- Current exclusions remain unchanged: no search, filtering, notifications, alert rules, Gateway, cloud sync or independent History page.

## 10. Current implementation boundary

The current frontend implements Dashboard as the single primary page, compact Connections and Settings controls, direct account detail, and the progressive Add account flow defined here. Automated frontend tests and browser QA cover these interaction states. Real Codex, Gemini CLI, OpenRouter and API-key authentication still require the provider-specific manual acceptance described in the authentication and development documents.
