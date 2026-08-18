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
  Plus,
  ArrowClockwise,
  ShieldCheck,
  SidebarSimple,
  SlidersHorizontal,
  Sun,
  Trash,
  X,
} from "@phosphor-icons/react";
import { siDeepseek, siGooglegemini, siOpenrouter } from "simple-icons";
import { defaultSettings, demoProviders, demoSubscriptions } from "./demoData";
import {
  isTauri,
  deleteAllLocalData,
  loadConnectedAccounts,
  openOfficialUrl,
  refreshConnectedAccount,
  removeConnectedAccount,
  saveConnectedAccount,
  setStartOnLogin,
  testConnection,
  type AccountRequest,
} from "./bridge";
import type {
  AppSettings,
  LayoutMode,
  Metric,
  Page,
  ProviderAccount,
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
  const [providers, setProviders] = useState(() => isTauri ? [] : demoProviders);
  const [subscriptions] = useState(() => isTauri ? [] : demoSubscriptions);
  const [settings, setSettings] = useState(defaultSettings);
  const [selected, setSelected] = useState<ProviderAccount | null>(null);
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
    loadConnectedAccounts().then(setProviders).catch(() => setProviders([]));
  }, []);

  const refreshAll = async () => {
    if (isRefreshing) return;
    setRefreshing(true);
    setProviders((items) => items.map((item) => ({ ...item, status: "Refreshing" })));
    if (isTauri) {
      const refreshed = await Promise.all(providers.map(refreshConnectedAccount));
      setProviders(refreshed);
      setRefreshing(false);
      return;
    }
    window.setTimeout(() => {
      setProviders((items) =>
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
            onSelect={setSelected}
          />
        )}
        {page === "accounts" && (
          <AccountsPage
            providers={providers}
            subscriptions={subscriptions}
            onAdd={() => setAddOpen(true)}
            onToggle={(id) =>
              setProviders((items) =>
                items.map((item) => (item.id === id ? { ...item, enabled: !item.enabled } : item)),
              )
            }
            onDelete={async (id) => {
              await removeConnectedAccount(id);
              setProviders((items) => items.filter((item) => item.id !== id));
            }}
          />
        )}
        {page === "settings" && (
          <SettingsPage
            settings={settings}
            onChange={(next) => {
              if (next.startOnLogin !== settings.startOnLogin) void setStartOnLogin(next.startOnLogin);
              setSettings(next);
            }}
            onDeleteData={async () => { await deleteAllLocalData(); setProviders([]); }}
          />
        )}
      </main>
      {selected && <DetailPanel account={selected} onClose={() => setSelected(null)} />}
      {addOpen && <AddAccountDialog onClose={() => setAddOpen(false)} onSaved={(account) => setProviders((items) => [...items, account])} />}
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
  return (
    <>
      <section className="page-heading">
        <div>
          <h1>AI Usage Dashboard</h1>
          <p>Last updated Aug 17, 2026 · 2:30 PM</p>
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
            <SummaryBlock label="USD spend" value="$42.18" meta="2 of 3 accounts" accent="blue" />
            <SummaryBlock label="CNY balance" value="¥82.31" meta="1 of 3 accounts" accent="violet" />
            <SummaryBlock label="Data coverage" value="5 connected" meta="3 API · 2 subscriptions" accent="green" />
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
              <SubscriptionCard key={subscription.id} account={subscription} />
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

function SubscriptionCard({ account }: { account: SubscriptionAccount }) {
  return (
    <article className="subscription-card">
      <div className="subscription-head">
        <div className="provider-identity compact">
          <ProviderLogo provider={account.provider} />
          <div><h3>{account.name}</h3><p>{account.plan}</p></div>
        </div>
        <StatusBadge status={account.status} />
      </div>
      <div className="source-line"><SourceBadge source={account.source} /><span>{account.scope}</span></div>
      <div className="quota-stack">
        {account.metrics.map((metric) => {
          const value = Number.parseInt(metric.value, 10);
          return (
            <div className="quota" key={metric.id}>
              <div><span>{metric.label}</span><strong>{metric.value}</strong></div>
              <div className="progress-track"><span style={{ width: `${value}%` }} /></div>
              <small>{metric.resetTime ? `Resets in ${metric.resetTime}` : metric.window}</small>
            </div>
          );
        })}
      </div>
      <div className="plan-line"><span>{account.monthlyPrice}</span><span>{account.renewalDate}</span><em>Manual</em></div>
      <div className="subscription-foot"><span>Updated {account.updatedLabel}</span><OfficialLink href={account.officialUrl} compact /></div>
    </article>
  );
}

function ProviderLogo({ provider }: { provider: ProviderAccount["provider"] | SubscriptionAccount["provider"] }) {
  const icon = provider === "deepseek" ? siDeepseek : provider === "openrouter" ? siOpenrouter : provider === "gemini" ? siGooglegemini : null;
  const tone = provider === "deepseek" ? "#4d7cff" : provider === "openrouter" ? "#8a74ff" : provider === "gemini" ? "#6b8cff" : "#67a5ff";
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
  onDelete,
}: {
  providers: ProviderAccount[];
  subscriptions: SubscriptionAccount[];
  onAdd: () => void;
  onToggle: (id: string) => void;
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
              <label className="switch"><input type="checkbox" checked={account.enabled} onChange={() => onToggle(account.id)} /><span /></label>
              <button className="icon-button danger" onClick={() => onDelete(account.id)} aria-label={`Delete ${account.name}`}><Trash /></button>
            </div>
          ))}
        </div>
      </section>
      <section className="accounts-panel">
        <div className="accounts-panel-head"><h2>Subscriptions</h2><span>{subscriptions.length} connected</span></div>
        <div className="accounts-table">
          {subscriptions.map((account) => (
            <div className="account-line subscription-line" key={account.id}>
              <div className="provider-identity"><ProviderLogo provider={account.provider} /><div><h3>{account.name}</h3><p>{account.plan}</p></div></div>
              <div className="account-source"><SourceBadge source={account.source} /><small>{account.scope}</small></div>
              <StatusBadge status={account.status} />
              <span className="manual-plan">{account.monthlyPrice} · {account.renewalDate}</span>
            </div>
          ))}
        </div>
      </section>
    </>
  );
}

