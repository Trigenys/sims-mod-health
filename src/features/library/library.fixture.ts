import type { StatusTone } from "../../components/ui/StatusBadge";

export type LibraryStatus =
  | "Compatible"
  | "Update available"
  | "Potential conflict"
  | "Broken"
  | "Unknown";

export type IdentificationConfidence = "exact" | "high" | "medium" | "unresolved";
export type LibraryKind = "Script mod" | "Package only";
export type LibrarySource = "CurseForge" | "GitHub" | "Creator page" | "Local only";

export type EvidenceItem = {
  type: "fact" | "inference" | "community";
  title: string;
  detail: string;
  source?: string;
};

export type LibraryItem = {
  id: string;
  canonicalName: string;
  filenameAliases: string[];
  creator: string;
  category: string;
  installedVersion: string | null;
  latestVersion: string | null;
  status: LibraryStatus;
  tone: StatusTone;
  source: LibrarySource;
  kind: LibraryKind;
  identified: boolean;
  confidence: IdentificationConfidence;
  enabled: boolean;
  dependencyCount: number;
  whatItDoes: string;
  dependencies: { name: string; state: "Installed" | "Missing" | "Outdated" }[];
  localFiles: string[];
  evidence: EvidenceItem[];
  relatedMods: string[];
  gameVersion?: string | null;
  libraryCount?: number;
  modsRoot?: string | null;
};

