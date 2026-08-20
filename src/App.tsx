import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import {
  ArrowLeft,
  ArrowSquareOut,
  Atom,
  CardsThree,
  CaretRight,
  Check,
  ClockCounterClockwise,
  Database,
  GearSix,
  Key,
  ListBullets,
  Moon,
  PencilSimple,
  PlugsConnected,
  Plus,
  ArrowClockwise,
  ShieldCheck,
  SidebarSimple,
  SlidersHorizontal,
  Sun,
  Trash,
  X,
} from "@phosphor-icons/react";
import { siDeepseek, siGoogle, siGooglegemini, siOpenrouter } from "simple-icons";
import { defaultSettings, demoProviders, demoSubscriptions } from "./demoData";
import {
  isTauri,
  deleteAllLocalData,
  cancelAuthentication,
  detectAuthentication,
  listenForAccountUpdates,
  loadAppSettings,
  loadConnectedAccounts,
  openOfficialUrl,
  pollAuthentication,
  refreshConnectedAccount,
  removeConnectedAccount,
  saveAppSettings,
  saveConnectedAccount,
  saveOpenRouterOauthAccount,
  setConnectedAccountEnabled,
  testConnection,
  startCodexLogin,
  startOpenRouterLogin,
  updateConnectedAccount,
  type AccountRequest,
  type AuthenticationAttempt,
  type AuthenticationDetection,
  type UpdateAccountRequest,
} from "./bridge";
import type {
  AppSettings,
  LayoutMode,
  Metric,
  ProviderAccount,
  ProviderId,
  Status,
  SubscriptionAccount,
  Theme,
} from "./types";

type ConnectionsEntry =
  | { kind: "list" }
  | { kind: "detail"; accountId: string }
  | { kind: "add" };

type ConnectionsView =
  | ConnectionsEntry
  | { kind: "edit-details"; accountId: string }
  | { kind: "replace-credential"; accountId: string }
  | { kind: "remove-confirm"; accountId: string };

type AccountCategory = ProviderAccount["type"];

const providerOptions: { id: ProviderId; category: AccountCategory; label: string; description: string; authentication: string; experimental?: boolean }[] = [
  { id: "openai", category: "API Platform", label: "OpenAI API", description: "Organization Costs and Usage", authentication: "Organization Admin Key" },
  { id: "deepseek", category: "API Platform", label: "DeepSeek API", description: "Account balance and balance sources", authentication: "API Key" },
  { id: "openrouter", category: "API Platform", label: "OpenRouter", description: "Key usage or account credits", authentication: "PKCE, API Key or Management Key" },
  { id: "google-ai-studio", category: "API Platform", label: "Google AI Studio API", description: "Project API access validation", authentication: "Gemini API Key" },
  { id: "chatgpt-codex", category: "Subscription", label: "ChatGPT / Codex", description: "Codex quota windows and credits", authentication: "Confirmed Codex session" },
  { id: "google-gemini-cli", category: "Subscription", label: "Google / Gemini CLI", description: "Per-model Gemini CLI quota", authentication: "Confirmed Gemini CLI session", experimental: true },
];

function storedValue<T extends string>(key: string, fallback: T): T {
  return (localStorage.getItem(key) as T | null) ?? fallback;
}

export function App() {
  const [theme, setTheme] = useState<Theme>(() => storedValue("aud-theme", "dark"));
  const [layout, setLayout] = useState<LayoutMode>(() => storedValue("aud-layout", "list"));
  const [accounts, setAccounts] = useState<ProviderAccount[]>(() => isTauri ? [] : [...demoProviders, ...demoSubscriptions]);
  const [settings, setSettings] = useState(defaultSettings);
  const [settingsError, setSettingsError] = useState("");
  const [connectionsEntry, setConnectionsEntry] = useState<ConnectionsEntry | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [isRefreshing, setRefreshing] = useState(false);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("aud-theme", theme);
  }, [theme]);

  useEffect(() => localStorage.setItem("aud-layout", layout), [layout]);

  useEffect(() => {
    if (!isTauri) return;
    let active = true;
    let stopListening: (() => void) | undefined;
    const replaceAccount = (updated: ProviderAccount) => {
      if (!active) return;
      setAccounts((items) => items.map((item) => item.id === updated.id ? updated : item));
    };
    void listenForAccountUpdates(replaceAccount).then((stop) => {
      if (active) stopListening = stop;
      else stop();
    });
    void loadConnectedAccounts()
      .then((cached) => {
        if (!active) return;
        setAccounts(cached);
        return Promise.all(cached.filter((account) => account.enabled).map(refreshConnectedAccount));
      })
      .then((fresh) => {
        if (active && fresh) setAccounts((items) => items.map((item) => fresh.find((next) => next.id === item.id) ?? item));
      })
      .catch(() => undefined);
    void loadAppSettings().then((value) => { if (active) setSettings(value); }).catch((reason) => {
      if (active) setSettingsError(String(reason));
    });
    return () => {
      active = false;
      stopListening?.();
    };
  }, []);

  const providers = accounts.filter((account) => account.type === "API Platform");
  const subscriptions = accounts.filter((account) => account.type === "Subscription");

  const replaceAccount = (updated: ProviderAccount) => {
    setAccounts((items) => items.map((item) => item.id === updated.id ? updated : item));
  };

  const toggleAccount = (id: string, enabled: boolean) => {
    if (!isTauri) {
      setAccounts((items) => items.map((item) => item.id === id ? { ...item, enabled } : item));
      return;
    }
    void setConnectedAccountEnabled(id, enabled).then(replaceAccount);
  };

  const removeAccount = async (id: string) => {
    await removeConnectedAccount(id);
    setAccounts((items) => items.filter((item) => item.id !== id));
  };

  const refreshAll = async () => {
    if (isRefreshing) return;
    setRefreshing(true);
    setAccounts((items) => items.map((item) => item.enabled ? { ...item, status: "Refreshing" } : item));
    if (isTauri) {
      const refreshed = await Promise.all(accounts.filter((account) => account.enabled).map(refreshConnectedAccount));
      setAccounts((items) => items.map((item) => refreshed.find((next) => next.id === item.id) ?? item));
      setRefreshing(false);
      return;
    }
    window.setTimeout(() => {
      setAccounts((items) =>
        items.map((item, index) => ({
          ...item,
          status: index === 2 ? "Stale" : "Live",
          updatedLabel: index === 2 ? "34 min ago" : "Just now",
        })),
      );
      setRefreshing(false);
    }, 900);
  };

  return (
    <div className="app-shell">
      <AppHeader
        theme={theme}
        onThemeChange={setTheme}
        onConnections={() => setConnectionsEntry({ kind: "list" })}
        onSettings={() => setSettingsOpen(true)}
      />
      <main className="app-main">
        <Dashboard
          layout={layout}
          providers={providers}
          subscriptions={subscriptions}
          refreshing={isRefreshing}
          onLayoutChange={setLayout}
          onRefresh={() => void refreshAll()}
          onAdd={() => setConnectionsEntry({ kind: "add" })}
          onSelect={(account) => setConnectionsEntry({ kind: "detail", accountId: account.id })}
        />
      </main>
      {connectionsEntry ? (
        <ConnectionsDialog
          entry={connectionsEntry}
          accounts={accounts}
          onClose={() => setConnectionsEntry(null)}
          onAdded={(account) => setAccounts((items) => [...items, account])}
          onUpdated={replaceAccount}
          onToggle={toggleAccount}
          onRemove={removeAccount}
        />
      ) : null}
      {settingsOpen ? (
        <SettingsDialog
          settings={settings}
          error={settingsError}
          onClose={() => setSettingsOpen(false)}
          onChange={(next) => {
            const previous = settings;
            setSettings(next);
            setSettingsError("");
            if (isTauri) void saveAppSettings(next).then(setSettings).catch((reason) => {
              setSettings(previous);
              setSettingsError(String(reason));
            });
          }}
          onDeleteData={async () => { await deleteAllLocalData(); setAccounts([]); }}
        />
      ) : null}
    </div>
  );
}

