import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DiagnosticsPage } from "./DiagnosticsPage";
import type { DiagnosticsGateway } from "./diagnostics.gateway";
import type { DiagnosticsSnapshot } from "./diagnostics.types";

const snapshot: DiagnosticsSnapshot = {
  installationId: 1,
  registryState: "ready",
  registryDetail: "resolved",
  reports: [
    {
      diagnosticId: 1,
      sourceKind: "last_exception",
      reportName: "lastException.txt",
      parseStatus: "parsed",
      observedAt: "unix-ms:1",
      contentHash: "hash",
      observations: [
        {
          kind: "module",
          value: "creator.cool_mod",
          context: "module: creator.cool_mod"
        }
      ],
      candidates: [
        {
          localFileId: 7,
          relativePath: "Creator/CoolMod.ts4script",
          confidence: "high",
          confidenceScore: 90,
          relationship: "implicated_by_diagnostic_evidence",
          evidence: [
            {
              kind: "module",
              reference: "creator.cool_mod",
              explanation:
                "Diagnostic module exactly matches a module exposed by this installed TS4Script archive."
            }
          ],
          canonical: {
            modId: "mod",
            releaseId: "release",
            confidence: "1.0",
            deterministic: true
          }
        }
      ],
      note: "Candidates are correlated with the report evidence; this is not a causality claim.",
      telemetryPreview: {
        sourceKind: "last_exception",
        observationKinds: ["module"],
        candidateCount: 1,
        redactionsApplied: 2
      }
    }
  ]
};

function gateway(
  value: DiagnosticsSnapshot,
  telemetryEnabled = false
): DiagnosticsGateway {
  return {
    analyze: vi.fn().mockResolvedValue(value),
    getPrivacyPreferences: vi.fn().mockResolvedValue({
      diagnosticTelemetryEnabled: telemetryEnabled
    }),
    setDiagnosticTelemetryConsent: vi.fn().mockImplementation(
      async (enabled: boolean) => ({
        diagnosticTelemetryEnabled: enabled
      })
    )
  };
}

describe("DiagnosticsPage", () => {
  it("renders evidence-linked candidates without asserting causality", async () => {
    render(<DiagnosticsPage gateway={gateway(snapshot)} />);

    expect(await screen.findByText("Creator/CoolMod.ts4script")).toBeVisible();
    expect(screen.getByText(/Correlated with report evidence/)).toBeVisible();
    expect(screen.getByText("Canonical artifact resolved")).toBeVisible();
    expect(document.body.textContent?.toLowerCase()).not.toContain("caused by");
  });

  it("keeps local evidence visible when registry resolution is offline", async () => {
    const offline: DiagnosticsSnapshot = {
      ...snapshot,
      registryState: "offline",
      registryDetail: "registry transport error"
    };

    render(<DiagnosticsPage gateway={gateway(offline)} />);

    expect(await screen.findByText("Registry offline")).toBeVisible();
    expect(screen.getByText("Creator/CoolMod.ts4script")).toBeVisible();
  });

  it("refreshes analysis on demand", async () => {
    const fakeGateway = gateway(snapshot);
    render(<DiagnosticsPage gateway={fakeGateway} />);

    await screen.findByText("Creator/CoolMod.ts4script");
    fireEvent.click(screen.getByRole("button", { name: "Analyze reports" }));

    await waitFor(() => expect(fakeGateway.analyze).toHaveBeenCalledTimes(2));
  });

  it("renders malformed reports as evidence states instead of crashing", async () => {
    const malformed: DiagnosticsSnapshot = {
      installationId: 1,
      registryState: "ready",
      registryDetail: "local",
      reports: [
        {
          diagnosticId: 2,
          sourceKind: "last_ui_exception",
          reportName: "lastUIException.txt",
          parseStatus: "malformed",
          observedAt: "unix-ms:2",
          contentHash: null,
          observations: [],
          candidates: [],
          note: "Report is empty.",
          telemetryPreview: {
            sourceKind: "last_ui_exception",
            observationKinds: [],
            candidateCount: 0,
            redactionsApplied: 0
          }
        }
      ]
    };

    render(<DiagnosticsPage gateway={gateway(malformed)} />);

    expect(await screen.findByText("Malformed")).toBeVisible();
    expect(screen.getByText("No installed mod candidate linked")).toBeVisible();
  });

  it("keeps diagnostic telemetry off by default and requires explicit consent", async () => {
    const fakeGateway = gateway(snapshot, false);
    render(<DiagnosticsPage gateway={fakeGateway} />);

    expect(await screen.findByText("Diagnostic telemetry is off")).toBeVisible();
    const checkbox = screen.getByRole("checkbox", {
      name: "Allow redacted diagnostic telemetry"
    });
    expect(checkbox).not.toBeChecked();
    await waitFor(() => expect(checkbox).not.toBeDisabled());

    fireEvent.click(checkbox);

    await waitFor(() =>
      expect(fakeGateway.setDiagnosticTelemetryConsent).toHaveBeenCalledWith(true)
    );
    expect(await screen.findByText("Diagnostic telemetry is on")).toBeVisible();
  });

  it("fails closed when privacy settings cannot be loaded", async () => {
    const fakeGateway = gateway(snapshot, true);
    fakeGateway.getPrivacyPreferences = vi
      .fn()
      .mockRejectedValue(new Error("storage unavailable"));

    render(<DiagnosticsPage gateway={fakeGateway} />);

    expect(
      await screen.findByText(
        "Privacy settings could not be loaded. Diagnostic telemetry remains off."
      )
    ).toBeVisible();
    expect(
      screen.getByRole("checkbox", {
        name: "Allow redacted diagnostic telemetry"
      })
    ).not.toBeChecked();
  });
});
