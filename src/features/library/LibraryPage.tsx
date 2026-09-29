import { useEffect, useMemo, useState, type ReactNode } from "react";
import { Topbar } from "../../components/layout/Topbar";
import { StatusBadge } from "../../components/ui/StatusBadge";
import { LibraryStateNotice, type LibraryRegistryState } from "./LibraryStateNotice";
import {
  libraryGateway,
  type LibraryGateway,
  type LibrarySnapshot
} from "./library.gateway";
import type {
  IdentificationConfidence,
  LibraryItem
} from "./library.fixture";
import { useI18n } from "../../i18n/i18n";

type LibraryPageProps = {
  onOpenItem: (item: LibraryItem) => void;
  registryState?: LibraryRegistryState;
  gateway?: LibraryGateway;
};

type Filters = {
  status: string;
  category: string;
  creator: string;
  source: string;
  kind: string;
  identification: string;
  enabled: string;
};

const initialFilters: Filters = {
  status: "All",
  category: "All",
  creator: "All",
  source: "All",
  kind: "All",
  identification: "All",
  enabled: "All"
};

const confidenceLabel: Record<IdentificationConfidence, string> = {
  exact: "Exact",
  high: "High confidence",
  medium: "Medium confidence",
  unresolved: "Not identified yet"
};

