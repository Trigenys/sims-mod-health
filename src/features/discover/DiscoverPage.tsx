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
        gameVersion={snapshot.patchVersion ? t("Patch {{version}}", { version: snapshot.patchVersion }) : t("Game version not detected yet")}
        platform="Windows"
        indexedCount={snapshot.recommendations.length}
      />

      <section className="discover-hero" aria-labelledby="discover-title">
        <div>
          <p className="eyebrow">{t("RECOMMENDATIONS")}</p>
          <h1 id="discover-title">{t("Mods you can consider")}</h1>
          <p className="lede">
            {t("We only show suggestions that fit your current game version and do not conflict with what we already know.")}
          </p>
        </div>
        <div className="discover-count">
          <span>{t("Suggestions")}</span>
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
                ? t("Recommendations are temporarily unavailable")
                : snapshot.state === "empty"
                  ? t("We need a little more information before recommending mods")
                  : t("Some recommendation checks are unavailable")}
            </strong>
            <p>
              {snapshot.state === "offline"
                ? t("Your local library is still available. We need online compatibility data before we can recommend mods safely.")
                : snapshot.state === "empty"
                  ? t("Finish scanning your game and mods so we can build recommendations for your setup.")
                  : t("Some compatibility checks could not be completed, so recommendations may be limited for now.")}
            </p>
          </div>
        </section>
      )}

      {visible.length === 0 ? (
        <Panel className="discover-empty">
          <div className="discover-empty__icon" aria-hidden="true">✦</div>
          <div>
            <h2>
              {snapshot.recommendations.length === 0
                ? t("No recommendations yet")
                : t("No recommendation matches these filters")}
            </h2>
            <p>
              {t("We only recommend mods when we have enough information about your game and installed mods.")}
            </p>
          </div>
        </Panel>
      ) : (
        <section className="discover-grid" aria-label={t("Recommendations")}>
          {visible.map((item) => (
            <RecommendationCard item={item} key={item.releaseId} />
          ))}
        </section>
      )}

      <Panel className="discover-principle">
        <div className="discover-principle__icon" aria-hidden="true">♡</div>
        <div>
          <span className="section-kicker">{t("HOW RECOMMENDATIONS WORK")}</span>
          <h2>{t("We will not recommend something that may break your setup.")}</h2>
          <p>
            {t("We leave out mods you already have and anything we cannot verify as compatible with your setup.")}
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
        <span>{t("Match score {{score}}", { score: item.score })}</span>
        <button type="button">
          {t("See why this was suggested")}
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
