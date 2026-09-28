import { Topbar } from "../../components/layout/Topbar";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { StatusBadge } from "../../components/ui/StatusBadge";
import { useI18n } from "../../i18n/I18nProvider";
import type { LibraryItem } from "./library.fixture";

type ModDetailPageProps = {
  item: LibraryItem;
  onBack: () => void;
};

const evidenceLabel = {
  fact: "Verified fact",
  inference: "Inferred identification",
  community: "Community report"
} as const;

export function ModDetailPage({ item, onBack }: ModDetailPageProps) {
  const { t } = useI18n();
  const identityLabel =
    item.confidence === "exact"
      ? "Exact fingerprint"
      : item.confidence === "unresolved"
        ? "Unresolved"
        : item.confidence.charAt(0).toUpperCase() + item.confidence.slice(1) + " confidence";

  return (
    <>
      <Topbar
        gameVersion={item.gameVersion ? "Patch " + item.gameVersion : t("common.patchUnknown")}
        platform={t("common.windows")}
        indexedCount={item.libraryCount ?? 0}
      />

      <button className="detail-back" onClick={onBack}>
        <span aria-hidden="true">←</span>
        {t("detail.back")}
      </button>

      <section className="detail-hero" aria-labelledby="mod-detail-title">
        <div className="detail-hero__identity">
          <span className="detail-monogram" aria-hidden="true">
            {item.canonicalName.slice(0, 2).toUpperCase()}
          </span>
          <div>
            <p className="eyebrow">{item.identified ? t("detail.resolvedMod") : t("detail.localFile")}</p>
            <h1 id="mod-detail-title">{item.canonicalName}</h1>
            <p>
              {item.creator} <span>·</span> {item.category}
            </p>
          </div>
        </div>

        <StatusBadge tone={item.tone}>{item.status}</StatusBadge>
      </section>

      <section className="detail-facts" aria-label="Version and identity facts">
        <Fact label={t("detail.installed")} value={item.installedVersion ?? t("common.unknown")} />
        <Fact label={t("detail.latest")} value={item.latestVersion ?? t("common.unknown")} />
        <Fact label={t("detail.patch")} value={item.gameVersion ?? t("common.unknown")} />
        <Fact label={t("detail.identity")} value={identityLabel} />
      </section>

      <div className="detail-actions">
        {item.latestVersion && item.latestVersion !== item.installedVersion && (
          <Button variant="primary">{t("detail.reviewUpdate")}</Button>
        )}
        {item.source !== "Local only" && <Button>{item.enabled ? t("detail.disable") : t("detail.enable")}</Button>}
        {item.source !== "Local only" && <Button>{t("detail.creatorPage")}</Button>}
      </div>

      <section className="detail-layout">
        <div className="detail-main">
          <Panel className="detail-section">
            <span className="section-kicker">{t("detail.whatItDoes")}</span>
            <h2>{t("detail.purpose")}</h2>
            <p className="detail-description">{item.whatItDoes}</p>
          </Panel>

          <Panel className="detail-section">
            <div className="panel-header">
              <div>
                <span className="section-kicker">{t("detail.evidence")}</span>
                <h2>{t("detail.why")}</h2>
              </div>
              <span className="evidence-count">{t("detail.signals", { count: item.evidence.length })}</span>
            </div>

            <div className="evidence-list">
              {item.evidence.map((evidence) => (
                <article
                  className={"evidence-card evidence-card--" + evidence.type}
                  key={evidence.type + "-" + evidence.title}
                >
                  <div className="evidence-card__label">
                    <span aria-hidden="true">
                      {evidence.type === "fact" ? "✓" : evidence.type === "inference" ? "≈" : "◌"}
                    </span>
                    {evidenceLabel[evidence.type]}
                  </div>
                  <strong>{evidence.title}</strong>
                  <p>{evidence.detail}</p>
                  {evidence.source && <small>{t("detail.source")}: {evidence.source}</small>}
                </article>
              ))}
            </div>
          </Panel>

          <Panel className="detail-section">
            <span className="section-kicker">{t("detail.localFiles")}</span>
            <h2>{t("detail.artifacts")}</h2>
            <ul className="file-list">
              {item.localFiles.map((file) => (
                <li key={file}>
                  <span aria-hidden="true">⌘</span>
                  <code>{file}</code>
                  <span>{item.enabled ? t("detail.enabled") : t("detail.disabled")}</span>
                </li>
              ))}
            </ul>
          </Panel>
        </div>

        <aside className="detail-side">
          <Panel className="detail-section">
            <span className="section-kicker">{t("detail.dependencies")}</span>
            <h2>
              {item.dependencies.length === 0
                ? t("detail.noDependencies")
                : t("detail.required", { count: item.dependencies.length })}
            </h2>
            {item.dependencies.length > 0 && (
              <ul className="dependency-list">
                {item.dependencies.map((dependency) => (
                  <li key={dependency.name}>
                    <span>
                      <strong>{dependency.name}</strong>
                      <small>{dependency.state}</small>
                    </span>
                    <span aria-hidden="true">›</span>
                  </li>
                ))}
              </ul>
            )}
          </Panel>

          <Panel className="detail-section">
            <span className="section-kicker">{t("detail.source")}</span>
            <h2>{item.source}</h2>
            <dl className="detail-source">
              <div><dt>{t("detail.creator")}</dt><dd>{item.creator}</dd></div>
              <div><dt>{t("detail.category")}</dt><dd>{item.category}</dd></div>
              <div><dt>{t("detail.fileType")}</dt><dd>{item.kind}</dd></div>
              <div><dt>{t("detail.state")}</dt><dd>{item.enabled ? t("detail.enabled") : t("detail.disabled")}</dd></div>
            </dl>
          </Panel>

          <Panel className="detail-section">
            <span className="section-kicker">{t("detail.related")}</span>
            <h2>{t("detail.nearby")}</h2>
            {item.relatedMods.length === 0 ? (
              <p className="detail-muted">{t("detail.noRelated")}</p>
            ) : (
              <div className="related-list">
                {item.relatedMods.map((name) => (
                  <button key={name}>
                    {name}
                    <span aria-hidden="true">›</span>
                  </button>
                ))}
              </div>
            )}
          </Panel>
        </aside>
      </section>
    </>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
