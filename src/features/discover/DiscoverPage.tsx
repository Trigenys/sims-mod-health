import { useEffect, useMemo, useState } from "react";
import { Topbar } from "../../components/layout/Topbar";
import { Panel } from "../../components/ui/Panel";
import { useI18n } from "../../i18n/I18nProvider";
import { discoverGateway, type DiscoverGateway } from "./discover.gateway";
import type { DiscoveryRecommendation, DiscoverySnapshot } from "./discover.types";

type DiscoverPageProps = {
  gateway?: DiscoverGateway;
};

export function DiscoverPage({ gateway = discoverGateway }: DiscoverPageProps) {
  const { t } = useI18n();
  const [snapshot, setSnapshot] = useState<DiscoverySnapshot | null>(null);
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("All");

  useEffect(() => {
    let active = true;
    gateway.load().then((value) => {
      if (active) setSnapshot(value);
    });
    return () => {
      active = false;
    };
  }, [gateway]);

  const categories = useMemo(() => {
    const values = snapshot?.recommendations.flatMap((item) => item.categories) ?? [];
    return ["All", ...Array.from(new Set(values)).sort()];
  }, [snapshot]);

  const visible = useMemo(() => {
    if (!snapshot) return [];
    const needle = query.trim().toLocaleLowerCase();

    return snapshot.recommendations.filter((item) => {
      const matchesQuery =
        !needle ||
        [
          item.name,
          item.creatorName,
          item.reason.becauseModName,
          ...item.categories,
          ...item.features
        ]
          .join(" ")
          .toLocaleLowerCase()
          .includes(needle);
      const matchesCategory =
        category === "All" || item.categories.includes(category);
      return matchesQuery && matchesCategory;
    });
  }, [category, query, snapshot]);

  if (!snapshot) {
    return (
      <section className="discover-loading" role="status">
        {t("discover.loading")}
      </section>
    );
  }

  return (
    <>
      <Topbar
        gameVersion={snapshot.patchVersion ? "Patch " + snapshot.patchVersion : t("common.patchUnknown")}
        platform={t("common.windows")}
        indexedCount={snapshot.recommendations.length}
      />

      <section className="discover-hero" aria-labelledby="discover-title">
        <div>
          <p className="eyebrow">{t("discover.eyebrow")}</p>
          <h1 id="discover-title">{t("discover.title")}</h1>
          <p className="lede">{t("discover.lede")}</p>
        </div>
        <div className="discover-count">
          <span>{t("discover.safeCandidates")}</span>
          <strong>{snapshot.recommendations.length}</strong>
        </div>
      </section>

      <section className="discover-toolbar" aria-label="Discover filters">
        <label className="discover-search">
          <span aria-hidden="true">⌕</span>
          <span className="sr-only">{t("discover.search")}</span>
          <input
            aria-label={t("discover.search")}
            placeholder={t("discover.searchPlaceholder")}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>

        <div className="discover-categories" aria-label="Recommendation categories">
          {categories.map((value) => (
            <button
              key={value}
              type="button"
              aria-pressed={category === value}
              className={
                category === value
                  ? "discover-chip discover-chip--active"
                  : "discover-chip"
              }
              onClick={() => setCategory(value)}
            >
              {humanize(value)}
            </button>
          ))}
        </div>
      </section>

      {snapshot.state !== "ready" && (
        <section className="discover-state" role="status">
          <span aria-hidden="true">!</span>
          <div>
            <strong>
              {snapshot.state === "offline"
                ? "Registry recommendations unavailable"
                : snapshot.state === "empty"
                  ? "No recommendation set yet"
                  : "Recommendation evidence is partial"}
            </strong>
            <p>{snapshot.detail}</p>
          </div>
        </section>
      )}

      {visible.length === 0 ? (
        <Panel className="discover-empty">
          <div className="discover-empty__icon" aria-hidden="true">✦</div>
          <div>
            <h2>
              {snapshot.recommendations.length === 0
                ? "No safe recommendation currently passes the filters"
                : "No recommendation matches these filters"}
            </h2>
            <p>
              Discover never fills empty space with unsafe or invented suggestions.
              Current-patch compatibility and known-conflict filters run before ranking.
            </p>
          </div>
        </Panel>
      ) : (
        <section className="discover-grid" aria-label="Safe recommendations">
          {visible.map((item) => (
            <RecommendationCard item={item} key={item.releaseId} />
          ))}
        </section>
      )}

      <Panel className="discover-principle">
        <div className="discover-principle__icon" aria-hidden="true">♡</div>
        <div>
          <span className="section-kicker">{t("discover.assurance")}</span>
          <h2>{t("discover.assuranceTitle")}</h2>
          <p>
            Already-installed mods, explicitly incompatible releases and candidates without current-patch compatibility evidence are filtered out before ranking.
          </p>
        </div>
      </Panel>
    </>
  );
}

function RecommendationCard({ item }: { item: DiscoveryRecommendation }) {
  const { t } = useI18n();
  return (
    <Panel as="article" className="recommendation-card">
      <div className="recommendation-card__top">
        <div className="recommendation-avatar" aria-hidden="true">
          {item.name.slice(0, 2).toUpperCase()}
        </div>
        <div className="recommendation-card__identity">
          <strong>{item.name}</strong>
          <span>{item.creatorName}</span>
        </div>
        <span className="recommendation-safe">{t("discover.compatible")}</span>
      </div>

      <p className="recommendation-reason">{item.reason.explanation}</p>

      <div className="recommendation-tags">
        {item.categories.map((tag) => (
          <span key={"category-" + tag}>{humanize(tag)}</span>
        ))}
        {item.features.slice(0, 2).map((tag) => (
          <span key={"feature-" + tag}>{humanize(tag)}</span>
        ))}
      </div>

      <div className="recommendation-card__evidence">
        <span>{t("discover.because")}</span>
        <strong>{item.reason.becauseModName}</strong>
      </div>

      <div className="recommendation-card__footer">
        <span>Deterministic score {item.score}</span>
        <button type="button">
          {t("discover.review")}
          <span aria-hidden="true">→</span>
        </button>
      </div>
    </Panel>
  );
}

function humanize(value: string) {
  return value
    .split(/[-_]/g)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}
