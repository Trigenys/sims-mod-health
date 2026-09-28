import { useI18n } from "../../i18n/I18nProvider";
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
  const { t } = useI18n();
  const navLabel = (label: NavItem["label"]) => {
    if (label === "Overview") return t("nav.overview");
    if (label === "Library") return t("nav.library");
    if (label === "Health") return t("nav.health");
    return t("nav.discover");
  };

  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark" aria-hidden="true">SMH</div>
        <div className="brand-copy">
          <strong>Sims Mod Health</strong>
          <span>{t("sidebar.engine")}</span>
        </div>
      </div>

      <nav className="nav-list" aria-label={t("nav.primary")}>
        {navItems.map((item) => {
          const active = item.label === activeItem;
          const count =
            item.label === "Health" && healthCount && healthCount > 0
              ? healthCount
              : undefined;

          return (
            <button
              aria-current={active ? "page" : undefined}
              aria-label={navLabel(item.label)}
              className={active ? "nav-item active" : "nav-item"}
              key={item.label}
              onClick={() => onNavigate?.(item.label)}
            >
              <span className="nav-icon" aria-hidden="true">{item.icon}</span>
              <span className="nav-label">{navLabel(item.label)}</span>
              {count !== undefined && (
                <span
                  className="nav-count"
                  aria-label={t("sidebar.healthFindings", { count })}
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
          <span>{t("sidebar.registry")}</span>
        </div>
        <button
          aria-current={activeItem === "Settings" ? "page" : undefined}
          className={
            activeItem === "Settings"
              ? "settings-link settings-link--active"
              : "settings-link"
          }
          aria-label={t("nav.settings")}
          onClick={() => onNavigate?.("Settings")}
        >
          <span aria-hidden="true">⚙</span>
          <span>{t("nav.settings")}</span>
        </button>
      </div>
    </aside>
  );
}
