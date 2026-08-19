import { useEffect, useState } from "react";
import {
  ArrowSquareOut,
  Atom,
  CardsThree,
  Check,
  ClockCounterClockwise,
  Database,
  GearSix,
  ListBullets,
  Moon,
  PencilSimple,
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
  Page,
  ProviderAccount,
  ProviderId,
  Status,
  SubscriptionAccount,
  Theme,
} from "./types";

const nav: { id: Page; label: string }[] = [
  { id: "dashboard", label: "Dashboard" },
  { id: "accounts", label: "Accounts" },
  { id: "settings", label: "Settings" },
];

function storedValue<T extends string>(key: string, fallback: T): T {
  return (localStorage.getItem(key) as T | null) ?? fallback;
}

export function App() {
  const [page, setPage] = useState<Page>(() => storedValue("aud-page", "dashboard"));
  const [theme, setTheme] = useState<Theme>(() => storedValue("aud-theme", "dark"));
  const [layout, setLayout] = useState<LayoutMode>(() => storedValue("aud-layout", "list"));
  const [accounts, setAccounts] = useState<ProviderAccount[]>(() => isTauri ? [] : [...demoProviders, ...demoSubscriptions]);
  const [settings, setSettings] = useState(defaultSettings);
  const [settingsError, setSettingsError] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editing, setEditing] = useState<ProviderAccount | null>(null);
  const [addOpen, setAddOpen] = useState(false);
  const [isRefreshing, setRefreshing] = useState(false);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("aud-theme", theme);
  }, [theme]);

  useEffect(() => localStorage.setItem("aud-layout", layout), [layout]);
  useEffect(() => localStorage.setItem("aud-page", page), [page]);

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
  const selected = selectedId ? accounts.find((account) => account.id === selectedId) ?? null : null;

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
        page={page}
        theme={theme}
        onPageChange={setPage}
        onThemeChange={setTheme}
      />
      <main className="app-main">
        {page === "dashboard" && (
          <Dashboard
            layout={layout}
            providers={providers}
            subscriptions={subscriptions}
            refreshing={isRefreshing}
            onLayoutChange={setLayout}
            onRefresh={() => void refreshAll()}
            onSelect={(account) => setSelectedId(account.id)}
          />
        )}
        {page === "accounts" && (
          <AccountsPage
            providers={providers}
            subscriptions={subscriptions}
            onAdd={() => setAddOpen(true)}
            onToggle={(id, enabled) => {
              if (!isTauri) {
                setAccounts((items) => items.map((item) => item.id === id ? { ...item, enabled } : item));
                return;
              }
              void setConnectedAccountEnabled(id, enabled).then((updated) => {
                setAccounts((items) => items.map((item) => item.id === id ? updated : item));
              });
            }}
            onEdit={setEditing}
            onDelete={async (id) => {
              await removeConnectedAccount(id);
              setAccounts((items) => items.filter((item) => item.id !== id));
            }}
          />
        )}
        {page === "settings" && (
          <SettingsPage
            settings={settings}
            error={settingsError}
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
        )}
      </main>
      {selected && <DetailPanel account={selected} onClose={() => setSelectedId(null)} />}
      {addOpen && <AddAccountDialog onClose={() => setAddOpen(false)} onSaved={(account) => setAccounts((items) => [...items, account])} />}
      {editing && <EditAccountDialog account={editing} onClose={() => setEditing(null)} onSaved={(account) => {
        setAccounts((items) => items.map((item) => item.id === account.id ? account : item));
        setEditing(null);
      }} />}
    </div>
  );
}