function AppHeader({
  theme,
  onThemeChange,
  onConnections,
  onSettings,
}: {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  onConnections: () => void;
  onSettings: () => void;
}) {
  return (
    <header className="topbar">
      <div className="brand" aria-label="AI Usage Dashboard">
        <span className="brand-mark"><SidebarSimple weight="fill" /></span>
        <span>AI Usage</span>
      </div>
      <div className="header-utilities">
        <button className="utility-button" onClick={onConnections} aria-label="Open connections" title="Connections"><PlugsConnected /></button>
        <button className="utility-button" onClick={onSettings} aria-label="Open settings" title="Settings"><GearSix /></button>
        <div className="theme-switch" aria-label="Color theme">
          <button
            className={theme === "light" ? "active" : ""}
            onClick={() => onThemeChange("light")}
            aria-pressed={theme === "light"}
          >
            <Sun /> <span>Light</span>
          </button>
          <button
            className={theme === "dark" ? "active" : ""}
            onClick={() => onThemeChange("dark")}
            aria-pressed={theme === "dark"}
          >
            <Moon /> <span>Dark</span>
          </button>
        </div>
      </div>
    </header>
  );
}

function Dashboard({
  layout,
  providers,
  subscriptions,
  refreshing,
  onLayoutChange,
  onRefresh,
  onAdd,
  onSelect,
}: {
  layout: LayoutMode;
  providers: ProviderAccount[];
  subscriptions: SubscriptionAccount[];
  refreshing: boolean;
  onLayoutChange: (layout: LayoutMode) => void;
  onRefresh: () => void;
  onAdd: () => void;
  onSelect: (account: ProviderAccount) => void;
}) {
  const balanceByCurrency = new Map<string, number>();
  let balanceCoverage = 0;
  let spend = 0;
  let spendCoverage = 0;
  let reporting = 0;
  for (const provider of providers) {
    if (provider.metrics.length) reporting += 1;
    const balance = provider.metrics.find((metric) => metric.metricKind === "total_balance" || metric.metricKind === "remaining_credits");
    if (balance?.currency) {
      balanceByCurrency.set(balance.currency, (balanceByCurrency.get(balance.currency) ?? 0) + balance.numericValue);
      balanceCoverage += 1;
    }
    const monthlySpend = provider.metrics.find((metric) => metric.metricKind === "cost");
    if (monthlySpend?.currency === "USD") {
      spend += monthlySpend.numericValue;
      spendCoverage += 1;
    }
  }
  const balances = ["USD", "CNY"]
    .filter((currency) => balanceByCurrency.has(currency))
    .map((currency) => `${currency === "CNY" ? "¥" : "$"}${balanceByCurrency.get(currency)!.toFixed(2)}`)
    .join(" · ") || "—";
  const live = providers.filter((provider) => provider.status === "Live").length;
  const stale = providers.filter((provider) => provider.status === "Stale").length;
  const latest = providers
    .flatMap((provider) => provider.metrics.map((metric) => metric.observedAt))
    .sort((a, b) => new Date(b).getTime() - new Date(a).getTime())[0];
  const updated = latest
    ? new Intl.DateTimeFormat("en", { month: "short", day: "numeric", year: "numeric", hour: "numeric", minute: "2-digit" }).format(new Date(latest))
    : "No successful refresh yet";
  return (
    <>
      <section className="page-heading">
        <div>
          <h1>AI Usage Dashboard</h1>
          <p>Last updated {updated}</p>
        </div>
        <div className="heading-actions">
          <button className="button primary" onClick={onAdd}><Plus />Add account</button>
          <button className="button secondary" onClick={onRefresh} disabled={refreshing}>
            <ArrowClockwise className={refreshing ? "spin" : ""} />
            {refreshing ? "Refreshing" : "Refresh all"}
          </button>
        </div>
      </section>

      <div className="dashboard-grid">
        <div className="dashboard-main-column">
          <section className="summary-grid" aria-label="Account summary">
            <SummaryBlock label="API balance" value={balances} meta={`${balanceCoverage} of ${providers.length} accounts reporting balance`} accent="blue" />
            <SummaryBlock label="USD spend · MTD" value={spendCoverage ? `$${spend.toFixed(2)}` : "—"} meta={`${spendCoverage} of ${providers.length} accounts reporting monthly spend`} accent="violet" />
            <SummaryBlock label="Data coverage" value={`${reporting}/${providers.length || 0}`} meta={`${live} live · ${stale} stale`} accent="green" />
          </section>
          <section className="content-section api-section">
            <div className="section-heading">
              <div><h2>API Platforms</h2><p>Provider-reported account metrics</p></div>
              <div className="view-switch" aria-label="API platform layout">
                <button className={layout === "list" ? "active" : ""} onClick={() => onLayoutChange("list")} aria-label="List layout" aria-pressed={layout === "list"}><ListBullets /> <span>List</span></button>
                <button className={layout === "cards" ? "active" : ""} onClick={() => onLayoutChange("cards")} aria-label="Card layout" aria-pressed={layout === "cards"}><CardsThree /> <span>Cards</span></button>
              </div>
            </div>
            {providers.length ? (
              <div className={layout === "cards" ? "providers providers-cards" : "providers providers-list"}>
                {providers.map((provider) => layout === "cards" ? <ProviderCard key={provider.id} account={provider} onSelect={onSelect} /> : <ProviderRow key={provider.id} account={provider} onSelect={onSelect} />)}
              </div>
            ) : <EmptyState onAdd={onAdd} />}
          </section>
        </div>

        <aside className="content-section subscriptions-section">
          <div className="section-heading">
            <div>
              <h2>Subscriptions</h2>
              <p>Plan-specific usage windows</p>
            </div>
          </div>
          <div className="subscription-stack">
            {subscriptions.map((subscription) => (
              <SubscriptionCard key={subscription.id} account={subscription} onSelect={onSelect} />
            ))}
          </div>
          <p className="scope-footnote"><ShieldCheck /> Automatic quota data covers the named CLI product only.</p>
        </aside>
      </div>
    </>
  );
}

