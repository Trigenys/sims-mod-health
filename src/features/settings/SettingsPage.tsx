import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Topbar } from "../../components/layout/Topbar";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { diagnosticsGateway } from "../diagnostics/diagnostics.gateway";
import { gameContentGateway } from "../game-content/gameContent.gateway";
import {
  formatProvider,
  providerCapabilityDetail
} from "../game-content/gameContent.presenter";
import type {
  GameContentSnapshot,
  ProviderUpdateCapability
} from "../game-content/gameContent.types";
import { useI18n } from "../../i18n/i18n";

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

const sections: { value: SettingsSection; label: string; icon: string }[] = [
  { value: "paths", label: "Game & folders", icon: "⌂" },
  { value: "privacy", label: "Checks & Privacy", icon: "✓" },
  { value: "scan", label: "Scan Behavior", icon: "↻" },
  { value: "recovery", label: "Recovery & Data", icon: "◫" },
  { value: "appearance", label: "Appearance", icon: "◐" }
];

export function SettingsPage() {
  const { t, tx } = useI18n();
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
        gameVersion={versionLabel(activeInstallation, t)}
        platform="Windows"
        indexedCount={0}
      />

      <section className="settings-hero" aria-labelledby="settings-title">
        <div>
          <p className="eyebrow">{t("APP SETTINGS")}</p>
          <h1 id="settings-title">{t("Settings")}</h1>
          <p className="lede">
            {t("Choose where your Sims files are, how scans behave and what the app may send.")}
          </p>
        </div>
        <div className="settings-engine-state">
          <span className="status-dot" aria-hidden="true" />
          <div>
            <span>{t("How the app works")}</span>
            <strong>{t("Files stay on this computer")}</strong>
          </div>
        </div>
      </section>

      <div className="settings-layout">
        <nav className="settings-nav" aria-label={t("Settings sections")}>
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
              {t(item.label as "Game & folders" | "Checks & Privacy" | "Scan Behavior" | "Recovery & Data" | "Appearance")}
            </button>
          ))}

          <div className="settings-nav__note">
            <span className="section-kicker">{t("WHEN ONLINE CHECKS ARE DOWN")}</span>
            <p>
              {t("Your local library, duplicate checks and reports still work even when online details are unavailable.")}
            </p>
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
              error={privacyError ? tx(privacyError) : null}
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
  const { t, tx } = useI18n();
  const programInstallation = gameInventory.installations[0];

  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">{t("YOUR SIMS 4 FOLDERS")}</span>
          <h2>{t("Game and Mods locations")}</h2>
          <p>
            {t("These folders are detected on your computer. Your raw mod files are not uploaded.")}
          </p>
        </div>
        <Button variant="secondary" onClick={() => void onDetect()} disabled={detecting}>
          {detecting ? t("Detecting…") : t("Rescan locations")}
        </Button>
      </div>

      <Panel className="settings-card settings-card--featured">
        <div className="settings-card__header">
          <div>
            <span className="section-kicker">{t("CURRENT SIMS 4 FOLDER")}</span>
            <h3>{active ? tx(sourceLabel(active.source)) : t("No installation detected")}</h3>
          </div>
          <span className={active?.modsAvailable ? "settings-ok" : "settings-muted"}>
            {active?.modsAvailable ? t("Mods folder available") : t("Not available")}
          </span>
        </div>

        <PathRow
          label={t("Sims 4 user folder")}
          value={active?.root ?? t("Run detection in the desktop application")}
        />
        <PathRow
          label={t("Mods folder")}
          value={active?.modsRoot ?? t("No Mods directory detected")}
        />
        <PathRow
          label={t("Game patch")}
          value={
            programInstallation?.build.version?.normalized
              ? t("Patch {{version}}", { version: programInstallation.build.version.normalized })
              : versionLabel(active, t)
          }
        />
        <PathRow
          label={t("Game program folder")}
          value={programInstallation?.installRoot ?? t("No program installation detected")}
        />
        <PathRow
          label={t("Update provider")}
          value={tx(formatProvider(programInstallation?.provider ?? providerCapability?.provider ?? "unknown"))}
        />
        <PathRow
          label={t("Installed packs")}
          value={String(programInstallation?.packs.length ?? 0)}
        />
      </Panel>

      {installations.length > 1 && (
        <Panel className="settings-card">
          <span className="section-kicker">{t("OTHER SIMS 4 FOLDERS FOUND")}</span>
          <div className="settings-installations">
            {installations.slice(1).map((item) => (
              <div key={item.root}>
                <strong>{tx(sourceLabel(item.source))}</strong>
                <code>{item.root}</code>
              </div>
            ))}
          </div>
        </Panel>
      )}

      <Panel className="settings-card">
        <span className="section-kicker">{t("GAME UPDATES")}</span>
        <h3>{t("Game updates stay with EA app or Steam")}</h3>
        <p>
          {providerCapability
            ? providerCapabilityDetail(providerCapability.provider, providerCapability.supported, t)
            : t("Provider capability is resolved locally when a game installation is available.")}
          {" "}{t("After you update the game, Sims Mod Health checks the local version and packs again before marking the update complete.")}
        </p>
      </Panel>

      <Panel className="settings-card">
        <span className="section-kicker">{t("ONLINE DETAILS")}</span>
        <h3>{t("Your local scan still works offline")}</h3>
        <p>
          {t("Online data adds mod names, compatibility, relationships and recommendations. If it is unavailable, your local library, duplicate checks and reports remain visible.")}
        </p>
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
          <span className="section-kicker">{t("PRIVACY")}</span>
          <h2>{t("Checks & privacy")}</h2>
          <p>
            {t("Diagnostic telemetry is explicit opt-in. Raw diagnostic reports and raw mod files are not uploaded automatically.")}
          </p>
        </div>
      </div>

      <Panel className="settings-card settings-toggle-card">
        <div>
          <h3>{t("Share anonymous diagnostic summaries")}</h3>
          <p>
            {t("If you turn this on, only a cleaned-up summary can be sent. Raw reports and mod files stay on your computer.")}
          </p>
          {error && <small className="settings-error">{error}</small>}
        </div>
        <label className="settings-switch">
          <input
            type="checkbox"
            checked={enabled}
            disabled={busy}
            onChange={(event) => void onChange(event.target.checked)}
          />
          <span>{enabled ? t("On") : t("Off")}</span>
        </label>
      </Panel>

      <div className="settings-grid">
        <PolicyCard
          title={t("Checked on this computer")}
          detail={t("Packages, script mods and supported reports are read on this computer.")}
        />
        <PolicyCard
          title={t("Online identification")}
          detail={t("We use file fingerprints and basic metadata to identify mods without uploading the raw files.")}
        />
        <PolicyCard
          title={t("No guessing")}
          detail={t("Possible problems stay labelled as possible until we can confirm them.")}
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
          <span className="section-kicker">{t("SCANNER")}</span>
          <h2>{t("Scan behavior")}</h2>
          <p>
            {t("After the first scan, unchanged files can be skipped so later scans finish faster.")}
          </p>
        </div>
      </div>
      <div className="settings-grid">
        <PolicyCard
          title={t("Faster repeat scans")}
          detail={t("Files that have not changed reuse the previous result. Changed files are checked again.")}
        />
        <PolicyCard
          title={t("Safe file reading")}
          detail={t("The scanner uses limits when reading packages, scripts and reports so a bad file cannot make it run forever.")}
        />
        <PolicyCard
          title={t("No automatic destructive cleanup")}
          detail={t("The app never deletes an unknown file just because it could not identify it.")}
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
          <span className="section-kicker">{t("RECOVERY & STORAGE")}</span>
          <h2>{t("Safe mutation policy")}</h2>
          <p>
            {t("Supported updates use app-controlled staging and verified restore points.")}
          </p>
        </div>
      </div>
      <div className="settings-grid">
        <PolicyCard
          title={t("Restore points")}
          detail={t("The original artifact is verified before the Mods folder is mutated.")}
        />
        <PolicyCard
          title={t("Persistent journal")}
          detail={t("Interrupted update transactions remain recoverable after restart.")}
        />
        <PolicyCard
          title={t("Rollback guard")}
          detail={t("Rollback refuses to overwrite a target that changed independently.")}
        />
      </div>
    </>
  );
}