function AppHeader({
  page,
  theme,
  onPageChange,
  onThemeChange,
}: {
  page: Page;
  theme: Theme;
  onPageChange: (page: Page) => void;
  onThemeChange: (theme: Theme) => void;
}) {
  return (
    <header className="topbar">
      <button className="brand" onClick={() => onPageChange("dashboard")} aria-label="Open Dashboard">
        <span className="brand-mark"><SidebarSimple weight="fill" /></span>
        <span>AI Usage</span>
      </button>
      <nav className="primary-nav" aria-label="Primary navigation">
        {nav.map((item) => (
          <button
            key={item.id}
            className={page === item.id ? "nav-item active" : "nav-item"}
            onClick={() => onPageChange(item.id)}
          >
            {item.label}
          </button>
        ))}
      </nav>
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
  onSelect,
}: {
  layout: LayoutMode;
  providers: ProviderAccount[];
  subscriptions: SubscriptionAccount[];
  refreshing: boolean;
  onLayoutChange: (layout: LayoutMode) => void;
  onRefresh: () => void;
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
            ) : <EmptyState />}
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
    <article className="provider-row" onClick={() => onSelect(account)}>
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
      <OfficialLink href={account.officialUrl} />
    </article>
  );
}

function ProviderCard({ account, onSelect }: { account: ProviderAccount; onSelect: (account: ProviderAccount) => void }) {
  return (
    <article className="provider-card" onClick={() => onSelect(account)}>
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
    <article className="subscription-card" onClick={() => onSelect(account)}>
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

function AccountsPage({
  providers,
  subscriptions,
  onAdd,
  onToggle,
  onEdit,
  onDelete,
}: {
  providers: ProviderAccount[];
  subscriptions: SubscriptionAccount[];
  onAdd: () => void;
  onToggle: (id: string, enabled: boolean) => void;
  onEdit: (account: ProviderAccount) => void;
  onDelete: (id: string) => void;
}) {
  return (
    <>
      <section className="page-heading compact-heading">
        <div><p className="eyebrow">CONNECTIONS</p><h1>Accounts</h1><p>Add read-only sources and inspect their connection health.</p></div>
        <button className="button primary" onClick={onAdd}><Plus />Add account</button>
      </section>
      <section className="accounts-panel">
        <div className="accounts-panel-head"><h2>API Platforms</h2><span>{providers.length} connected</span></div>
        <div className="accounts-table" role="table" aria-label="API platform accounts">
          {providers.map((account) => (
            <div className="account-line" role="row" key={account.id}>
              <div className="provider-identity"><ProviderLogo provider={account.provider} /><div><h3>{account.name}</h3><p>{account.credentialHint}</p></div></div>
              <div className="account-source"><SourceBadge source={account.source} /><small>{account.scope}</small></div>
              <StatusBadge status={account.enabled ? account.status : "Unavailable"} />
              <label className="switch"><input type="checkbox" checked={account.enabled} onChange={(event) => onToggle(account.id, event.target.checked)} aria-label={`Enable ${account.name}`} /><span /></label>
              <button className="icon-button" onClick={() => onEdit(account)} aria-label={`Edit ${account.name}`}><PencilSimple /></button>
              <button className="icon-button danger" onClick={() => onDelete(account.id)} aria-label={`Delete ${account.name}`}><Trash /></button>
            </div>
          ))}
        </div>
      </section>
      <section className="accounts-panel">
        <div className="accounts-panel-head"><h2>Subscriptions</h2><span>{subscriptions.length} connected</span></div>
        <div className="accounts-table" role="table" aria-label="Subscription accounts">
          {subscriptions.map((account) => (
            <div className="account-line" role="row" key={account.id}>
              <div className="provider-identity"><ProviderLogo provider={account.provider} /><div><h3>{account.name}</h3><p>{account.plan}</p></div></div>
              <div className="account-source"><SourceBadge source={account.source} /><small>{account.scope} · {account.monthlyPrice || "Price not set"}</small></div>
              <StatusBadge status={account.enabled ? account.status : "Unavailable"} />
              <label className="switch"><input type="checkbox" checked={account.enabled} onChange={(event) => onToggle(account.id, event.target.checked)} aria-label={`Enable ${account.name}`} /><span /></label>
              <button className="icon-button" onClick={() => onEdit(account)} aria-label={`Edit ${account.name}`}><PencilSimple /></button>
              <button className="icon-button danger" onClick={() => onDelete(account.id)} aria-label={`Delete ${account.name}`}><Trash /></button>
            </div>
          ))}
        </div>
      </section>
    </>
  );
}

function SettingsPage({ settings, error, onChange, onDeleteData }: { settings: AppSettings; error: string; onChange: (settings: AppSettings) => void; onDeleteData: () => Promise<void> }) {
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const update = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => onChange({ ...settings, [key]: value });
  return (
    <>
      <section className="page-heading compact-heading"><div><p className="eyebrow">LOCAL APP</p><h1>Settings</h1><p>Refresh behavior, Windows startup and local data.</p></div></section>
      {error ? <p className="form-error settings-error" role="alert">{error}</p> : null}
      <div className="settings-layout">
        <section className="settings-card">
          <div className="settings-title"><ArrowClockwise /><div><h2>Refresh schedule</h2><p>Each provider refreshes independently with exponential backoff.</p></div></div>
          <SettingSelect label="Official API sources" value={settings.apiRefreshMinutes} onChange={(value) => update("apiRefreshMinutes", value)} options={[5, 15, 30, 60]} />
          <SettingSelect label="CLI, OAuth and browser sources" value={settings.localRefreshMinutes} onChange={(value) => update("localRefreshMinutes", value)} options={[15, 30, 60, 120]} />
        </section>
        <section className="settings-card">
          <div className="settings-title"><GearSix /><div><h2>Windows behavior</h2><p>Keep monitoring even when the main window is closed.</p></div></div>
          <SettingToggle label="Start on login" description="Launch quietly after you sign in to Windows." checked={settings.startOnLogin} onChange={(value) => update("startOnLogin", value)} />
          <SettingToggle label="Minimize to system tray" description="Closing the window keeps background refresh active." checked={settings.minimizeToTray} onChange={(value) => update("minimizeToTray", value)} />
        </section>
        <section className="settings-card">
          <div className="settings-title"><ClockCounterClockwise /><div><h2>History</h2><p>Provider history and balance snapshots remain separate.</p></div></div>
          <SettingSelect label="Keep local history" value={settings.historyRetentionDays} onChange={(value) => update("historyRetentionDays", value)} options={[30, 60, 90, 180]} suffix="days" />
        </section>
        <section className="settings-card danger-zone">
          <div className="settings-title"><Trash /><div><h2>Local data</h2><p>Delete account configuration, metrics and credential references from this PC.</p></div></div>
          <button className="button danger" onClick={() => {
            if (!confirmingDelete) { setConfirmingDelete(true); return; }
            void onDeleteData().then(() => setConfirmingDelete(false));
          }}><Trash />{confirmingDelete ? "Confirm permanent deletion" : "Delete local data"}</button>
        </section>
      </div>
    </>
  );
}

function SettingSelect({ label, value, onChange, options, suffix = "minutes" }: { label: string; value: number; onChange: (value: number) => void; options: number[]; suffix?: string }) {
  return <label className="setting-row"><span>{label}</span><select value={value} onChange={(event) => onChange(Number(event.target.value))}>{options.map((option) => <option value={option} key={option}>{option} {suffix}</option>)}</select></label>;
}

function SettingToggle({ label, description, checked, onChange }: { label: string; description: string; checked: boolean; onChange: (value: boolean) => void }) {
  return <label className="setting-row"><span><strong>{label}</strong><small>{description}</small></span><span className="switch"><input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} /><span /></span></label>;
}

function DetailPanel({ account, onClose }: { account: ProviderAccount; onClose: () => void }) {
  const max = Math.max(...account.history, 0);
  return (
    <div className="panel-backdrop" onMouseDown={onClose}>
      <aside className="detail-panel" onMouseDown={(event) => event.stopPropagation()} aria-label={`${account.name} details`}>
        <button className="panel-close" onClick={onClose} aria-label="Close details"><X /></button>
        <div className="provider-identity detail-identity"><ProviderLogo provider={account.provider} /><div><p className="eyebrow">ACCOUNT DETAIL</p><h2>{account.name}</h2><p>{account.scope}</p></div></div>
        <StatusBadge status={account.status} />
        <div className="detail-metrics">{account.metrics.map((metric) => <MetricValue key={metric.id} metric={metric} />)}</div>
        <section className="trend-section"><div><h3>{account.historyLabel}</h3><span>{account.historyBasis}</span></div>{account.history.length ? <div className="mini-chart" aria-label="Seven day trend">{account.history.map((value, index) => <span key={index} style={{ height: `${Math.max(18, max ? (value / max) * 100 : 18)}%` }} />)}</div> : <p className="history-empty">History starts after the first stored refresh.</p>}</section>
        <section className="diagnostic"><h3>Source diagnostic</h3><dl><div><dt>Strategy</dt><dd>{account.source}</dd></div><div><dt>Authentication</dt><dd>{account.authenticationMode} · owner: {account.credentialOwner}</dd></div>{account.identityLabel ? <div><dt>Identity</dt><dd>{account.identityLabel}</dd></div> : null}<div><dt>Credential</dt><dd>{account.credentialHint}</dd></div><div><dt>Last observed</dt><dd>{account.updatedAt}</dd></div>{account.type === "Subscription" ? <div><dt>Plan metadata</dt><dd>{account.plan || "Not set"} · Manual</dd></div> : null}<div><dt>Failure behavior</dt><dd>Keep last success and mark Stale</dd></div></dl>{account.diagnostic ? <p>{account.diagnostic}</p> : null}</section>
        {account.lastError ? <p className="form-error" role="alert">{account.lastError}</p> : null}
        <OfficialLink href={account.officialUrl} />
      </aside>
    </div>
  );
}

function AddAccountDialog({ onClose, onSaved }: { onClose: () => void; onSaved: (account: ProviderAccount) => void }) {
  const [testing, setTesting] = useState(false);
  const [tested, setTested] = useState(false);
  const [providerId, setProviderId] = useState<ProviderId>("openai");
  const [displayName, setDisplayName] = useState("Personal account");
  const [credential, setCredential] = useState("");
  const [openRouterMethod, setOpenRouterMethod] = useState<"oauth" | "api_key" | "management">("oauth");
  const [planName, setPlanName] = useState("");
  const [monthlyPrice, setMonthlyPrice] = useState("");
  const [renewalDate, setRenewalDate] = useState("");
  const [detection, setDetection] = useState<AuthenticationDetection | null>(null);
  const [attempt, setAttempt] = useState<AuthenticationAttempt | null>(null);
  const [error, setError] = useState("");
  const subscription = providerId === "chatgpt-codex" || providerId === "google-gemini-cli";
  const oauthOpenRouter = providerId === "openrouter" && openRouterMethod === "oauth";
  const secretRequired = !subscription && !oauthOpenRouter;
  const credentialKind = providerId === "openrouter" ? (openRouterMethod === "management" ? "management" : "api_key") : providerId === "openai" ? "admin" : subscription ? "local_oauth" : "api_key";
  const request: AccountRequest = { providerId, displayName, credential, credentialKind, planName: subscription ? planName : undefined, monthlyPrice: subscription ? monthlyPrice : undefined, renewalDate: subscription ? renewalDate : undefined, identityLabel: detection?.identityLabel, authenticationMode: detection?.authenticationMode };
  const reset = (next?: ProviderId) => { if (attempt) void cancelAuthentication(attempt.attemptId); if (next) setProviderId(next); setAttempt(null); setDetection(null); setTested(false); setError(""); setCredential(""); };
  const close = () => { if (attempt) void cancelAuthentication(attempt.attemptId); onClose(); };
  const mockDetection = (): AuthenticationDetection => ({ providerId, availability: "existingSession", authenticationMode: providerId === "chatgpt-codex" ? "sharedLocalSession" : "localCliOauth", credentialOwner: providerId === "chatgpt-codex" ? "codex" : "geminiCli", identityLabel: providerId === "chatgpt-codex" ? "user@example.com" : "user@gmail.com", planLabel: providerId === "chatgpt-codex" ? "plus" : "Gemini CLI", scope: providerId === "chatgpt-codex" ? "Codex only" : "Gemini CLI only · Experimental", diagnostic: "Preview account detection" });
  const test = async () => {
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
    setTesting(true); setError("");
    try {
      const demo = [...demoProviders, ...demoSubscriptions].find((item) => item.provider === providerId);
      const account = isTauri
        ? oauthOpenRouter && attempt ? await saveOpenRouterOauthAccount(attempt.attemptId, displayName) : await saveConnectedAccount(request)
        : { ...demo!, id: crypto.randomUUID(), name: displayName, plan: planName || demo?.plan, monthlyPrice: monthlyPrice || demo?.monthlyPrice, renewalDate: renewalDate || demo?.renewalDate };
      onSaved(account); onClose();
    } catch (reason) { setError(String(reason)); setTesting(false); }
  };
  const unavailable = detection && detection.availability !== "existingSession";
  return (
    <div className="dialog-backdrop" onMouseDown={close}>
      <form className="dialog" onMouseDown={(event) => event.stopPropagation()} onSubmit={(event) => { event.preventDefault(); void save(); }}>
        <button type="button" className="panel-close" onClick={close} aria-label="Close"><X /></button>
        <p className="eyebrow">READ-ONLY CONNECTION</p><h2>Add account</h2><p className="dialog-copy">Each provider keeps its own authentication boundary. Secrets stay in Windows Credential Manager; shared Codex and Gemini sessions remain owned by their installed clients.</p>
        <label>Provider<select value={providerId} disabled={testing} onChange={(event) => reset(event.target.value as ProviderId)}><option value="openai">OpenAI API</option><option value="deepseek">DeepSeek API</option><option value="openrouter">OpenRouter</option><option value="google-ai-studio">Google AI Studio API</option><option value="chatgpt-codex">ChatGPT / Codex subscription</option><option value="google-gemini-cli">Google / Gemini CLI subscription · Experimental</option></select></label>
        {providerId === "openrouter" && <label>Connection method<select value={openRouterMethod} disabled={testing} onChange={(event) => { setOpenRouterMethod(event.target.value as typeof openRouterMethod); setTested(false); setDetection(null); setCredential(""); }}><option value="oauth">Connect with OpenRouter · PKCE</option><option value="api_key">Paste normal API key · key-level</option><option value="management">Add Management Key · advanced account scope</option></select></label>}
        <label>Account name<input value={displayName} onChange={(event) => setDisplayName(event.target.value)} placeholder="Personal account" /></label>
        {secretRequired ? <label>{providerId === "openai" ? "Organization Admin Key" : providerId === "openrouter" && openRouterMethod === "management" ? "Management Key" : providerId === "google-ai-studio" ? "Gemini API Key" : "API Key"}<input value={credential} onChange={(event) => { setCredential(event.target.value); setTested(false); }} type="password" placeholder="Paste credential" autoComplete="off" /></label> : null}
        {providerId === "openai" && <p className="field-help">Requires an Organization Admin Key with elevated organization access for Costs and Usage; it is not ChatGPT quota or prepaid balance. <button type="button" className="text-link" onClick={() => void openOfficialUrl("https://platform.openai.com/settings/organization/admin-keys")}>Get an Admin Key</button></p>}
        {providerId === "deepseek" && <p className="field-help">Reads account balance authorized by this key. Total, topped-up and granted balances remain separate. <button type="button" className="text-link" onClick={() => void openOfficialUrl("https://platform.deepseek.com/api_keys")}>Open API keys</button></p>}
        {providerId === "google-ai-studio" && <p className="field-help">Validates key/project API access only. Usage, spend and prepaid balance remain in AI Studio. <button type="button" className="text-link" onClick={() => void openOfficialUrl("https://aistudio.google.com/app/apikey")}>Get a Gemini API Key</button></p>}
        {providerId === "openrouter" && <p className="field-help">{oauthOpenRouter ? "Preferred: system-browser authorization returns a user-controlled normal key through localhost PKCE S256. OpenRouter does not define OAuth state for this flow." : openRouterMethod === "management" ? "Advanced account-level credits scope. This remains separate from normal key-level usage." : "Manual compatibility path for a normal key with key-level usage."}</p>}
        {subscription ? <><label>Plan name · Manual metadata<input value={planName} onChange={(event) => setPlanName(event.target.value)} placeholder={providerId === "chatgpt-codex" ? "ChatGPT Plus" : "Optional plan label"} /></label><label>Monthly price · Manual<input value={monthlyPrice} onChange={(event) => setMonthlyPrice(event.target.value)} placeholder="$20 / month" /></label><label>Renewal · Manual<input value={renewalDate} onChange={(event) => setRenewalDate(event.target.value)} placeholder="Renews Sep 3" /></label></> : null}
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
          <button className="button primary" disabled={!tested || testing || !displayName.trim()}>{subscription ? "Use this account" : "Save account"}</button>
        </div>
      </form>
    </div>
  );
}

function EditAccountDialog({ account, onClose, onSaved }: { account: ProviderAccount; onClose: () => void; onSaved: (account: ProviderAccount) => void }) {
  const [displayName, setDisplayName] = useState(account.name);
  const [credential, setCredential] = useState("");
  const [credentialKind, setCredentialKind] = useState(account.credentialKind ?? "api_key");
  const [planName, setPlanName] = useState(account.plan ?? "");
  const [monthlyPrice, setMonthlyPrice] = useState(account.monthlyPrice ?? "");
  const [renewalDate, setRenewalDate] = useState(account.renewalDate ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const save = async () => {
    setSaving(true);
    setError("");
    const request: UpdateAccountRequest = {
      accountId: account.id,
      displayName,
      credential: credential.trim() || undefined,
      credentialKind,
      planName: account.type === "Subscription" ? planName : undefined,
      monthlyPrice: account.type === "Subscription" ? monthlyPrice : undefined,
      renewalDate: account.type === "Subscription" ? renewalDate : undefined,
    };
    try {
      const updated = isTauri
        ? await updateConnectedAccount(request)
        : { ...account, name: displayName, credentialKind, plan: planName, monthlyPrice, renewalDate };
      onSaved(updated);
    } catch (reason) {
      setError(String(reason));
      setSaving(false);
    }
  };
  return (
    <div className="dialog-backdrop" onMouseDown={onClose}>
      <form className="dialog" onMouseDown={(event) => event.stopPropagation()} onSubmit={(event) => { event.preventDefault(); void save(); }}>
        <button type="button" className="panel-close" onClick={onClose} aria-label="Close"><X /></button>
        <p className="eyebrow">ACCOUNT SETTINGS</p><h2>Edit account</h2><p className="dialog-copy">{account.type === "Subscription" ? "Quota is automatic; plan name, price and renewal remain manual." : "Leave the credential blank to keep the existing secret in Windows Credential Manager."}</p>
        <label>Account name<input value={displayName} onChange={(event) => setDisplayName(event.target.value)} /></label>
        {account.provider === "openrouter" ? <label>Credential scope<select value={credentialKind} onChange={(event) => setCredentialKind(event.target.value)}><option value="api_key">API key · key-level usage</option><option value="management">Management · account credits</option></select></label> : null}
        {account.type === "API Platform" ? <label>Replace credential<input value={credential} onChange={(event) => setCredential(event.target.value)} type="password" placeholder="Leave blank to keep current credential" autoComplete="off" /></label> : <><label>Plan name · Manual<input value={planName} onChange={(event) => setPlanName(event.target.value)} /></label><label>Monthly price · Manual<input value={monthlyPrice} onChange={(event) => setMonthlyPrice(event.target.value)} /></label><label>Renewal · Manual<input value={renewalDate} onChange={(event) => setRenewalDate(event.target.value)} /></label></>}
        {account.provider === "openrouter" && credentialKind !== account.credentialKind && !credential ? <p className="field-help">Changing OpenRouter scope requires the corresponding new credential.</p> : null}
        {error ? <p className="form-error" role="alert">{error}</p> : null}
        <div className="dialog-actions"><button type="button" className="button secondary" onClick={onClose}>Cancel</button><button className="button primary" disabled={saving || !displayName.trim()}>{saving ? <ArrowClockwise className="spin" /> : <Check />}{saving ? "Saving" : "Save changes"}</button></div>
      </form>
    </div>
  );
}

function EmptyState() {
  return <div className="empty-state"><SlidersHorizontal /><h3>No API platforms yet</h3><p>Add an account from Accounts to start monitoring.</p></div>;
}