function SummaryBlock({ label, value, meta, accent }: { label: string; value: string; meta: string; accent: string }) {
  return (
    <article className={`summary-block ${accent}`}>
      <div className="summary-label"><span className="summary-dot" />{label}</div>
      <strong>{value}</strong>
      <span>{meta}</span>
    </article>
  );
}

function ProviderRow({ account, onSelect }: { account: ProviderAccount; onSelect: (account: ProviderAccount) => void }) {
  return (
    <article className="provider-row">
      <button className="provider-row-detail" onClick={() => onSelect(account)} aria-label={`Open ${account.name} details`}>
        <div className="provider-identity">
          <ProviderLogo provider={account.provider} />
          <div>
            <h3>{account.name}</h3>
            <p><SourceBadge source={account.source} /> · {account.scope}</p>
          </div>
        </div>
        <div className="row-metrics">
          {account.metrics.map((metric) => <MetricValue key={metric.id} metric={metric} />)}
        </div>
        <div className="row-status">
          <StatusBadge status={account.status} />
          <span>{account.updatedLabel}</span>
        </div>
      </button>
      <OfficialLink href={account.officialUrl} />
    </article>
  );
}

function ProviderCard({ account, onSelect }: { account: ProviderAccount; onSelect: (account: ProviderAccount) => void }) {
  return (
    <article className="provider-card">
      <button className="provider-card-detail" onClick={() => onSelect(account)} aria-label={`Open ${account.name} details`}>
        <div className="provider-card-head">
          <ProviderLogo provider={account.provider} />
          <StatusBadge status={account.status} />
        </div>
        <div className="provider-card-title">
          <h3>{account.name}</h3>
          <p>{account.scope}</p>
        </div>
        <div className="card-metrics">
          {account.metrics.map((metric) => <MetricValue key={metric.id} metric={metric} />)}
        </div>
      </button>
      <div className="provider-card-foot">
        <div><SourceBadge source={account.source} /><span>{account.updatedLabel}</span></div>
        <OfficialLink href={account.officialUrl} compact />
      </div>
    </article>
  );
}

function MetricValue({ metric }: { metric: Metric }) {
  return (
    <div className="metric-value" title={`${metric.scope} · ${metric.source}`}>
      <span>{metric.label}</span>
      <strong>{metric.value}</strong>
      {metric.window && <small>{metric.window}</small>}
    </div>
  );
}

function SubscriptionCard({ account, onSelect }: { account: SubscriptionAccount; onSelect: (account: ProviderAccount) => void }) {
  return (
    <article className="subscription-card">
      <button className="subscription-card-detail" onClick={() => onSelect(account)} aria-label={`Open ${account.name} details`}>
        <div className="subscription-head">
          <div className="provider-identity compact">
            <ProviderLogo provider={account.provider} />
            <div><h3>{account.name}</h3><p>{account.plan}</p></div>
          </div>
          <StatusBadge status={account.status} />
        </div>
        <div className="source-line"><SourceBadge source={account.source} /><span>{account.scope}</span></div>
        <div className="quota-stack">
          {account.metrics.filter((metric) => metric.kind === "quota").map((metric) => {
            return (
              <div className="quota" key={metric.id}>
                <div><span>{metric.label}</span><strong>{metric.value}</strong></div>
                <div className="progress-track"><span style={{ width: `${metric.numericValue}%` }} /></div>
                <small>{metric.resetTime ? `Resets in ${metric.resetTime}` : metric.window}</small>
              </div>
            );
          })}
        </div>
        <div className="plan-line"><span>{account.monthlyPrice || "Price not set"}</span><span>{account.renewalDate || "Renewal not set"}</span><em>Manual</em></div>
      </button>
      <div className="subscription-foot"><span>Updated {account.updatedLabel}</span><OfficialLink href={account.officialUrl} compact /></div>
    </article>
  );
}

function ProviderLogo({ provider }: { provider: ProviderAccount["provider"] | SubscriptionAccount["provider"] }) {
  const icon = provider === "deepseek" ? siDeepseek : provider === "openrouter" ? siOpenrouter : provider === "google-gemini-cli" ? siGooglegemini : provider === "google-ai-studio" ? siGoogle : null;
  const tone = provider === "deepseek" ? "#4d7cff" : provider === "openrouter" ? "#8a74ff" : provider.startsWith("google-") ? "#6b8cff" : "#67a5ff";
  return (
    <span className={`provider-logo ${provider}`} style={{ color: tone }} aria-hidden="true">
      {icon ? <svg viewBox="0 0 24 24"><path d={icon.path} fill="currentColor" /></svg> : <Atom weight="duotone" />}
    </span>
  );
}

function SourceBadge({ source }: { source: string }) {
  return <span className="source-badge"><Database />{source}</span>;
}

function StatusBadge({ status }: { status: Status }) {
  return <span className={`status-badge status-${status.toLowerCase().replaceAll(" ", "-")}`}><i />{status}</span>;
}

function OfficialLink({ href, compact = false }: { href: string; compact?: boolean }) {
  return (
    <a className={compact ? "official-link compact" : "official-link"} href={href} target="_blank" rel="noreferrer" onClick={(event) => { event.stopPropagation(); if (isTauri) { event.preventDefault(); void openOfficialUrl(href); } }}>
      {compact ? "Open" : "Open platform"}<ArrowSquareOut />
    </a>
  );
}

function SettingSelect({ label, value, onChange, options, suffix = "minutes" }: { label: string; value: number; onChange: (value: number) => void; options: number[]; suffix?: string }) {
  return <label className="setting-row"><span>{label}</span><select value={value} onChange={(event) => onChange(Number(event.target.value))}>{options.map((option) => <option value={option} key={option}>{option} {suffix}</option>)}</select></label>;
}

function SettingToggle({ label, description, checked, onChange }: { label: string; description: string; checked: boolean; onChange: (value: boolean) => void }) {
  return <label className="setting-row"><span><strong>{label}</strong><small>{description}</small></span><span className="switch"><input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} /><span /></span></label>;
}

type CloseGuardRegistration = (guard: (() => void) | null) => void;