function AppearanceSettings() {
  const { t } = useI18n();
  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">{t("APPEARANCE")}</span>
          <h2>{t("Approved light product theme")}</h2>
          <p>
            {t("The current beta uses the approved light Sims Mod Health design system for consistent evidence scanning and accessibility.")}
          </p>
        </div>
      </div>
      <Panel className="settings-card appearance-preview">
        <div className="appearance-swatch appearance-swatch--primary" />
        <div className="appearance-swatch appearance-swatch--mint" />
        <div className="appearance-swatch appearance-swatch--amber" />
        <div className="appearance-swatch appearance-swatch--indigo" />
        <div>
          <strong>{t("Light / Porcelain")}</strong>
          <span>{t("Current product theme")}</span>
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

function versionLabel(
  candidate: InstallationCandidate | undefined,
  t: ReturnType<typeof useI18n>["t"]
) {
  if (!candidate) return t("Patch unknown");
  if (candidate.version.status === "available") {
    return t("Patch {{version}}", { version: candidate.version.version.normalized });
  }
  return t("Patch unknown");
}

function sourceLabel(source: string) {
  if (source === "knownDocuments") return "Documents";
  if (source === "oneDriveFallback") return "OneDrive Documents";
  if (source === "manual") return "Manual path";
  return source;
}
