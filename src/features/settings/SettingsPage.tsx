import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Topbar } from "../../components/layout/Topbar";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { useI18n } from "../../i18n/I18nProvider";
import { diagnosticsGateway } from "../diagnostics/diagnostics.gateway";
import { gameContentGateway } from "../game-content/gameContent.gateway";
import { formatProvider } from "../game-content/gameContent.presenter";
import type {
  GameContentSnapshot,
  ProviderUpdateCapability
} from "../game-content/gameContent.types";

type SettingsSection =
  | "paths"
  | "privacy"
  | "scan"
  | "recovery"
  | "appearance";

type InstallationCandidate = {
  root: string;
  modsRoot: string;
  source: "knownDocuments" | "oneDriveFallback" | "manual" | string;
  modsAvailable: boolean;
  version:
    | { status: "available"; version: { normalized: string } }
    | { status: "missing" }
    | { status: "invalid"; reason: string };
};

const sections: { value: SettingsSection; key: "settings.paths" | "settings.privacy" | "settings.scan" | "settings.recovery" | "settings.appearance"; icon: string }[] = [
  { value: "paths", key: "settings.paths", icon: "⌂" },
  { value: "privacy", key: "settings.privacy", icon: "✓" },
  { value: "scan", key: "settings.scan", icon: "↻" },
  { value: "recovery", key: "settings.recovery", icon: "◫" },
  { value: "appearance", key: "settings.appearance", icon: "◐" }
];

export function SettingsPage() {
  const { t } = useI18n();
  const [section, setSection] = useState<SettingsSection>("paths");
  const [installations, setInstallations] = useState<InstallationCandidate[]>([]);
  const [detecting, setDetecting] = useState(true);
  const [gameInventory, setGameInventory] = useState<GameContentSnapshot>({ installations: [] });
  const [providerCapability, setProviderCapability] = useState<ProviderUpdateCapability | null>(null);
  const [telemetryEnabled, setTelemetryEnabled] = useState(false);
  const [privacyBusy, setPrivacyBusy] = useState(true);
  const [privacyError, setPrivacyError] = useState<string | null>(null);

  const detect = async () => {
    setDetecting(true);
    try {
      const [values, content, capability] = await Promise.all([
        invoke<InstallationCandidate[]>("discover_sims_installations").catch(() => []),
        gameContentGateway.refreshInventory(),
        gameContentGateway.loadCapability()
      ]);
      setInstallations(values);
      setGameInventory(content);
      setProviderCapability(capability);
    } finally {
      setDetecting(false);
    }
  };

  useEffect(() => {
    void detect();

    diagnosticsGateway
      .getPrivacyPreferences()
      .then((preferences) => {
        setTelemetryEnabled(preferences.diagnosticTelemetryEnabled);
        setPrivacyError(null);
      })
      .catch(() => {
        setTelemetryEnabled(false);
        setPrivacyError("Privacy settings could not be loaded. Telemetry remains off.");
      })
      .finally(() => setPrivacyBusy(false));
  }, []);

  const activeInstallation = useMemo(
    () => installations.find((item) => item.modsAvailable) ?? installations[0],
    [installations]
  );

  const setTelemetry = async (enabled: boolean) => {
    setPrivacyBusy(true);
    setPrivacyError(null);
    try {
      const preferences =
        await diagnosticsGateway.setDiagnosticTelemetryConsent(enabled);
      setTelemetryEnabled(preferences.diagnosticTelemetryEnabled);
    } catch {
      setTelemetryEnabled(false);
      setPrivacyError("Consent could not be saved. Telemetry remains off.");
    } finally {
      setPrivacyBusy(false);
    }
  };

  return (
    <>
      <Topbar
        gameVersion={activeInstallation?.version.status === "available" ? "Patch " + activeInstallation.version.version.normalized : t("common.patchUnknown")}
        platform={t("common.windows")}
        indexedCount={0}
      />

      <section className="settings-hero" aria-labelledby="settings-title">
        <div>
          <p className="eyebrow">{t("settings.eyebrow")}</p>
          <h1 id="settings-title">{t("settings.title")}</h1>
          <p className="lede">{t("settings.lede")}</p>
        </div>
        <div className="settings-engine-state">
          <span className="status-dot" aria-hidden="true" />
          <div>
            <span>{t("settings.engineMode")}</span>
            <strong>{t("settings.localFirst")}</strong>
          </div>
        </div>
      </section>

      <div className="settings-layout">
        <nav className="settings-nav" aria-label={t("settings.sections")}>
          {sections.map((item) => (
            <button
              type="button"
              key={item.value}
              aria-current={section === item.value ? "page" : undefined}
              className={
                section === item.value
                  ? "settings-nav__item settings-nav__item--active"
                  : "settings-nav__item"
              }
              onClick={() => setSection(item.value)}
            >
              <span aria-hidden="true">{item.icon}</span>
              {t(item.key)}
            </button>
          ))}

          <div className="settings-nav__note">
            <span className="section-kicker">{t("settings.betaPolicy")}</span>
            <p>{t("settings.betaPolicyCopy")}</p>
          </div>
        </nav>

        <section className="settings-content">
          {section === "paths" && (
            <PathsSettings
              active={activeInstallation}
              installations={installations}
              detecting={detecting}
              onDetect={detect}
              gameInventory={gameInventory}
              providerCapability={providerCapability}
            />
          )}

          {section === "privacy" && (
            <PrivacySettings
              enabled={telemetryEnabled}
              busy={privacyBusy}
              error={privacyError}
              onChange={setTelemetry}
            />
          )}

          {section === "scan" && <ScanSettings />}
          {section === "recovery" && <RecoverySettings />}
          {section === "appearance" && <AppearanceSettings />}
        </section>
      </div>
    </>
  );
}