export const libraryFixture: LibraryItem[] = [
  {
    id: "mccc",
    canonicalName: "MC Command Center",
    filenameAliases: ["mc_cmd_center.package", "mc_cmd_center.ts4script", "mccc"],
    creator: "Deaderpool",
    category: "Gameplay / Story Progression",
    installedVersion: "2026.4.0",
    latestVersion: "2026.5.0",
    status: "Update available",
    tone: "update",
    source: "Creator page",
    kind: "Script mod",
    identified: true,
    confidence: "exact",
    enabled: true,
    dependencyCount: 0,
    whatItDoes:
      "Controls population, relationships, pregnancies, careers and story progression through a modular in-game command system.",
    dependencies: [],
    localFiles: ["mc_cmd_center.package", "mc_cmd_center.ts4script"],
    evidence: [
      {
        type: "fact",
        title: "Exact artifact identity",
        detail: "Installed files match the canonical release fingerprint."
      },
      {
        type: "fact",
        title: "Current patch evidence",
        detail: "The installed release has sourced compatibility evidence for Patch 1.128.90.",
        source: "Creator page"
      },
      {
        type: "inference",
        title: "Update candidate",
        detail: "A newer release exists, but update availability is kept separate from compatibility."
      }
    ],
    relatedMods: ["UI Cheats Extension", "Better Exceptions", "More Columns in CAS"]
  },
  {
    id: "rpo",
    canonicalName: "Relationship & Pregnancy Overhaul",
    filenameAliases: ["Lumpinou_RPO_Collection.package", "RPO_Core.ts4script", "rpo"],
    creator: "Lumpinou",
    category: "Relationships",
    installedVersion: "2.98",
    latestVersion: "2.98",
    status: "Unknown",
    tone: "muted",
    source: "Creator page",
    kind: "Script mod",
    identified: true,
    confidence: "high",
    enabled: true,
    dependencyCount: 1,
    whatItDoes:
      "Expands relationship, pregnancy, family and reproductive gameplay with modular systems and optional add-ons.",
    dependencies: [{ name: "Mood Pack Mod", state: "Installed" }],
    localFiles: ["Lumpinou_RPO_Collection.package", "RPO_Core.ts4script"],
    evidence: [
      {
        type: "inference",
        title: "Likely canonical identity",
        detail: "Creator alias, filename and script signature agree, but no exact source fingerprint is available."
      },
      {
        type: "fact",
        title: "Patch evidence missing",
        detail: "No current-patch compatibility report is available, so the state remains Unknown."
      },
      {
        type: "community",
        title: "Community report",
        detail: "One recent report mentions a save-specific issue. This is not treated as verified compatibility evidence.",
        source: "Community report"
      }
    ],
    relatedMods: ["Mood Pack Mod", "First Impressions", "Open Love Life"]
  },
  {
    id: "bbb",
    canonicalName: "Better BuildBuy",
    filenameAliases: ["Tmex-BetterBuildBuy.package", "BetterBuildBuy.ts4script"],
    creator: "TwistedMexi",
    category: "Build / Buy",
    installedVersion: "3.3.1",
    latestVersion: "3.3.1",
    status: "Compatible",
    tone: "healthy",
    source: "CurseForge",
    kind: "Script mod",
    identified: true,
    confidence: "exact",
    enabled: true,
    dependencyCount: 0,
    whatItDoes:
      "Improves Build/Buy organization, filtering and advanced object visibility for builders.",
    dependencies: [],
    localFiles: ["Tmex-BetterBuildBuy.package", "BetterBuildBuy.ts4script"],
    evidence: [
      {
        type: "fact",
        title: "Exact CurseForge fingerprint",
        detail: "The installed archive maps deterministically to the known release.",
        source: "CurseForge"
      }
    ],
    relatedMods: ["TOOL", "Better Exceptions"]
  },
  {
    id: "ui-cheats",
    canonicalName: "UI Cheats Extension",
    filenameAliases: ["weerbesu_UI_Cheats_Extension.package"],
    creator: "weerbesu",
    category: "Interface",
    installedVersion: "1.46.1",
    latestVersion: "1.46.1",
    status: "Compatible",
    tone: "healthy",
    source: "Creator page",
    kind: "Package only",
    identified: true,
    confidence: "high",
    enabled: true,
    dependencyCount: 0,
    whatItDoes:
      "Adds direct UI interactions for needs, money, relationships, careers and other common gameplay values.",
    dependencies: [],
    localFiles: ["weerbesu_UI_Cheats_Extension.package"],
    evidence: [
      {
        type: "inference",
        title: "High-confidence identity",
        detail: "Canonical filename and creator identity agree with the registry record."
      }
    ],
    relatedMods: ["MC Command Center", "More Columns in CAS"]
  },
  {
    id: "cas-overlap",
    canonicalName: "CAS Lighting Override",
    filenameAliases: ["CAS_Lighting_Golden.package", "cas-light.package"],
    creator: "Local library",
    category: "CAS / Visual",
    installedVersion: null,
    latestVersion: null,
    status: "Potential conflict",
    tone: "warning",
    source: "Local only",
    kind: "Package only",
    identified: false,
    confidence: "unresolved",
    enabled: true,
    dependencyCount: 0,
    whatItDoes:
      "Unresolved package that appears to override CAS lighting resources. The file remains usable even without a canonical registry identity.",
    dependencies: [],
    localFiles: ["CAS_Lighting_Golden.package"],
    evidence: [
      {
        type: "fact",
        title: "Local resource overlap",
        detail: "This package shares DBPF resource keys with another enabled package. That is a potential conflict, not proof of breakage."
      }
    ],
    relatedMods: []
  },
  {
    id: "old-tool",
    canonicalName: "TOOL",
    filenameAliases: ["Tmex-TOOL.package", "Tmex-TOOL.ts4script"],
    creator: "TwistedMexi",
    category: "Build / Buy",
    installedVersion: "2.8.0",
    latestVersion: "2.8.1",
    status: "Broken",
    tone: "danger",
    source: "CurseForge",
    kind: "Script mod",
    identified: true,
    confidence: "exact",
    enabled: false,
    dependencyCount: 0,
    whatItDoes:
      "Provides advanced object positioning and manipulation tools outside normal Build/Buy placement limits.",
    dependencies: [],
    localFiles: ["Tmex-TOOL.package", "Tmex-TOOL.ts4script"],
    evidence: [
      {
        type: "fact",
        title: "Known broken release",
        detail: "The installed release has sourced broken-state evidence for the current patch.",
        source: "CurseForge"
      }
    ],
    relatedMods: ["Better BuildBuy"]
  }
];

export const libraryFacets = {
  statuses: ["Compatible", "Update available", "Potential conflict", "Broken", "Unknown"],
  categories: [...new Set(libraryFixture.map((item) => item.category))],
  creators: [...new Set(libraryFixture.map((item) => item.creator))],
  sources: [...new Set(libraryFixture.map((item) => item.source))]
} as const;

export function findLibraryItem(id: string) {
  const item = libraryFixture.find((entry) => entry.id === id) ?? libraryFixture[0];
  return {
    ...item,
    gameVersion: "1.128.90",
    libraryCount: 324,
    modsRoot: "C:/Users/Player/Documents/Electronic Arts/The Sims 4/Mods"
  };
}
