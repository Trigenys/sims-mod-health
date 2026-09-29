import { useEffect, useMemo, useState } from "react";
import { Topbar } from "../../components/layout/Topbar";
import { Panel } from "../../components/ui/Panel";
import { discoverGateway, type DiscoverGateway } from "./discover.gateway";
import type { DiscoveryRecommendation, DiscoverySnapshot } from "./discover.types";
import { useI18n } from "../../i18n/i18n";

type DiscoverPageProps = {
  gateway?: DiscoverGateway;
};

export function DiscoverPage({ gateway = discoverGateway }: DiscoverPageProps) {
  const { t, tx } = useI18n();
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
        {t("Building safe recommendations…")}
      </section>
    );
  }

  return (
    <>
      <Topbar
        gameVersion={snapshot.patchVersion ? t("Patch {{version}}", { version: snapshot.patchVersion }) : t("Patch unknown")}
        platform="Windows"
        indexedCount={snapshot.recommendations.length}
      />

      <section className="discover-hero" aria-labelledby="discover-title">
        <div>
          <p className="eyebrow">{t("SAFE ADDITIONS FOR YOUR GAME")}</p>
          <h1 id="discover-title">{t("Curated & safe additions")}</h1>
          <p className="lede">
            {t("Recommendations are filtered for compatibility and known conflicts before deterministic ranking.")}
          </p>
        </div>
        <div className="discover-count">
          <span>{t("Safe candidates")}</span>
          <strong>{snapshot.recommendations.length}</strong>
        </div>
      </section>

      <section className="discover-toolbar" aria-label={t("Discover filters")}>
        <label className="discover-search">
          <span aria-hidden="true">⌕</span>
          <span className="sr-only">{t("Search recommendations")}</span>
          <input
            aria-label={t("Search recommendations")}
            placeholder={t("Search curated recommendations…")}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>

        <div className="discover-categories" aria-label={t("Recommendation categories")}>
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
              {value === "All" ? t("All") : tx(humanize(value))}
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
                ? t("Registry recommendations unavailable")
                : snapshot.state === "empty"
                  ? t("No recommendation set yet")
                  : t("Recommendation evidence is partial")}
            </strong>
            <p>{tx(snapshot.detail)}</p>
          </div>
        </section>
      )}

      {visible.length === 0 ? (
        <Panel className="discover-empty">
          <div className="discover-empty__icon" aria-hidden="true">✦</div>
          <div>
            <h2>
              {snapshot.recommendations.length === 0
                ? t("No safe recommendation currently passes the filters")
                : t("No recommendation matches these filters")}
            </h2>
            <p>
              {t("Discover never fills empty space with unsafe or invented suggestions. Current-patch compatibility and known-conflict filters run before ranking.")}
            </p>
          </div>
        </Panel>
      ) : (
        <section className="discover-grid" aria-label={t("Safe recommendations")}>
          {visible.map((item) => (
            <RecommendationCard item={item} key={item.releaseId} />
          ))}
        </section>
      )}

      <Panel className="discover-principle">
        <div className="discover-principle__icon" aria-hidden="true">♡</div>
        <div>
          <span className="section-kicker">{t("SIM MOD HEALTH SAFETY ASSURANCE")}</span>
          <h2>{t("Recommendations never outrank health evidence.")}</h2>
          <p>
            {t("Already-installed mods, explicitly incompatible releases and candidates without current-patch compatibility evidence are filtered out before ranking.")}
          </p>
        </div>
      </Panel>
    </>
  );
}

function RecommendationCard({ item }: { item: DiscoveryRecommendation }) {
  const { t, tx } = useI18n();
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
        <span className="recommendation-safe">{t("Compatible")}</span>
      </div>

      <p className="recommendation-reason">{tx(item.reason.explanation)}</p>

      <div className="recommendation-tags">
        {item.categories.map((tag) => (
          <span key={"category-" + tag}>{tx(humanize(tag))}</span>
        ))}
        {item.features.slice(0, 2).map((tag) => (
          <span key={"feature-" + tag}>{humanize(tag)}</span>
        ))}
      </div>

      <div className="recommendation-card__evidence">
        <span>{t("Because you use")}</span>
        <strong>{item.reason.becauseModName}</strong>
      </div>

      <div className="recommendation-card__footer">
        <span>{t("Deterministic score {{score}}", { score: item.score })}</span>
        <button type="button">
          {t("Review recommendation")}
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
