import type { StatusTone } from "../../components/ui/StatusBadge";

export type OverviewStat = {
  label: string;
  value: string;
  tone: StatusTone;
};

export type AttentionItem = {
  name: string;
  creator: string;
  detail: string;
  badge: string;
  tone: StatusTone;
};

export const overviewFixture = {
  gameVersion: "Patch 1.128.90",
  platform: "Windows",
  indexedCount: 324,
  healthScore: 87,
  attentionCount: 28,
  lastScan: "Last scan 4 min ago",
  stats: [
    { label: "Healthy", value: "267", tone: "healthy" },
    { label: "Updates", value: "21", tone: "update" },
    { label: "Conflicts", value: "7", tone: "warning" },
    { label: "Unknown", value: "25", tone: "muted" }
  ] satisfies OverviewStat[],
  attention: [
    {
      name: "MC Command Center",
      creator: "Deaderpool",
      detail: "Update available · installed 2026.4.0",
      badge: "Update",
      tone: "update"
    },
    {
      name: "Relationship & Pregnancy Overhaul",
      creator: "Lumpinou",
      detail: "Compatibility not confirmed for current patch",
      badge: "Unknown",
      tone: "muted"
    },
    {
      name: "Duplicate CAS package",
      creator: "Local library",
      detail: "2 exact copies share the same SHA-256 fingerprint",
      badge: "Duplicate",
      tone: "warning"
    }
  ] satisfies AttentionItem[],
  installation: {
    mods: 43,
    customContent: 177,
    unidentified: 25,
    exactDuplicates: 18
  }
} as const;