function DialogFrame({ labelledBy, className = "", onRequestClose, children }: { labelledBy: string; className?: string; onRequestClose: () => void; children: ReactNode }) {
  const dialogRef = useRef<HTMLElement>(null);
  const closeRef = useRef(onRequestClose);
  useEffect(() => { closeRef.current = onRequestClose; }, [onRequestClose]);
  useEffect(() => {
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    dialogRef.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") { closeRef.current(); return; }
      if (event.key !== "Tab" || !dialogRef.current) return;
      const focusable = Array.from(dialogRef.current.querySelectorAll<HTMLElement>('button:not(:disabled), a[href], input:not(:disabled), select:not(:disabled), [tabindex]:not([tabindex="-1"])')).filter((element) => element.offsetParent !== null);
      if (focusable.length === 0) { event.preventDefault(); return; }
      const first = focusable[0];
      const last = focusable.at(-1)!;
      if (event.shiftKey && (document.activeElement === first || document.activeElement === dialogRef.current)) {
        event.preventDefault(); last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault(); first.focus();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.body.style.overflow = previousOverflow;
      previousFocus?.focus();
    };
  }, []);
  return (
    <div className="dialog-backdrop" onMouseDown={() => closeRef.current()}>
      <section ref={dialogRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby={labelledBy} className={`dialog ${className}`} onMouseDown={(event) => event.stopPropagation()}>
        {children}
      </section>
    </div>
  );
}

function DialogHeading({ eyebrow, title, copy, onBack }: { eyebrow: string; title: string; copy: string; onBack?: () => void }) {
  return (
    <div className="dialog-heading">
      {onBack ? <button className="dialog-back" onClick={onBack} aria-label="Back"><ArrowLeft /></button> : null}
      <div><p className="eyebrow">{eyebrow}</p><h2 id="connections-dialog-title">{title}</h2><p className="dialog-copy">{copy}</p></div>
    </div>
  );
}

function ConnectionsDialog({ entry, accounts, onClose, onAdded, onUpdated, onToggle, onRemove }: { entry: ConnectionsEntry; accounts: ProviderAccount[]; onClose: () => void; onAdded: (account: ProviderAccount) => void; onUpdated: (account: ProviderAccount) => void; onToggle: (id: string, enabled: boolean) => void; onRemove: (id: string) => Promise<void> }) {
  const [view, setView] = useState<ConnectionsView>(entry);
  const closeGuardRef = useRef<(() => void) | null>(null);
  const registerCloseGuard = useCallback<CloseGuardRegistration>((guard) => { closeGuardRef.current = guard; }, []);
  const requestClose = useCallback(() => closeGuardRef.current ? closeGuardRef.current() : onClose(), [onClose]);
  const account = "accountId" in view ? accounts.find((item) => item.id === view.accountId) ?? null : null;
  useEffect(() => { closeGuardRef.current = null; }, [view]);

  return (
    <DialogFrame labelledBy="connections-dialog-title" className="connections-dialog" onRequestClose={requestClose}>
      <button className="panel-close" onClick={requestClose} aria-label="Close connections"><X /></button>
      {view.kind === "list" ? (
        <ConnectionsList accounts={accounts} onAdd={() => setView({ kind: "add" })} onSelect={(accountId) => setView({ kind: "detail", accountId })} onToggle={onToggle} />
      ) : null}
      {view.kind === "add" ? (
        <AddAccountFlow
          onClose={onClose}
          registerCloseGuard={registerCloseGuard}
          onSaved={(saved) => { onAdded(saved); setView({ kind: "detail", accountId: saved.id }); }}
        />
      ) : null}
      {view.kind === "detail" && account ? (
        <AccountDetail account={account} onBack={() => setView({ kind: "list" })} onEdit={() => setView({ kind: "edit-details", accountId: account.id })} onReplace={() => setView({ kind: "replace-credential", accountId: account.id })} onRemove={() => setView({ kind: "remove-confirm", accountId: account.id })} onToggle={onToggle} />
      ) : null}
      {view.kind === "edit-details" && account ? (
        <EditAccountDetails account={account} registerCloseGuard={registerCloseGuard} onCancel={() => setView({ kind: "detail", accountId: account.id })} onSaved={(updated) => { onUpdated(updated); setView({ kind: "detail", accountId: updated.id }); }} />
      ) : null}
      {view.kind === "replace-credential" && account ? (
        <ReplaceCredential account={account} registerCloseGuard={registerCloseGuard} onCancel={() => setView({ kind: "detail", accountId: account.id })} onSaved={(updated) => { onUpdated(updated); setView({ kind: "detail", accountId: updated.id }); }} />
      ) : null}
      {view.kind === "remove-confirm" && account ? (
        <RemoveAccountConfirm account={account} onCancel={() => setView({ kind: "detail", accountId: account.id })} onConfirm={async () => { await onRemove(account.id); onClose(); }} />
      ) : null}
    </DialogFrame>
  );
}

function ConnectionsList({ accounts, onAdd, onSelect, onToggle }: { accounts: ProviderAccount[]; onAdd: () => void; onSelect: (accountId: string) => void; onToggle: (id: string, enabled: boolean) => void }) {
  const providers = accounts.filter((account) => account.type === "API Platform");
  const subscriptions = accounts.filter((account) => account.type === "Subscription");
  return (
    <>
      <div className="dialog-title-row">
        <DialogHeading eyebrow="CONNECTIONS" title="Accounts" copy="Add read-only sources and inspect connection health." />
        <button className="button primary" onClick={onAdd}><Plus />Add account</button>
      </div>
      <div className="connection-groups">
        <ConnectionGroup title="API Platforms" accounts={providers} onSelect={onSelect} onToggle={onToggle} />
        <ConnectionGroup title="Subscriptions" accounts={subscriptions} onSelect={onSelect} onToggle={onToggle} />
      </div>
    </>
  );
}

function ConnectionGroup({ title, accounts, onSelect, onToggle }: { title: string; accounts: ProviderAccount[]; onSelect: (accountId: string) => void; onToggle: (id: string, enabled: boolean) => void }) {
  return (
    <section className="connection-group">
      <div className="accounts-panel-head"><h3>{title}</h3><span>{accounts.length} connected</span></div>
      <div className="connection-list">
        {accounts.map((account) => (
          <div className="connection-row" key={account.id}>
            <button className="connection-row-main" onClick={() => onSelect(account.id)}>
              <ProviderLogo provider={account.provider} />
              <span className="connection-row-copy"><strong>{account.name}</strong><small>{account.type === "Subscription" ? account.plan || account.scope : account.credentialHint}</small></span>
              <StatusBadge status={account.enabled ? account.status : "Unavailable"} />
              <CaretRight />
            </button>
            <label className="switch"><input type="checkbox" checked={account.enabled} onChange={(event) => onToggle(account.id, event.target.checked)} aria-label={`Enable ${account.name}`} /><span /></label>
          </div>
        ))}
        {accounts.length === 0 ? <p className="connection-empty">No connected accounts in this category.</p> : null}
      </div>
    </section>
  );
}

function AccountDetail({ account, onBack, onEdit, onReplace, onRemove, onToggle }: { account: ProviderAccount; onBack: () => void; onEdit: () => void; onReplace: () => void; onRemove: () => void; onToggle: (id: string, enabled: boolean) => void }) {
  const max = Math.max(...account.history, 0);
  return (
    <>
      <DialogHeading eyebrow="ACCOUNT DETAIL" title={account.name} copy={`${account.scope} · Updated ${account.updatedLabel}`} onBack={onBack} />
      <div className="detail-summary-row">
        <div className="provider-identity"><ProviderLogo provider={account.provider} /><div><h3>{account.name}</h3><p>{account.credentialHint}</p></div></div>
        <StatusBadge status={account.enabled ? account.status : "Unavailable"} />
        <label className="setting-inline"><span>Monitoring</span><span className="switch"><input type="checkbox" checked={account.enabled} onChange={(event) => onToggle(account.id, event.target.checked)} aria-label={`Enable ${account.name}`} /><span /></span></label>
      </div>
      <div className="connections-dialog-body">
        <div className="detail-metrics">{account.metrics.map((metric) => <MetricValue key={metric.id} metric={metric} />)}</div>
        <section className="trend-section"><div><h3>{account.historyLabel}</h3><span>{account.historyBasis}</span></div>{account.history.length ? <div className="mini-chart" aria-label="Seven day trend">{account.history.map((value, index) => <span key={index} style={{ height: `${Math.max(18, max ? (value / max) * 100 : 18)}%` }} />)}</div> : <p className="history-empty">History starts after the first stored refresh.</p>}</section>
        <section className="diagnostic"><h3>Connection and data scope</h3><dl><div><dt>Source</dt><dd>{account.source}</dd></div><div><dt>Authentication</dt><dd>{account.authenticationMode} · owner: {account.credentialOwner}</dd></div>{account.identityLabel ? <div><dt>Identity</dt><dd>{account.identityLabel}</dd></div> : null}<div><dt>Credential</dt><dd>{account.credentialHint}</dd></div><div><dt>Last successful update</dt><dd>{account.updatedAt}</dd></div>{account.type === "Subscription" ? <div><dt>Plan metadata</dt><dd>{account.plan || "Not set"} · Manual</dd></div> : null}<div><dt>Failure behavior</dt><dd>Keep last success and mark Stale</dd></div></dl>{account.diagnostic ? <p>{account.diagnostic}</p> : null}</section>
        {account.lastError ? <p className="form-error" role="alert">{account.lastError}</p> : null}
      </div>
      <div className="dialog-actions detail-actions">
        <button className="button danger subtle-danger" onClick={onRemove}><Trash />Remove</button>
        <OfficialLink href={account.officialUrl} />
        {account.type === "API Platform" ? <button className="button secondary" onClick={onReplace}><Key />Replace credential</button> : null}
        <button className="button primary" onClick={onEdit}><PencilSimple />Edit details</button>
      </div>
    </>
  );
}

function SettingsDialog({ settings, error, onClose, onChange, onDeleteData }: { settings: AppSettings; error: string; onClose: () => void; onChange: (settings: AppSettings) => void; onDeleteData: () => Promise<void> }) {
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const update = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => onChange({ ...settings, [key]: value });
  return (
    <DialogFrame labelledBy="settings-dialog-title" className="settings-dialog" onRequestClose={onClose}>
      <button className="panel-close" onClick={onClose} aria-label="Close settings"><X /></button>
      <div className="dialog-heading"><div><p className="eyebrow">LOCAL APP</p><h2 id="settings-dialog-title">Settings</h2><p className="dialog-copy">Refresh behavior, Windows startup and local data.</p></div></div>
      {error ? <p className="form-error settings-error" role="alert">{error}</p> : null}
      {confirmingDelete ? (
        <div className="inline-confirmation">
          <Trash />
          <h3>Delete all local dashboard data?</h3>
          <p>This removes account configuration, metrics, history and local credential references from this PC. It does not revoke provider keys or sign out shared Codex or Gemini sessions.</p>
          <div className="dialog-actions"><button className="button secondary" onClick={() => setConfirmingDelete(false)}>Keep data</button><button className="button danger" onClick={() => void onDeleteData().then(() => setConfirmingDelete(false))}>Delete local data</button></div>
        </div>
      ) : (
        <div className="settings-layout">
          <section className="settings-card">
            <div className="settings-title"><ArrowClockwise /><div><h3>Refresh schedule</h3><p>Each provider refreshes independently.</p></div></div>
            <SettingSelect label="Official API sources" value={settings.apiRefreshMinutes} onChange={(value) => update("apiRefreshMinutes", value)} options={[5, 15, 30, 60]} />
            <SettingSelect label="CLI and OAuth sources" value={settings.localRefreshMinutes} onChange={(value) => update("localRefreshMinutes", value)} options={[15, 30, 60, 120]} />
          </section>
          <section className="settings-card">
            <div className="settings-title"><GearSix /><div><h3>Windows behavior</h3><p>Choose how background monitoring behaves.</p></div></div>
            <SettingToggle label="Start on login" description="Launch quietly after Windows sign-in." checked={settings.startOnLogin} onChange={(value) => update("startOnLogin", value)} />
            <SettingToggle label="Minimize to system tray" description="Keep background refresh active." checked={settings.minimizeToTray} onChange={(value) => update("minimizeToTray", value)} />
          </section>
          <section className="settings-card">
            <div className="settings-title"><ClockCounterClockwise /><div><h3>History</h3><p>Keep provider snapshots on this PC.</p></div></div>
            <SettingSelect label="Keep local history" value={settings.historyRetentionDays} onChange={(value) => update("historyRetentionDays", value)} options={[30, 60, 90, 180]} suffix="days" />
          </section>
          <section className="settings-card danger-zone">
            <div className="settings-title"><Trash /><div><h3>Local data</h3><p>Remove dashboard data without revoking provider access.</p></div></div>
            <button className="button danger" onClick={() => setConfirmingDelete(true)}><Trash />Delete local data</button>
          </section>
        </div>
      )}
    </DialogFrame>
  );
}

function AddAccountFlow({ onClose, onSaved, registerCloseGuard }: { onClose: () => void; onSaved: (account: ProviderAccount) => void; registerCloseGuard: CloseGuardRegistration }) {
  const [stage, setStage] = useState<"category" | "provider" | "connect">("category");
  const [category, setCategory] = useState<AccountCategory | null>(null);
  const [providerId, setProviderId] = useState<ProviderId | null>(null);
  const [testing, setTesting] = useState(false);
  const [tested, setTested] = useState(false);
  const [displayName, setDisplayName] = useState("");
  const [credential, setCredential] = useState("");
  const [openRouterMethod, setOpenRouterMethod] = useState<"oauth" | "api_key" | "management">("oauth");
  const [detection, setDetection] = useState<AuthenticationDetection | null>(null);
  const [attempt, setAttempt] = useState<AuthenticationAttempt | null>(null);
  const [error, setError] = useState("");
  const [touched, setTouched] = useState(false);
  const [discardAction, setDiscardAction] = useState<"back" | "close" | null>(null);
  const selectedProvider = providerOptions.find((option) => option.id === providerId) ?? null;
  const subscription = providerId === "chatgpt-codex" || providerId === "google-gemini-cli";
  const oauthOpenRouter = providerId === "openrouter" && openRouterMethod === "oauth";
  const secretRequired = !subscription && !oauthOpenRouter;
  const credentialKind = providerId === "openrouter" ? (openRouterMethod === "management" ? "management" : "api_key") : providerId === "openai" ? "admin" : subscription ? "local_oauth" : "api_key";
  const request: AccountRequest | null = providerId ? { providerId, displayName, credential, credentialKind, identityLabel: detection?.identityLabel, authenticationMode: detection?.authenticationMode } : null;
  const dirty = touched || tested || Boolean(detection) || Boolean(attempt);
  const clearConnection = useCallback(() => {
    if (attempt) void cancelAuthentication(attempt.attemptId);
    setAttempt(null); setDetection(null); setTested(false); setError(""); setCredential(""); setTouched(false); setOpenRouterMethod("oauth");
  }, [attempt]);
  const requestClose = useCallback(() => {
    if (dirty) setDiscardAction("close");
    else onClose();
  }, [dirty, onClose]);
  useEffect(() => {
    registerCloseGuard(requestClose);
    return () => registerCloseGuard(null);
  }, [registerCloseGuard, requestClose]);
  const back = () => {
    if (stage === "connect") {
      if (dirty) { setDiscardAction("back"); return; }
      clearConnection(); setProviderId(null); setStage("provider");
      return;
    }
    if (stage === "provider") { setCategory(null); setStage("category"); }
  };
  const confirmDiscard = () => {
    const action = discardAction;
    setDiscardAction(null);
    clearConnection();
    if (action === "close") onClose();
    else { setProviderId(null); setStage("provider"); }
  };
  const mockDetection = (): AuthenticationDetection => {
    if (providerId === "chatgpt-codex") return { providerId, availability: "existingSession", authenticationMode: "sharedLocalSession", credentialOwner: "codex", identityLabel: "user@example.com", planLabel: "Plus", scope: "Codex only", diagnostic: "Preview Codex account detection" };
    if (providerId === "google-gemini-cli") return { providerId, availability: "existingSession", authenticationMode: "localCliOauth", credentialOwner: "geminiCli", identityLabel: "user@gmail.com", planLabel: "Gemini CLI", scope: "Gemini CLI only · Experimental", diagnostic: "Preview Gemini CLI account detection" };
    return { providerId: "openrouter", availability: "existingSession", authenticationMode: "providerOauth", credentialOwner: "provider", identityLabel: "OpenRouter account", scope: "Key-level usage", diagnostic: "Preview OpenRouter authorization" };
  };
  const test = async () => {
    if (!providerId || !request) return;
    setTesting(true); setError(""); setTested(false); setDetection(null);
    try {
      if (subscription) {
        const found = isTauri ? await detectAuthentication(providerId) : mockDetection();
        setDetection(found);
        setTested(found.availability === "existingSession");
      } else {
        if (isTauri) await testConnection(request); else await new Promise((resolve) => window.setTimeout(resolve, 250));
        setTested(true);
      }
    } catch (reason) { setError(String(reason)); }
    finally { setTesting(false); }
  };
  const authorize = async (deviceCode = false) => {
    if (!providerId) return;
    setTesting(true); setError(""); setTested(false); setDetection(null);
    try {
      if (!isTauri) { await new Promise((resolve) => window.setTimeout(resolve, 250)); setDetection(mockDetection()); setTested(true); return; }
      const started = providerId === "openrouter" ? await startOpenRouterLogin() : await startCodexLogin(deviceCode);
      setAttempt(started);
      if (started.authorizationUrl) await openOfficialUrl(started.authorizationUrl);
      let current = started;
      while (current.status === "authorizing") {
        await new Promise((resolve) => window.setTimeout(resolve, 500));
        current = await pollAuthentication(started.attemptId);
        setAttempt(current);
      }
      if (current.status === "completed" && current.detection) { setDetection(current.detection); setTested(true); }
      else if (current.status !== "cancelled") setError(current.error || `Authorization ${current.status}`);
    } catch (reason) { setError(String(reason)); }
    finally { setTesting(false); }
  };
  const save = async () => {
    if (!providerId || !request) return;
    setTesting(true); setError("");
    try {
      const demo = [...demoProviders, ...demoSubscriptions].find((item) => item.provider === providerId);
      const account = isTauri
        ? oauthOpenRouter && attempt ? await saveOpenRouterOauthAccount(attempt.attemptId, displayName) : await saveConnectedAccount(request)
        : { ...demo!, id: crypto.randomUUID(), name: displayName };
      onSaved(account);
    } catch (reason) { setError(String(reason)); setTesting(false); }
  };
  const unavailable = detection && detection.availability !== "existingSession";
  if (discardAction) {
    return (
      <div className="inline-confirmation discard-confirmation">
        <ShieldCheck />
        <h2 id="connections-dialog-title">Discard this connection setup?</h2>
        <p>The entered credential and in-progress authorization will be cleared. Nothing has been saved.</p>
        <div className="dialog-actions"><button className="button secondary" onClick={() => setDiscardAction(null)}>Keep editing</button><button className="button danger" onClick={confirmDiscard}>Discard setup</button></div>
      </div>
    );
  }
  if (stage === "category") {
    return (
      <>
        <DialogHeading eyebrow="ADD ACCOUNT · STEP 1 OF 3" title="What do you want to connect?" copy="Choose the kind of usage you want to monitor. Authentication stays provider-specific in the next steps." />
        <div className="choice-grid category-choice-grid">
          <button className="choice-card" onClick={() => { setCategory("API Platform"); setStage("provider"); }}><Database /><span><strong>API Platform</strong><small>API spend, balance, key usage or project access.</small></span><CaretRight /></button>
          <button className="choice-card" onClick={() => { setCategory("Subscription"); setStage("provider"); }}><PlugsConnected /><span><strong>Subscription</strong><small>Quota from a confirmed Codex or Gemini CLI session.</small></span><CaretRight /></button>
        </div>
      </>
    );
  }
  if (stage === "provider") {
    return (
      <>
        <DialogHeading eyebrow="ADD ACCOUNT · STEP 2 OF 3" title={`Choose a ${category === "API Platform" ? "platform" : "subscription"}`} copy="Each provider shows its data scope and connection method before you continue." onBack={back} />
        <div className="provider-choice-list">
          {providerOptions.filter((option) => option.category === category).map((option) => (
            <button className="provider-choice" key={option.id} onClick={() => { clearConnection(); setProviderId(option.id); setDisplayName(option.label); setStage("connect"); }}>
              <ProviderLogo provider={option.id} />
              <span><strong>{option.label}{option.experimental ? <em>Experimental</em> : null}</strong><small>{option.description}</small><small>{option.authentication}</small></span>
              <CaretRight />
            </button>
          ))}
        </div>
      </>
    );
  }
  return (
    <form className="connection-form" onSubmit={(event) => { event.preventDefault(); void save(); }}>
        <DialogHeading eyebrow="ADD ACCOUNT · STEP 3 OF 3" title={`Connect ${selectedProvider?.label ?? "provider"}`} copy={`${selectedProvider?.description ?? "Provider data"}. Complete this provider's connection and confirmation flow.`} onBack={back} />
        <div className="selected-provider-summary"><ProviderLogo provider={providerId!} /><span><strong>{selectedProvider?.label}</strong><small>{selectedProvider?.authentication}</small></span></div>
        {providerId === "openrouter" ? <fieldset className="method-choices"><legend>Connection method</legend>{[
          ["oauth", "Connect with OpenRouter", "Recommended · browser PKCE"],
          ["api_key", "Normal API Key", "Key-level usage"],
          ["management", "Management Key", "Advanced account-credit scope"],
        ].map(([value, label, description]) => <button type="button" role="radio" aria-checked={openRouterMethod === value} className={openRouterMethod === value ? "method-choice active" : "method-choice"} key={value} onClick={() => { setOpenRouterMethod(value as typeof openRouterMethod); setTested(false); setDetection(null); setCredential(""); setTouched(true); }}><span><strong>{label}</strong><small>{description}</small></span><Check /></button>)}</fieldset> : null}
        <label>Account name<input value={displayName} onChange={(event) => { setDisplayName(event.target.value); setTouched(true); }} placeholder="Personal account" /></label>
        {secretRequired ? <label>{providerId === "openai" ? "Organization Admin Key" : providerId === "openrouter" && openRouterMethod === "management" ? "Management Key" : providerId === "google-ai-studio" ? "Gemini API Key" : "API Key"}<input value={credential} onChange={(event) => { setCredential(event.target.value); setTested(false); setTouched(true); }} type="password" placeholder="Paste credential" autoComplete="off" /></label> : null}
        {providerId === "openai" && <p className="field-help">Requires an Organization Admin Key with elevated organization access for Costs and Usage; it is not ChatGPT quota or prepaid balance. <button type="button" className="text-link" onClick={() => void openOfficialUrl("https://platform.openai.com/settings/organization/admin-keys")}>Get an Admin Key</button></p>}
        {providerId === "deepseek" && <p className="field-help">Reads account balance authorized by this key. Total, topped-up and granted balances remain separate. <button type="button" className="text-link" onClick={() => void openOfficialUrl("https://platform.deepseek.com/api_keys")}>Open API keys</button></p>}
        {providerId === "google-ai-studio" && <p className="field-help">Validates key/project API access only. Usage, spend and prepaid balance remain in AI Studio. <button type="button" className="text-link" onClick={() => void openOfficialUrl("https://aistudio.google.com/app/apikey")}>Get a Gemini API Key</button></p>}
        {providerId === "openrouter" && <p className="field-help">{oauthOpenRouter ? "Preferred: system-browser authorization returns a user-controlled normal key through localhost PKCE S256. OpenRouter does not define OAuth state for this flow." : openRouterMethod === "management" ? "Advanced account-level credits scope. This remains separate from normal key-level usage." : "Manual compatibility path for a normal key with key-level usage."}</p>}
        {detection ? <div className="consent-note"><ShieldCheck /><span><strong>{detection.identityLabel || (unavailable ? "No usable identity detected" : "Identity unavailable")}</strong><br />{detection.planLabel ? `${detection.planLabel} · ` : ""}{detection.scope}<br />{detection.diagnostic}</span></div> : <div className="consent-note"><ShieldCheck />{secretRequired ? "The new credential is tested before save or replacement and is never written to logs or SQLite." : providerId === "chatgpt-codex" ? "Detects the current Codex identity first and requires your confirmation. Scope is Codex quota only." : providerId === "google-gemini-cli" ? "Detects the official CLI session and requires confirmation. Scope is per-model Gemini CLI quota only · Experimental." : "Uses a random localhost callback, one-time code exchange and secure key storage."}</div>}
        {attempt?.status === "authorizing" && attempt.userCode ? <p className="field-help">Device code: <strong>{attempt.userCode}</strong></p> : null}
        {unavailable && providerId === "google-gemini-cli" ? <p className="field-help">Install the official Gemini CLI, run <code>gemini</code>, choose “Sign in with Google”, then use Check again. The dashboard does not automate the Google form.</p> : null}
        {error && <p className="form-error" role="alert">{error}</p>}
        <div className="dialog-actions">
          {attempt?.status === "authorizing" ? <button type="button" className="button secondary" onClick={() => { void cancelAuthentication(attempt.attemptId); setAttempt({ ...attempt, status: "cancelled" }); setTesting(false); }}>Cancel</button> : null}
          {providerId === "chatgpt-codex" && unavailable ? <><button type="button" className="button secondary" onClick={() => void authorize(true)}>Use device code</button><button type="button" className="button secondary" onClick={() => void authorize(false)}>Sign in with Codex</button></> : null}
          {providerId === "chatgpt-codex" && detection?.availability === "existingSession" ? <button type="button" className="button secondary" onClick={() => void authorize(false)}>Sign in with another account</button> : null}
          {oauthOpenRouter && !tested ? <button type="button" className="button secondary" disabled={testing} onClick={() => void authorize(false)}>{testing ? <ArrowClockwise className="spin" /> : <ArrowSquareOut />}{testing ? "Authorizing" : "Connect with OpenRouter"}</button> : null}
          {!oauthOpenRouter ? <button type="button" className="button secondary" disabled={testing} onClick={test}>{tested ? <Check /> : <ArrowClockwise className={testing ? "spin" : ""} />}{tested ? subscription ? "Identity confirmed" : "Connection valid" : testing ? "Checking" : detection ? "Check again" : subscription ? "Detect account" : "Test connection"}</button> : null}
          <button className="button primary" disabled={!tested || testing || !displayName.trim()}>{subscription ? "Connect this account" : "Save account"}</button>
        </div>
    </form>
  );
}

function EditAccountDetails({ account, onCancel, onSaved, registerCloseGuard }: { account: ProviderAccount; onCancel: () => void; onSaved: (account: ProviderAccount) => void; registerCloseGuard: CloseGuardRegistration }) {
  const [displayName, setDisplayName] = useState(account.name);
  const [planName, setPlanName] = useState(account.plan ?? "");
  const [monthlyPrice, setMonthlyPrice] = useState(account.monthlyPrice ?? "");
  const [renewalDate, setRenewalDate] = useState(account.renewalDate ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const dirty = displayName !== account.name || planName !== (account.plan ?? "") || monthlyPrice !== (account.monthlyPrice ?? "") || renewalDate !== (account.renewalDate ?? "");
  const requestCancel = useCallback(() => dirty ? setConfirmDiscard(true) : onCancel(), [dirty, onCancel]);
  useEffect(() => {
    registerCloseGuard(requestCancel);
    return () => registerCloseGuard(null);
  }, [registerCloseGuard, requestCancel]);
  const save = async () => {
    setSaving(true);
    setError("");
    const request: UpdateAccountRequest = {
      accountId: account.id,
      displayName,
      credentialKind: account.credentialKind,
      planName: account.type === "Subscription" ? planName : undefined,
      monthlyPrice: account.type === "Subscription" ? monthlyPrice : undefined,
      renewalDate: account.type === "Subscription" ? renewalDate : undefined,
    };
    try {
      const updated = isTauri
        ? await updateConnectedAccount(request)
        : { ...account, name: displayName, plan: planName, monthlyPrice, renewalDate };
      onSaved(updated);
    } catch (reason) {
      setError(String(reason));
      setSaving(false);
    }
  };
  if (confirmDiscard) return <div className="inline-confirmation"><PencilSimple /><h2 id="connections-dialog-title">Discard unsaved changes?</h2><p>Your account connection remains unchanged.</p><div className="dialog-actions"><button className="button secondary" onClick={() => setConfirmDiscard(false)}>Keep editing</button><button className="button danger" onClick={onCancel}>Discard changes</button></div></div>;
  return (
      <form className="connection-form" onSubmit={(event) => { event.preventDefault(); void save(); }}>
        <DialogHeading eyebrow="ACCOUNT DETAILS" title="Edit details" copy={account.type === "Subscription" ? "Quota remains automatic. Plan name, price and renewal are manual metadata." : "This changes the dashboard label only. The existing credential remains untouched."} onBack={requestCancel} />
        <label>Account name<input value={displayName} onChange={(event) => setDisplayName(event.target.value)} /></label>
        {account.type === "Subscription" ? <><label>Plan name · Manual<input value={planName} onChange={(event) => setPlanName(event.target.value)} /></label><label>Monthly price · Manual<input value={monthlyPrice} onChange={(event) => setMonthlyPrice(event.target.value)} /></label><label>Renewal · Manual<input value={renewalDate} onChange={(event) => setRenewalDate(event.target.value)} /></label></> : null}
        {error ? <p className="form-error" role="alert">{error}</p> : null}
        <div className="dialog-actions"><button type="button" className="button secondary" onClick={requestCancel}>Cancel</button><button className="button primary" disabled={saving || !displayName.trim() || !dirty}>{saving ? <ArrowClockwise className="spin" /> : <Check />}{saving ? "Saving" : "Save changes"}</button></div>
      </form>
  );
}

function ReplaceCredential({ account, onCancel, onSaved, registerCloseGuard }: { account: ProviderAccount; onCancel: () => void; onSaved: (account: ProviderAccount) => void; registerCloseGuard: CloseGuardRegistration }) {
  const [credential, setCredential] = useState("");
  const [credentialKind, setCredentialKind] = useState(account.credentialKind ?? "api_key");
  const [testing, setTesting] = useState(false);
  const [tested, setTested] = useState(false);
  const [error, setError] = useState("");
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const dirty = Boolean(credential) || credentialKind !== (account.credentialKind ?? "api_key") || tested;
  const requestCancel = useCallback(() => dirty ? setConfirmDiscard(true) : onCancel(), [dirty, onCancel]);
  useEffect(() => {
    registerCloseGuard(requestCancel);
    return () => registerCloseGuard(null);
  }, [registerCloseGuard, requestCancel]);
  const test = async () => {
    setTesting(true); setTested(false); setError("");
    try {
      if (isTauri) await testConnection({ providerId: account.provider, displayName: account.name, credential, credentialKind });
      else await new Promise((resolve) => window.setTimeout(resolve, 250));
      setTested(true);
    } catch (reason) { setError(String(reason)); }
    finally { setTesting(false); }
  };
  const save = async () => {
    setTesting(true); setError("");
    try {
      const updated = isTauri ? await updateConnectedAccount({ accountId: account.id, displayName: account.name, credential, credentialKind, planName: account.plan, monthlyPrice: account.monthlyPrice, renewalDate: account.renewalDate }) : { ...account, credentialKind, credentialHint: `${credentialKind === "management" ? "Management key" : account.provider === "openai" ? "Admin key" : "API key"} · Windows Credential Manager` };
      onSaved(updated);
    } catch (reason) { setError(String(reason)); setTesting(false); }
  };
  if (confirmDiscard) return <div className="inline-confirmation"><Key /><h2 id="connections-dialog-title">Discard the new credential?</h2><p>The saved credential remains active and unchanged.</p><div className="dialog-actions"><button className="button secondary" onClick={() => setConfirmDiscard(false)}>Keep editing</button><button className="button danger" onClick={onCancel}>Discard credential</button></div></div>;
  return (
    <form className="connection-form" onSubmit={(event) => { event.preventDefault(); void save(); }}>
      <DialogHeading eyebrow="ACCOUNT CREDENTIAL" title="Replace credential" copy="The new credential is tested before it replaces the saved Windows credential." onBack={requestCancel} />
      <div className="selected-provider-summary"><ProviderLogo provider={account.provider} /><span><strong>{account.name}</strong><small>{account.scope}</small></span></div>
      {account.provider === "openrouter" ? <fieldset className="method-choices"><legend>Credential scope</legend>{[["api_key", "Normal API Key", "Key-level usage"], ["management", "Management Key", "Advanced account credits"]].map(([value, label, description]) => <button type="button" role="radio" aria-checked={credentialKind === value} className={credentialKind === value ? "method-choice active" : "method-choice"} key={value} onClick={() => { setCredentialKind(value); setTested(false); }}><span><strong>{label}</strong><small>{description}</small></span><Check /></button>)}</fieldset> : null}
      <label>{account.provider === "openai" ? "Organization Admin Key" : account.provider === "openrouter" && credentialKind === "management" ? "Management Key" : account.provider === "google-ai-studio" ? "Gemini API Key" : "API Key"}<input value={credential} onChange={(event) => { setCredential(event.target.value); setTested(false); }} type="password" placeholder="Paste new credential" autoComplete="off" /></label>
      <div className="consent-note"><ShieldCheck />The current saved credential stays active until this replacement passes validation and you confirm the change.</div>
      {error ? <p className="form-error" role="alert">{error}</p> : null}
      <div className="dialog-actions"><button type="button" className="button secondary" onClick={requestCancel}>Cancel</button><button type="button" className="button secondary" disabled={testing || !credential.trim()} onClick={() => void test()}>{tested ? <Check /> : <ArrowClockwise className={testing ? "spin" : ""} />}{tested ? "Connection valid" : testing ? "Testing" : "Test connection"}</button><button className="button primary" disabled={!tested || testing}>Replace credential</button></div>
    </form>
  );
}

function RemoveAccountConfirm({ account, onCancel, onConfirm }: { account: ProviderAccount; onCancel: () => void; onConfirm: () => Promise<void> }) {
  const [removing, setRemoving] = useState(false);
  const [error, setError] = useState("");
  return (
    <div className="inline-confirmation remove-confirmation">
      <Trash />
      <h2 id="connections-dialog-title">Remove {account.name} from the dashboard?</h2>
      <p>This removes its local configuration, metrics and saved credential reference. It does not revoke a provider key or sign out a shared Codex or Gemini CLI session.</p>
      {error ? <p className="form-error" role="alert">{error}</p> : null}
      <div className="dialog-actions"><button className="button secondary" disabled={removing} onClick={onCancel}>Keep account</button><button className="button danger" disabled={removing} onClick={() => { setRemoving(true); setError(""); void onConfirm().catch((reason) => { setError(String(reason)); setRemoving(false); }); }}>{removing ? <ArrowClockwise className="spin" /> : <Trash />}{removing ? "Removing" : "Remove account"}</button></div>
    </div>
  );
}

function EmptyState({ onAdd }: { onAdd: () => void }) {
  return <div className="empty-state"><SlidersHorizontal /><h3>No API platforms yet</h3><p>Connect an API platform to start monitoring.</p><button className="button primary" onClick={onAdd}><Plus />Add account</button></div>;
}
