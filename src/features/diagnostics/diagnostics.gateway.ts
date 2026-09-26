import { invoke } from "@tauri-apps/api/core";
import { diagnosticsVisualFixture } from "./diagnostics.visual";
import type { DiagnosticsSnapshot } from "./diagnostics.types";

export type DiagnosticsGateway = {
  analyze: () => Promise<DiagnosticsSnapshot>;
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
  }
};