function PathsSettings({
  active,
  installations,
  detecting,
  onDetect,
  gameInventory,
  providerCapability
}: {
  active?: InstallationCandidate;
  installations: InstallationCandidate[];
  detecting: boolean;
  onDetect: () => Promise<void>;
  gameInventory: GameContentSnapshot;
  providerCapability: ProviderUpdateCapability | null;
}) {
  const { t } = useI18n();
  const programInstallation = gameInventory.installations[0];

  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">{t("settings.localInstallation")}</span>
          <h2>{t("settings.pathsTitle")}</h2>
          <p>{t("settings.pathsCopy")}</p>
        </div>
        <Button variant="secondary" onClick={() => void onDetect()} disabled={detecting}>
          {detecting ? t("settings.detecting") : t("settings.rescanLocations")}
        </Button>
      </div>

      <Panel className="settings-card settings-card--featured">
        <div className="settings-card__header">
          <div>
            <span className="section-kicker">{t("settings.activeRoot")}</span>
            <h3>{active ? sourceLabel(active.source) : t("settings.noInstall")}</h3>
          </div>
          <span className={active?.modsAvailable ? "settings-ok" : "settings-muted"}>
            {active?.modsAvailable ? t("settings.modsAvailable") : t("settings.notAvailable")}
          </span>
        </div>

        <PathRow
          label={t("settings.userFolder")}
          value={active?.root ?? t("settings.runDetection")}
        />
        <PathRow
          label={t("settings.modsFolder")}
          value={active?.modsRoot ?? t("settings.noMods")}
        />
        <PathRow
          label={t("settings.gamePatch")}
          value={
            programInstallation?.build.version?.normalized
              ? "Patch " + programInstallation.build.version.normalized
              : versionLabel(active)
          }
        />
        <PathRow
          label={t("settings.programFolder")}
          value={programInstallation?.installRoot ?? t("settings.noProgram")}
        />
        <PathRow
          label={t("settings.updateProvider")}
          value={formatProvider(programInstallation?.provider ?? providerCapability?.provider ?? "unknown")}
        />
        <PathRow
          label={t("settings.installedPacks")}
          value={String(programInstallation?.packs.length ?? 0)}
        />
      </Panel>

      {installations.length > 1 && (
        <Panel className="settings-card">
          <span className="section-kicker">{t("settings.otherRoots")}</span>
          <div className="settings-installations">
            {installations.slice(1).map((item) => (
              <div key={item.root}>
                <strong>{sourceLabel(item.source)}</strong>
                <code>{item.root}</code>
              </div>
            ))}
          </div>
        </Panel>
      )}

      <Panel className="settings-card">
        <span className="section-kicker">{t("settings.updateProvider").toUpperCase()}</span>
        <h3>{t("settings.providerTitle")}</h3>
        <p>
          {providerCapability?.detail
            ?? t("settings.providerFallback")}
          {" "}{t("settings.providerCopy")}
        </p>
      </Panel>

      <Panel className="settings-card">
        <span className="section-kicker">{t("settings.registryBehavior")}</span>
        <h3>{t("settings.registryTitle")}</h3>
        <p>{t("settings.registryCopy")}</p>
      </Panel>
    </>
  );
}