function SettingsPage({ settings, onChange, onDeleteData }: { settings: AppSettings; onChange: (settings: AppSettings) => void; onDeleteData: () => Promise<void> }) {
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const update = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => onChange({ ...settings, [key]: value });
  return (
    <>
      <section className="page-heading compact-heading"><div><p className="eyebrow">LOCAL APP</p><h1>Settings</h1><p>Refresh behavior, Windows startup and local data.</p></div></section>
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
  const max = Math.max(...account.history);
  return (
    <div className="panel-backdrop" onMouseDown={onClose}>
      <aside className="detail-panel" onMouseDown={(event) => event.stopPropagation()} aria-label={`${account.name} details`}>
        <button className="panel-close" onClick={onClose} aria-label="Close details"><X /></button>
        <div className="provider-identity detail-identity"><ProviderLogo provider={account.provider} /><div><p className="eyebrow">ACCOUNT DETAIL</p><h2>{account.name}</h2><p>{account.scope}</p></div></div>
        <StatusBadge status={account.status} />
        <div className="detail-metrics">{account.metrics.map((metric) => <MetricValue key={metric.id} metric={metric} />)}</div>
        <section className="trend-section"><div><h3>7-day local trend</h3><span>Snapshots, not derived spend</span></div><div className="mini-chart" aria-label="Seven day trend">{account.history.map((value, index) => <span key={index} style={{ height: `${Math.max(18, (value / max) * 100)}%` }} />)}</div></section>
        <section className="diagnostic"><h3>Source diagnostic</h3><dl><div><dt>Strategy</dt><dd>{account.source}</dd></div><div><dt>Credential</dt><dd>{account.credentialHint}</dd></div><div><dt>Last observed</dt><dd>{account.updatedAt}</dd></div><div><dt>Failure behavior</dt><dd>Keep last success and mark Stale</dd></div></dl></section>
        <OfficialLink href={account.officialUrl} />
      </aside>
    </div>
  );
}

function AddAccountDialog({ onClose, onSaved }: { onClose: () => void; onSaved: (account: ProviderAccount) => void }) {
  const [testing, setTesting] = useState(false);
  const [tested, setTested] = useState(false);
  const [providerId, setProviderId] = useState<ProviderAccount["provider"]>("openai");
  const [displayName, setDisplayName] = useState("Personal account");
  const [credential, setCredential] = useState("");
  const [credentialKind, setCredentialKind] = useState("api_key");
  const [error, setError] = useState("");
  const request: AccountRequest = { providerId, displayName, credential, credentialKind };
  const test = async () => {
    setTesting(true); setError(""); setTested(false);
    try {
      if (isTauri) await testConnection(request);
      else await new Promise((resolve) => window.setTimeout(resolve, 650));
      setTested(true);
    } catch (reason) { setError(String(reason)); }
    finally { setTesting(false); }
  };
  const save = async () => {
    setTesting(true); setError("");
    try {
      const account = isTauri ? await saveConnectedAccount(request) : { ...demoProviders.find((item) => item.provider === providerId)!, id: crypto.randomUUID(), name: displayName };
      onSaved(account); onClose();
    } catch (reason) { setError(String(reason)); setTesting(false); }
  };
  return (
    <div className="dialog-backdrop" onMouseDown={onClose}>
      <form className="dialog" onMouseDown={(event) => event.stopPropagation()} onSubmit={(event) => { event.preventDefault(); void save(); }}>
        <button type="button" className="panel-close" onClick={onClose} aria-label="Close"><X /></button>
        <p className="eyebrow">READ-ONLY CONNECTION</p><h2>Add API account</h2><p className="dialog-copy">Credentials are stored in Windows Credential Manager. SQLite stores only a reference.</p>
        <label>Provider<select value={providerId} onChange={(event) => { setProviderId(event.target.value as ProviderAccount["provider"]); setTested(false); }}><option value="openai">OpenAI API</option><option value="deepseek">DeepSeek API</option><option value="openrouter">OpenRouter</option></select></label>
        {providerId === "openrouter" && <label>Credential scope<select value={credentialKind} onChange={(event) => { setCredentialKind(event.target.value); setTested(false); }}><option value="api_key">API key · key-level usage</option><option value="management">Management · account credits</option></select></label>}
        <label>Account name<input value={displayName} onChange={(event) => setDisplayName(event.target.value)} placeholder="Personal account" /></label>
        <label>{providerId === "openai" ? "Admin credential" : "Credential"}<input value={credential} onChange={(event) => { setCredential(event.target.value); setTested(false); }} type="password" placeholder="Paste credential" autoComplete="off" /></label>
        {providerId === "openai" && <p className="field-help">Organization usage requires an Admin Key; a normal API key is not treated as a balance credential.</p>}
        <div className="consent-note"><ShieldCheck />The credential is never written to app logs or the local database.</div>
        {error && <p className="form-error" role="alert">{error}</p>}
        <div className="dialog-actions"><button type="button" className="button secondary" onClick={test}>{tested ? <Check /> : <ArrowClockwise className={testing ? "spin" : ""} />}{tested ? "Connection valid" : testing ? "Testing" : "Test connection"}</button><button className="button primary" disabled={!tested}>Save account</button></div>
      </form>
    </div>
  );
}

function EmptyState() {
  return <div className="empty-state"><SlidersHorizontal /><h3>No API platforms yet</h3><p>Add an account from Accounts to start monitoring.</p></div>;
}
