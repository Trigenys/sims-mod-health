import { LanguageSwitcher } from "./LanguageSwitcher";
import { useI18n } from "../../i18n/i18n";

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

  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark" aria-hidden="true">SMH</div>
        <div className="brand-copy">
          <strong>Sims Mod Health</strong>
          <span>{t("Offline desktop engine")}</span>
        </div>
      </div>

      <nav className="nav-list" aria-label={t("Primary navigation")}>
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
              <span className="nav-label">{t(item.label)}</span>
              {count !== undefined && (
                <span
                  className="nav-count"
                  aria-label={t("{{count}} health findings", { count })}
                >
                  {count}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      <div className="sidebar-footer">
        <LanguageSwitcher />
        <div className="sync-state" role="status">
          <span className="status-dot" aria-hidden="true" />
          <span>{t("Registry available")}</span>
        </div>
        <button
          aria-current={activeItem === "Settings" ? "page" : undefined}
          className={
            activeItem === "Settings"
              ? "settings-link settings-link--active"
              : "settings-link"
          }
          aria-label={t("Settings")}
          onClick={() => onNavigate?.("Settings")}
        >
          <span aria-hidden="true">⚙</span>
          <span>{t("Settings")}</span>
        </button>
      </div>
    </aside>
  );
}
