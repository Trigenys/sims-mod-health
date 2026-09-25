const navItems = [
  ["Overview", "⌂"],
  ["Library", "▦"],
  ["Updates", "↻"],
  ["Conflicts", "⚠"],
  ["Diagnostics", "⌁"],
  ["Discover", "✦"],
  ["Backups", "◫"]
] as const;

const stats = [
  { label: "Healthy", value: "267", tone: "healthy" },
  { label: "Updates", value: "21", tone: "update" },
  { label: "Conflicts", value: "7", tone: "warning" },
  { label: "Unknown", value: "25", tone: "muted" }
] as const;

const attention = [
  {
    name: "MC Command Center",
    creator: "Deaderpool",
    detail: "Update available · installed 2026.4.0",
    badge: "Update",
    tone: "update"
  },
  {
    name: "Relationship & Pregnancy Overhaul",
    creator: "Lumpinou",
    detail: "Compatibility not confirmed for current patch",
    badge: "Unknown",
    tone: "muted"
  },
  {
    name: "Duplicate CAS package",
    creator: "Local library",
    detail: "2 exact copies share the same SHA-256 fingerprint",
    badge: "Duplicate",
    tone: "warning"
  }
] as const;

function App() {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">SM</div>
          <div>
            <strong>Sims Mod Health</strong>
            <span>by Trigenys</span>
          </div>
        </div>

        <nav className="nav-list" aria-label="Primary">
          {navItems.map(([label, icon], index) => (
            <button className={index === 0 ? "nav-item active" : "nav-item"} key={label}>
              <span className="nav-icon" aria-hidden="true">{icon}</span>
              <span>{label}</span>
              {label === "Updates" && <span className="nav-count">21</span>}
            </button>
          ))}
        </nav>

        <div className="sidebar-footer">
          <div className="sync-state">
            <span className="status-dot" />
            Registry synced
          </div>
          <button className="settings-link">Settings</button>
        </div>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div>
            <span className="context-label">THE SIMS 4</span>
            <div className="version-row">
              <strong>Patch 1.128.90</strong>
              <span>Windows</span>
            </div>
          </div>
          <div className="topbar-actions">
            <label className="search-box">
              <span aria-hidden="true">⌕</span>
              <input placeholder="Search 324 mods & CC" aria-label="Search library" />
            </label>
            <button className="primary-button">Scan now</button>
          </div>
        </header>

        <section className="page-heading">
          <div>
            <p className="eyebrow">LIBRARY HEALTH</p>
            <h1>Your mods are mostly healthy.</h1>
            <p className="lede">28 items need attention before the next long save session.</p>
          </div>
          <div className="health-score" aria-label="Overall health 87 percent">
            <div className="score-ring"><span>87</span><small>%</small></div>
            <div><strong>Overall health</strong><span>Last scan 4 min ago</span></div>
          </div>
        </section>

        <section className="stat-grid" aria-label="Health summary">
          {stats.map((stat) => (
            <article className="stat-card" key={stat.label}>
              <span className={`status-pip ${stat.tone}`} />
              <div><strong>{stat.value}</strong><span>{stat.label}</span></div>
            </article>
          ))}
        </section>

        <section className="content-grid">
          <article className="panel attention-panel">
            <div className="panel-header">
              <div>
                <span className="section-kicker">Needs attention</span>
                <h2>Resolve the risky stuff first</h2>
              </div>
              <button className="text-button">View all</button>
            </div>

            <div className="attention-list">
              {attention.map((item) => (
                <button className="attention-row" key={item.name}>
                  <div className={`item-avatar ${item.tone}`}>{item.name.slice(0, 2).toUpperCase()}</div>
                  <div className="item-copy">
                    <strong>{item.name}</strong>
                    <span>{item.creator} · {item.detail}</span>
                  </div>
                  <span className={`badge ${item.tone}`}>{item.badge}</span>
                  <span className="row-arrow" aria-hidden="true">›</span>
                </button>
              ))}
            </div>
          </article>

          <aside className="panel scan-panel">
            <span className="section-kicker">Current installation</span>
            <h2>324 items indexed</h2>
            <div className="scan-meter"><span /></div>
            <dl className="scan-facts">
              <div><dt>Mods</dt><dd>43</dd></div>
              <div><dt>Custom content</dt><dd>177</dd></div>
              <div><dt>Unidentified</dt><dd>25</dd></div>
              <div><dt>Exact duplicates</dt><dd>18</dd></div>
            </dl>
            <button className="secondary-button">Open scan details</button>
          </aside>
        </section>

        <section className="panel recommendation-strip">
          <div className="recommendation-icon">✦</div>
          <div>
            <span className="section-kicker">Discover</span>
            <h2>Your library leans toward relationships, family and realism.</h2>
            <p>Recommendations will only show mods compatible with your patch and filtered against known conflicts.</p>
          </div>
          <button className="secondary-button">Explore similar mods</button>
        </section>
      </main>
    </div>
  );
}

export default App;
