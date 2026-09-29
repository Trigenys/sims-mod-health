import { useEffect, useMemo, useState } from "react";
import { Topbar } from "../../components/layout/Topbar";
import { Panel } from "../../components/ui/Panel";
import { Button } from "../../components/ui/Button";
import { discoverGateway, type DiscoverGateway } from "./discover.gateway";
import type { DiscoveryBlocker, DiscoveryRecommendation, DiscoverySnapshot } from "./discover.types";
import { useI18n } from "../../i18n/i18n";

type DiscoverPageProps = {
  gateway?: DiscoverGateway;
  onOpenOverview?: () => void;
  onOpenLibrary?: () => void;
  onOpenSettings?: () => void;
};

export function DiscoverPage({
  gateway = discoverGateway,
  onOpenOverview,
  onOpenLibrary,
  onOpenSettings
}: DiscoverPageProps) {
  const { t, tx } = useI18n();
  const [snapshot, setSnapshot] = useState<DiscoverySnapshot | null>(null);
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("All");
  const [retrying, setRetrying] = useState(false);

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

  const retry = async () => {
    setRetrying(true);
    try {
      setSnapshot(await gateway.load());
    } finally {
      setRetrying(false);
    }
  };

  const blockerAction = blockerActionFor(
    snapshot.blocker,
    {
      onOpenOverview,
      onOpenLibrary,
      onOpenSettings,
      onRetry: () => void retry()
    },
    t
  );

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

      <DiscoverReadiness snapshot={snapshot} />

      {snapshot.state !== "ready" && (
        <section className="discover-state" role="status">
          <span aria-hidden="true">!</span>
          <div>
            <strong>{blockerTitle(snapshot.blocker, t)}</strong>
            <p>{blockerDetail(snapshot.blocker, snapshot, t)}</p>
          </div>
          {blockerAction && (
            <Button
              variant="secondary"
              onClick={blockerAction.run}
              disabled={retrying}
            >
              {retrying && snapshot.blocker === "registry"
                ? t("Checking again…")
                : blockerAction.label}
            </Button>
          )}
        </section>
      )}

      {visible.length === 0 ? (
        <Panel className="discover-empty">
          <div className="discover-empty__icon" aria-hidden="true">✦</div>
          <div>
            <h2>
              {snapshot.recommendations.length === 0
                ? snapshot.state === "ready"
                  ? t("Your setup is ready, but there is nothing safe to suggest right now")
                  : t("Recommendations will appear when the missing steps are complete")
                : t("No recommendation matches these filters")}
            </h2>
            <p>
              {snapshot.state === "ready"
                ? t("Every available candidate was filtered out because it is already installed, incompatible, conflicting or not verified for your current game version.")
                : t("Complete the readiness steps above and Discover will try again without inventing suggestions.")}
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

function DiscoverReadiness({ snapshot }: { snapshot: DiscoverySnapshot }) {
  const { t } = useI18n();
  const prerequisites = snapshot.prerequisites;

  return (
    <Panel className="discover-readiness" as="section">
      <div className="discover-readiness__header">
        <div>
          <span className="section-kicker">{t("RECOMMENDATION READINESS")}</span>
          <h2>
            {snapshot.state === "ready"
              ? t("Everything needed for recommendations is ready")
              : t("Finish these checks to unlock recommendations")}
          </h2>
        </div>
        <span className={snapshot.state === "ready" ? "discover-readiness__state discover-readiness__state--ready" : "discover-readiness__state"}>
          {snapshot.state === "ready" ? t("Ready") : t("Needs setup")}
        </span>
      </div>

      <div className="discover-prerequisite-grid">
        <Prerequisite
          ready={prerequisites.gameDetected}
          label={t("Game")}
          detail={prerequisites.gameDetected ? t("Game found") : t("Game not detected")}
        />
        <Prerequisite
          ready={prerequisites.patchKnown}
          label={t("Game version")}
          detail={
            prerequisites.patchKnown && snapshot.patchVersion
              ? t("Patch {{version}}", { version: snapshot.patchVersion })
              : t("Version not detected")
          }
        />
        <Prerequisite
          ready={prerequisites.packsKnown}
          label={t("Packs")}
          detail={
            !prerequisites.packsKnown
              ? t("Packs not checked")
              : prerequisites.installedPackCount === 0
                ? t("Pack check complete · base game only")
                : t("{{count}} installed packs found", { count: prerequisites.installedPackCount ?? 0 })
          }
        />
        <Prerequisite
          ready={(prerequisites.identifiedMods ?? 0) > 0}
          label={t("Installed mods")}
          detail={
            !prerequisites.modsScanned
              ? t("Mods not scanned")
              : prerequisites.identifiedMods === null
                ? t("Mod identification not checked yet")
                : prerequisites.identifiedMods > 0
                  ? t("{{count}} installed mods identified", { count: prerequisites.identifiedMods })
                  : prerequisites.installedModFiles === 0
                    ? t("No enabled mod files found")
                    : t("No installed mod identified yet")
          }
          secondary={
            prerequisites.installedModFiles === null
              ? null
              : t("{{count}} mod files scanned", { count: prerequisites.installedModFiles })
          }
        />
        <Prerequisite
          ready={prerequisites.registryAvailable === true}
          label={t("Online recommendation data")}
          detail={
            prerequisites.registryAvailable === true
              ? t("Available")
              : prerequisites.registryAvailable === false
                ? t("Unavailable")
                : t("Not checked yet")
          }
        />
      </div>
    </Panel>
  );
}

function Prerequisite({
  ready,
  label,
  detail,
  secondary
}: {
  ready: boolean;
  label: string;
  detail: string;
  secondary?: string | null;
}) {
  return (
    <div className={ready ? "discover-prerequisite discover-prerequisite--ready" : "discover-prerequisite"}>
      <span className="discover-prerequisite__icon" aria-hidden="true">{ready ? "✓" : "!"}</span>
      <div>
        <strong>{label}</strong>
        <span>{detail}</span>
        {secondary && <small>{secondary}</small>}
      </div>
    </div>
  );
}

function blockerTitle(
  blocker: DiscoveryBlocker | null,
  t: ReturnType<typeof useI18n>["t"]
) {
  if (blocker === "game") return t("Connect your game before we recommend mods");
  if (blocker === "patch") return t("We need the game version first");
  if (blocker === "packs") return t("We need to check your installed packs");
  if (blocker === "mods_scan") return t("Scan your Mods folder first");
  if (blocker === "identified_mods") return t("We need at least one identified installed mod");
  if (blocker === "registry") return t("Online recommendation data is unavailable");
  return t("Discover could not finish checking your setup");
}

function blockerDetail(
  blocker: DiscoveryBlocker | null,
  snapshot: DiscoverySnapshot,
  t: ReturnType<typeof useI18n>["t"]
) {
  if (blocker === "game") {
    return t("Your Mods can still be scanned, but recommendations need the installed game so we can check compatibility.");
  }
  if (blocker === "patch") {
    return t("We found the game, but not its version. Recommendations stay locked until the version is known.");
  }
  if (blocker === "packs") {
    return t("We need the pack inventory so we do not recommend a mod that depends on content you do not have.");
  }
  if (blocker === "mods_scan") {
    return t("Discover uses your installed mods to understand what you already use and avoid suggesting duplicates.");
  }
  if (blocker === "identified_mods") {
    return snapshot.prerequisites.installedModFiles
      ? t("{{count}} mod files were scanned, but none are identified well enough to seed recommendations yet.", {
          count: snapshot.prerequisites.installedModFiles
        })
      : t("We need to identify at least one installed mod before we can suggest similar compatible options.");
  }
  if (blocker === "registry") {
    return t("Your local library still works. Retry when online compatibility data is available again.");
  }
  return t("Your local data is safe. Try the check again; if it keeps failing, review the setup on Overview.");
}

function blockerActionFor(
  blocker: DiscoveryBlocker | null,
  actions: {
    onOpenOverview?: () => void;
    onOpenLibrary?: () => void;
    onOpenSettings?: () => void;
    onRetry: () => void;
  },
  t: ReturnType<typeof useI18n>["t"]
): { label: string; run: () => void } | null {
  if (blocker === "game" || blocker === "patch" || blocker === "packs") {
    return actions.onOpenSettings
      ? { label: t("Review game folders"), run: actions.onOpenSettings }
      : null;
  }
  if (blocker === "mods_scan") {
    return actions.onOpenOverview
      ? { label: t("Go to Overview"), run: actions.onOpenOverview }
      : null;
  }
  if (blocker === "identified_mods") {
    return actions.onOpenLibrary
      ? { label: t("Review Library"), run: actions.onOpenLibrary }
      : null;
  }
  if (blocker === "registry" || blocker === "local_state") {
    return { label: t("Try again"), run: actions.onRetry };
  }
  return null;
}

function recommendationReason(
  item: DiscoveryRecommendation,
  t: ReturnType<typeof useI18n>["t"],
  tx: ReturnType<typeof useI18n>["tx"]
) {
  const features = item.reason.sharedFeatures.map((value) => tx(humanize(value))).join(", ");
  const categories = item.reason.sharedCategories.map((value) => tx(humanize(value))).join(", ");

  if (features && categories) {
    return t("Because you use {{anchor}}, this mod shares features like {{features}} and categories like {{categories}}.", {
      anchor: item.reason.becauseModName,
      features,
      categories
    });
  }
  if (features) {
    return t("Because you use {{anchor}}, this mod shares features like {{features}}.", {
      anchor: item.reason.becauseModName,
      features
    });
  }
  if (categories) {
    return t("Because you use {{anchor}}, this mod is in similar categories: {{categories}}.", {
      anchor: item.reason.becauseModName,
      categories
    });
  }
  return t("This recommendation is similar to {{anchor}} based on the information available.", {
    anchor: item.reason.becauseModName
  });
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

      <p className="recommendation-reason">{recommendationReason(item, t, tx)}</p>

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
