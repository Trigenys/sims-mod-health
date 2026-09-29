import { readFile } from "node:fs/promises";

const files = [
  "src/features/overview/OverviewPage.tsx",
  "src/features/library/LibraryPage.tsx",
  "src/features/library/LibraryStateNotice.tsx",
  "src/features/library/ModDetailPage.tsx",
  "src/features/health/HealthPage.tsx",
  "src/features/discover/DiscoverPage.tsx",
  "src/features/diagnostics/DiagnosticsPage.tsx",
  "src/features/settings/SettingsPage.tsx",
  "src/features/game-content/GameContentDrawer.tsx"
];

const bannedVisibleCopy = [
  "Registry offline",
  "Registry data is partial",
  "Partial registry results",
  "Registry request failed",
  "canonical identity",
  "canonical mod",
  "source adapters",
  "deterministic score",
  "Registry recommendations unavailable",
  "Recommendation evidence is partial",
  "evidence-first"
];

const failures = [];

for (const file of files) {
  const content = await readFile(file, "utf8");
  for (const phrase of bannedVisibleCopy) {
    const quoted = [
      JSON.stringify(phrase),
      "'" + phrase.replaceAll("'", "\\'") + "'"
    ];
    if (quoted.some((value) => content.includes(value))) {
      failures.push(file + ": " + phrase);
    }
  }
}

if (failures.length > 0) {
  console.error("Player-facing copy guard failed:");
  for (const failure of failures) console.error(" - " + failure);
  process.exit(1);
}

console.log("Player-facing copy guard: primary UI has no banned internal jargon.");
