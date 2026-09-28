import brandLogo from "../../assets/brand-logo.svg";
type NavItem = {
  label: "Overview" | "Library" | "Health" | "Discover";
  icon: string;
};

const navItems: NavItem[] = [
  { label: "Overview", icon: "⌂" },
  { label: "Library", icon: "▦" },
  { label: "Health", icon: "♥" },
  { label: "Discover", icon: "✦" }
];

type SidebarProps = {
  activeItem?: string;
  healthCount?: number;
  onNavigate?: (label: string) => void;
};

export function Sidebar({
  activeItem = "Overview",
  healthCount,
  onNavigate
}: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <img className="brand-logo" src={brandLogo} alt="" aria-hidden="true" />
        <div className="brand-copy">
          <strong>Sims Mod Health</strong>
          <span>Offline desktop engine</span>
        </div>
      </div>

      <nav className="nav-list" aria-label="Primary navigation">
        {navItems.map((item) => {
          const active = item.label === activeItem;
          const count =
            item.label === "Health" && healthCount && healthCount > 0
              ? healthCount
              : undefined;

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
              {count !== undefined && (
                <span
                  className="nav-count"
                  aria-label={count + " health findings"}
                >
                  {count}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      <div className="sidebar-footer">
        <div className="sync-state" role="status">
          <span className="status-dot" aria-hidden="true" />
          <span>Registry available</span>
        </div>
        <button
          aria-current={activeItem === "Settings" ? "page" : undefined}
          className={
            activeItem === "Settings"
              ? "settings-link settings-link--active"
              : "settings-link"
          }
          aria-label="Settings"
          onClick={() => onNavigate?.("Settings")}
        >
          <span aria-hidden="true">⚙</span>
          <span>Settings</span>
        </button>
      </div>
    </aside>
  );
}
