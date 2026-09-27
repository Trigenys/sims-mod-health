import { invoke } from "@tauri-apps/api/core";
import { diagnosticsVisualFixture } from "./diagnostics.visual";
import type {
  DiagnosticsSnapshot,
  PrivacyPreferences
} from "./diagnostics.types";

export type DiagnosticsGateway = {
  analyze: () => Promise<DiagnosticsSnapshot>;
  getPrivacyPreferences: () => Promise<PrivacyPreferences>;
  setDiagnosticTelemetryConsent: (
    enabled: boolean
  ) => Promise<PrivacyPreferences>;
};

function isVisualHarness() {
  return new URLSearchParams(window.location.search).get("visual") === "diagnostics";
}

export const diagnosticsGateway: DiagnosticsGateway = {
  async analyze() {
    if (isVisualHarness()) {
      return diagnosticsVisualFixture;
    }

    try {
      return await invoke<DiagnosticsSnapshot>("analyze_latest_diagnostics");
    } catch (error) {
      return {
        installationId: null,
        registryState: "offline",
        registryDetail:
          error instanceof Error
            ? error.message
            : "Diagnostics are available in the desktop application.",
        reports: []
      };
    }
  },

  async getPrivacyPreferences() {
    if (isVisualHarness()) {
      return { diagnosticTelemetryEnabled: false };
    }

    try {
      return await invoke<PrivacyPreferences>("get_privacy_preferences");
    } catch {
      return { diagnosticTelemetryEnabled: false };
    }
  },

  async setDiagnosticTelemetryConsent(enabled) {
    if (isVisualHarness()) {
      return { diagnosticTelemetryEnabled: enabled };
    }

    return invoke<PrivacyPreferences>("set_diagnostic_telemetry_consent", {
      enabled
    });
  }
};
