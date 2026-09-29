import { Topbar } from "../../components/layout/Topbar";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { StatusBadge } from "../../components/ui/StatusBadge";
import type { LibraryItem } from "./library.fixture";
import { useI18n } from "../../i18n/i18n";
import { localizeLibraryGeneratedCopy } from "./library.i18n";

type ModDetailPageProps = {
  item: LibraryItem;
  onBack: () => void;
};

const evidenceLabel = {
  fact: "Confirmed",
  inference: "Best match",
  community: "Community report"
} as const;

export function ModDetailPage({ item, onBack }: ModDetailPageProps) {
  const { t, tx } = useI18n();
  const identityLabel =
    item.confidence === "exact"
      ? t("Exact fingerprint")
      : item.confidence === "unresolved"
        ? t("Not identified yet")
        : tx(item.confidence.charAt(0).toUpperCase() + item.confidence.slice(1) + " confidence");

  return (
    <>
      <Topbar
        gameVersion={item.gameVersion ? t("Patch {{version}}", { version: item.gameVersion }) : t("Game version not detected yet")}
        platform="Windows"
        indexedCount={item.libraryCount ?? 0}
      />

      <button className="detail-back" onClick={onBack}>
        <span aria-hidden="true">←</span>
        {t("Back to Library")}
      </button>

      <section className="detail-hero" aria-labelledby="mod-detail-title">
        <div className="detail-hero__identity">
          <span className="detail-monogram" aria-hidden="true">
            {item.canonicalName.slice(0, 2).toUpperCase()}
          </span>
          <div>
            <p className="eyebrow">{item.identified ? t("IDENTIFIED MOD") : t("LOCAL FILE")}</p>
            <h1 id="mod-detail-title">{item.canonicalName}</h1>
            <p>
              {tx(item.creator)} <span>·</span> {tx(item.category)}
            </p>
          </div>
        </div>

        <StatusBadge tone={item.tone}>{tx(item.status)}</StatusBadge>
      </section>

      <section className="detail-facts" aria-label={t("Version and identity facts")}>
        <Fact label={t("Installed")} value={item.installedVersion ?? t("Unknown")} />
        <Fact label={t("Latest")} value={item.latestVersion ?? t("Unknown")} />
        <Fact label={t("Patch")} value={item.gameVersion ?? t("Unknown")} />
        <Fact label={t("Identity")} value={identityLabel} />
      </section>

      <div className="detail-actions">
        {item.latestVersion && item.latestVersion !== item.installedVersion && (
          <Button variant="primary">{t("Review update")}</Button>
        )}
        {item.source !== "Local only" && <Button>{item.enabled ? t("Disable") : t("Enable")}</Button>}
        {item.source !== "Local only" && <Button>{t("Creator page")}</Button>}
      </div>

      <section className="detail-layout">
        <div className="detail-main">
          <Panel className="detail-section">
            <span className="section-kicker">{t("What it does")}</span>
            <h2>{t("Purpose and scope")}</h2>
            <p className="detail-description">{localizeLibraryGeneratedCopy(item.whatItDoes, t)}</p>
          </Panel>

          <Panel className="detail-section">
            <div className="panel-header">
              <div>
                <span className="section-kicker">{t("How we know")}</span>
                <h2>{t("Why this status appears")}</h2>
              </div>
              <span className="evidence-count">{t("{{count}} signals", { count: item.evidence.length })}</span>
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
                    {tx(evidenceLabel[evidence.type])}
                  </div>
                  <strong>{tx(evidence.title)}</strong>
                  <p>{localizeLibraryGeneratedCopy(evidence.detail, t)}</p>
                  {evidence.source && <small>{t("Source: {{source}}", { source: evidence.source })}</small>}
                </article>
              ))}
            </div>
          </Panel>

          <Panel className="detail-section">
            <span className="section-kicker">{t("Files on this computer")}</span>
            <h2>{t("Installed files")}</h2>
            <ul className="file-list">
              {item.localFiles.map((file) => (
                <li key={file}>
                  <span aria-hidden="true">⌘</span>
                  <code>{file}</code>
                  <span>{item.enabled ? t("Enabled") : t("Disabled")}</span>
                </li>
              ))}
            </ul>
          </Panel>
        </div>

        <aside className="detail-side">
          <Panel className="detail-section">
            <span className="section-kicker">{t("Required mods")}</span>
            <h2>
              {item.dependencies.length === 0
                ? t("No extra mods required")
                : t("{{count}} required mods", { count: item.dependencies.length })}
            </h2>
            {item.dependencies.length > 0 && (
              <ul className="dependency-list">
                {item.dependencies.map((dependency) => (
                  <li key={dependency.name}>
                    <span>
                      <strong>{dependency.name}</strong>
                      <small>{tx(dependency.state)}</small>
                    </span>
                    <span aria-hidden="true">›</span>
                  </li>
                ))}
              </ul>
            )}
          </Panel>

          <Panel className="detail-section">
            <span className="section-kicker">{t("About this mod")}</span>
            <h2>{tx(item.source)}</h2>
            <dl className="detail-source">
              <div><dt>{t("Creator")}</dt><dd>{tx(item.creator)}</dd></div>
              <div><dt>{t("Category")}</dt><dd>{tx(item.category)}</dd></div>
              <div><dt>{t("File type")}</dt><dd>{tx(item.kind)}</dd></div>
              <div><dt>{t("State")}</dt><dd>{item.enabled ? t("Enabled") : t("Disabled")}</dd></div>
            </dl>
          </Panel>

          <Panel className="detail-section">
            <span className="section-kicker">{t("Related mods")}</span>
            <h2>{t("Related items in your library")}</h2>
            {item.relatedMods.length === 0 ? (
              <p className="detail-muted">{t("No related mods found yet.")}</p>
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