function PrivacySettings({
  enabled,
  busy,
  error,
  onChange
}: {
  enabled: boolean;
  busy: boolean;
  error: string | null;
  onChange: (enabled: boolean) => Promise<void>;
}) {
  const { t } = useI18n();
  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">PRIVACY</span>
          <h2>{t("settings.privacyTitle")}</h2>
          <p>{t("settings.privacyCopy")}</p>
        </div>
      </div>

      <Panel className="settings-card settings-toggle-card">
        <div>
          <h3>{t("settings.telemetryTitle")}</h3>
          <p>{t("settings.telemetryCopy")}</p>
          {error && <small className="settings-error">{error}</small>}
        </div>
        <label className="settings-switch">
          <input
            type="checkbox"
            checked={enabled}
            disabled={busy}
            onChange={(event) => void onChange(event.target.checked)}
          />
          <span>{enabled ? t("settings.on") : t("settings.off")}</span>
        </label>
      </Panel>

      <div className="settings-grid">
        <PolicyCard
          title="Local parsing"
          detail="DBPF, TS4Script and supported diagnostic reports are parsed on-device with bounded readers."
        />
        <PolicyCard
          title="Registry requests"
          detail="Canonical identity uses constrained artifact metadata and supported fingerprints rather than raw file uploads."
        />
        <PolicyCard
          title="Evidence language"
          detail="Correlation and potential conflicts remain distinct from deterministic breakage."
        />
      </div>
    </>
  );
}

function ScanSettings() {
  const { t } = useI18n();
  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">SCANNER</span>
          <h2>{t("settings.scanTitle")}</h2>
          <p>{t("settings.scanCopy")}</p>
        </div>
      </div>
      <div className="settings-grid">
        <PolicyCard
          title="Incremental by default"
          detail="Known unchanged artifacts reuse persisted evidence; changed files are hashed and re-inspected."
        />
        <PolicyCard
          title="Bounded inspection"
          detail="Archive, package, script and diagnostic parsing use explicit limits rather than unbounded recursive work."
        />
        <PolicyCard
          title="No automatic destructive cleanup"
          detail="Health findings are reviewable evidence. Unknown files are not silently removed."
        />
      </div>
    </>
  );
}

function RecoverySettings() {
  const { t } = useI18n();
  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">{t("settings.recoveryStorage")}</span>
          <h2>{t("settings.recoveryTitle")}</h2>
          <p>{t("settings.recoveryCopy")}</p>
        </div>
      </div>
      <div className="settings-grid">
        <PolicyCard
          title="Restore points"
          detail="The original artifact is verified before the Mods folder is mutated."
        />
        <PolicyCard
          title="Persistent journal"
          detail="Interrupted update transactions remain recoverable after restart."
        />
        <PolicyCard
          title="Rollback guard"
          detail="Rollback refuses to overwrite a target that changed independently."
        />
      </div>
    </>
  );
}

function AppearanceSettings() {
  const { locale, setLocale, t } = useI18n();

  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">{t("settings.appearance")}</span>
          <h2>{t("settings.language")}</h2>
          <p>{t("settings.languageCopy")}</p>
        </div>
      </div>

      <Panel className="settings-card settings-toggle-card">
        <div>
          <h3>{t("settings.language")}</h3>
          <p>{t("settings.languageCopy")}</p>
        </div>
        <label className="language-select">
          <span className="sr-only">{t("settings.language")}</span>
          <select
            aria-label={t("settings.language")}
            value={locale}
            onChange={(event) => setLocale(event.target.value === "fr" ? "fr" : "en")}
          >
            <option value="fr">{t("settings.french")}</option>
            <option value="en">{t("settings.english")}</option>
          </select>
        </label>
      </Panel>

      <Panel className="settings-card appearance-preview">
        <div className="appearance-swatch appearance-swatch--primary" />
        <div className="appearance-swatch appearance-swatch--mint" />
        <div className="appearance-swatch appearance-swatch--amber" />
        <div className="appearance-swatch appearance-swatch--indigo" />
        <div>
          <strong>Light / Porcelain</strong>
          <span>{t("settings.theme")}</span>
        </div>
      </Panel>
    </>
  );
}

function PathRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="settings-path-row">
      <span>{label}</span>
      <code>{value}</code>
    </div>
  );
}

function PolicyCard({ title, detail }: { title: string; detail: string }) {
  return (
    <Panel className="settings-card">
      <div className="settings-policy-icon" aria-hidden="true">✓</div>
      <h3>{title}</h3>
      <p>{detail}</p>
    </Panel>
  );
}

function versionLabel(candidate?: InstallationCandidate) {
  if (!candidate) return "Patch unknown";
  if (candidate.version.status === "available") {
    return "Patch " + candidate.version.version.normalized;
  }
  return "Patch unknown";
}

function sourceLabel(source: string) {
  if (source === "knownDocuments") return "Documents";
  if (source === "oneDriveFallback") return "OneDrive Documents";
  if (source === "manual") return "Manual path";
  return source;
}
