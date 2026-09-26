type NavItem = {
  label: string;
  icon: string;
  count?: number;
};

const navItems: NavItem[] = [
  { label: "Overview", icon: "⌂" },
  { label: "Library", icon: "▦" },
  { label: "Updates", icon: "↻", count: 21 },
  { label: "Conflicts", icon: "⚠" },
  { label: "Diagnostics", icon: "⌁" },
  { label: "Discover", icon: "✦" },
  { label: "Backups", icon: "◫" }
];

type SidebarProps = {
  activeItem?: string;
  onNavigate?: (label: string) => void;
};

export function Sidebar({
  activeItem = "Overview",
  onNavigate
}: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark" aria-hidden="true">SM</div>
        <div className="brand-copy">
          <strong>Sims Mod Health</strong>
          <span>by Trigenys</span>
        </div>
      </div>

      <nav className="nav-list" aria-label="Primary navigation">
        {navItems.map((item) => {
          const active = item.label === activeItem;

          return (
            <button
              aria-current={active ? "page" : undefined}
              aria-label={item.label}
              className={active ? "nav-item active" : "nav-item"}
              key={item.label}
              onClick={() => onNavigate?.(item.label)}
            >
              <span className="nav-icon" aria-hidden="true">{item.icon}</span>
              <span className="nav-label">{item.label}</span>
              {item.count !== undefined && (
                <span className="nav-count" aria-label={item.count + " available updates"}>
                  {item.count}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      <div className="sidebar-footer">
        <div className="sync-state" role="status">
          <span className="status-dot" aria-hidden="true" />
          <span>Registry synced</span>
        </div>
        <button className="settings-link" aria-label="Settings">Settings</button>
      </div>
    </aside>
  );
}