export function LibraryPage({
  onOpenItem,
  registryState = "ready",
  gateway = libraryGateway
}: LibraryPageProps) {
  const { t, tx } = useI18n();
  const [snapshot, setSnapshot] = useState<LibrarySnapshot | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [filters, setFilters] = useState<Filters>(initialFilters);

  useEffect(() => {
    let active = true;

    gateway
      .load()
      .then((next) => {
        if (active) {
          setSnapshot(next);
          setLoadError(null);
        }
      })
      .catch((error) => {
        if (active) {
          setSnapshot({
            hasInstallation: false,
            gameVersion: null,
            modsRoot: null,
            indexedCount: 0,
            items: []
          });
          setLoadError(error instanceof Error ? error.message : String(error));
        }
      });

    return () => {
      active = false;
    };
  }, [gateway]);

  const items = snapshot?.items ?? [];

  const facets = useMemo(
    () => ({
      statuses: [...new Set(items.map((item) => item.status))].sort(),
      categories: [...new Set(items.map((item) => item.category))].sort(),
      creators: [...new Set(items.map((item) => item.creator))].sort(),
      sources: [...new Set(items.map((item) => item.source))].sort()
    }),
    [items]
  );

  const visibleItems = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase();

    return items.filter((item) => {
      const searchable = [
        item.canonicalName,
        item.creator,
        ...item.filenameAliases
      ]
        .join(" ")
        .toLocaleLowerCase();

      if (needle && !searchable.includes(needle)) return false;
      if (filters.status !== "All" && item.status !== filters.status) return false;
      if (filters.category !== "All" && item.category !== filters.category) return false;
      if (filters.creator !== "All" && item.creator !== filters.creator) return false;
      if (filters.source !== "All" && item.source !== filters.source) return false;
      if (filters.kind !== "All" && item.kind !== filters.kind) return false;
      if (filters.identification === "Identified" && !item.identified) return false;
      if (filters.identification === "Unknown" && item.identified) return false;
      if (filters.enabled === "Enabled" && !item.enabled) return false;
      if (filters.enabled === "Disabled" && item.enabled) return false;
      return true;
    });
  }, [filters, items, query]);

  const updateFilter = (key: keyof Filters, value: string) => {
    setFilters((currentFilters) => ({ ...currentFilters, [key]: value }));
  };

  const resetFilters = () => {
    setQuery("");
    setFilters(initialFilters);
  };

  const patchLabel = snapshot?.gameVersion
    ? t("Patch {{version}}", { version: snapshot.gameVersion })
    : t("Game version not detected yet");

  return (
    <>
      <Topbar
        gameVersion={patchLabel}
        platform="Windows"
        indexedCount={snapshot?.indexedCount ?? 0}
      />

      <section className="library-heading" aria-labelledby="library-title">
        <div>
          <p className="eyebrow">{t("LOCAL INVENTORY")}</p>
          <h1 id="library-title">{t("Library")}</h1>
          <p className="lede">
            {t("These are the files found in your Mods folder. If we cannot identify a mod yet, we say so instead of guessing.")}
          </p>
          {snapshot?.modsRoot && (
            <p className="detail-muted">
              <code>{snapshot.modsRoot}</code>
            </p>
          )}
        </div>
        <div className="library-summary" aria-label={t("{{count}} visible items", { count: visibleItems.length })}>
          <strong>{visibleItems.length}</strong>
          <span>{t("visible items")}</span>
        </div>
      </section>

      <LibraryStateNotice state={registryState} />

      {loadError && (
        <section className="library-empty" role="alert">
          <div className="library-empty__icon" aria-hidden="true">!</div>
          <h2>{t("Library could not be loaded")}</h2>
          <p>{loadError}</p>
        </section>
      )}

      {!snapshot ? (
        <section className="library-empty" role="status">
          <div className="library-empty__icon" aria-hidden="true">…</div>
          <h2>{t("Loading local inventory")}</h2>
          <p>{t("Reading the latest scan from local storage.")}</p>
        </section>
      ) : !snapshot.hasInstallation ? (
        <section className="library-empty">
          <div className="library-empty__icon" aria-hidden="true">⌂</div>
          <h2>{t("No Mods folder scanned yet")}</h2>
          <p>
            {t("Go to Overview, choose your real Sims 4 Mods folder, and run the first scan. This page will not invent sample mods.")}
          </p>
        </section>
      ) : snapshot.items.length === 0 ? (
        <section className="library-empty">
          <div className="library-empty__icon" aria-hidden="true">0</div>
          <h2>{t("No supported mod files found")}</h2>
          <p>
            {t("The selected folder was scanned, but no .package or .ts4script files were indexed.")}
          </p>
        </section>
      ) : (
        <>
          <section className="library-toolbar" aria-label={t("Library search and filters")}>
            <label className="library-search">
              <span aria-hidden="true">⌕</span>
              <span className="sr-only">{t("Search local filename")}</span>
              <input
                aria-label={t("Search local filename")}
                placeholder={t("Search filename or folder…")}
                value={query}
                onChange={(event) => setQuery(event.target.value)}
              />
            </label>

            <div className="filter-strip">
              <Filter label={t("Status")} value={filters.status} onChange={(value) => updateFilter("status", value)}>
                {facets.statuses.map((value) => <option key={value} value={value}>{tx(value)}</option>)}
              </Filter>
              <Filter label={t("Category")} value={filters.category} onChange={(value) => updateFilter("category", value)}>
                {facets.categories.map((value) => <option key={value} value={value}>{tx(value)}</option>)}
              </Filter>
              <Filter label={t("Creator")} value={filters.creator} onChange={(value) => updateFilter("creator", value)}>
                {facets.creators.map((value) => <option key={value} value={value}>{value}</option>)}
              </Filter>
              <Filter label={t("Source")} value={filters.source} onChange={(value) => updateFilter("source", value)}>
                {facets.sources.map((value) => <option key={value} value={value}>{tx(value)}</option>)}
              </Filter>
              <Filter label={t("Type")} value={filters.kind} onChange={(value) => updateFilter("kind", value)}>
                <option value="Script mod">{t("Script mod")}</option>
                <option value="Package only">{t("Package only")}</option>
              </Filter>
              <Filter
                label={t("Identification")}
                value={filters.identification}
                onChange={(value) => updateFilter("identification", value)}
              >
                <option value="Identified">{t("Identified")}</option>
                <option value="Unknown">{t("Unknown")}</option>
              </Filter>
              <Filter label={t("State")} value={filters.enabled} onChange={(value) => updateFilter("enabled", value)}>
                <option value="Enabled">{t("Enabled")}</option>
                <option value="Disabled">{t("Disabled")}</option>
              </Filter>
            </div>
          </section>

          {visibleItems.length === 0 ? (
            <section className="library-empty" aria-live="polite">
              <div className="library-empty__icon" aria-hidden="true">⌕</div>
              <h2>{t("No matching items")}</h2>
              <p>
                {t("Try a filename or folder name, or clear the active filters.")}
              </p>
              <button className="button button--secondary" onClick={resetFilters}>
                {t("Clear filters")}
              </button>
            </section>
          ) : (
            <section className="library-table" aria-label={t("Installed library")}>
              <div className="library-table__header" aria-hidden="true">
                <span>{t("Mod / CC")}</span>
                <span>{t("Installed")}</span>
                <span>{t("Status")}</span>
                <span>{t("Source")}</span>
                <span>{t("Identity")}</span>
                <span />
              </div>

              <div className="library-table__body">
                {visibleItems.map((item) => (
                  <button
                    className="library-row"
                    key={item.id}
                    onClick={() => onOpenItem(item)}
                    aria-label={t("Open {{name}}", { name: item.canonicalName })}
                  >
                    <span className="library-row__identity">
                      <span className="library-avatar" aria-hidden="true">
                        {item.canonicalName.slice(0, 2).toUpperCase()}
                      </span>
                      <span>
                        <strong>{item.canonicalName}</strong>
                        <small>
                          {tx(item.creator)} · {tx(item.category)}
                          {!item.enabled && " · " + t("Disabled")}
                        </small>
                        {!item.identified && (
                          <em>{t("Local file")} · {item.filenameAliases[0]}</em>
                        )}
                      </span>
                    </span>
                    <span className="library-version">{item.installedVersion ?? "—"}</span>
                    <span><StatusBadge tone={item.tone}>{tx(item.status)}</StatusBadge></span>
                    <span className="library-source">{tx(item.source)}</span>
                    <span>
                      {item.confidence === "exact" ? (
                        <span className="identity-exact">{t("Exact")}</span>
                      ) : (
                        <span className={"identity-confidence identity-confidence--" + item.confidence}>
                          {tx(confidenceLabel[item.confidence])}
                        </span>
                      )}
                    </span>
                    <span className="row-arrow" aria-hidden="true">›</span>
                  </button>
                ))}
              </div>
            </section>
          )}
        </>
      )}
    </>
  );
}

type FilterProps = {
  label: string;
  value: string;
  onChange: (value: string) => void;
  children: ReactNode;
};

function Filter({ label, value, onChange, children }: FilterProps) {
  const { t } = useI18n();
  return (
    <label className="filter-control">
      <span>{label}</span>
      <select
        aria-label={t("Filter by {{label}}", { label: label.toLocaleLowerCase() })}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      >
        <option value="All">{t("All")}</option>
        {children}
      </select>
    </label>
  );
}
