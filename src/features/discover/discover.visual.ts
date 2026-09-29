import type { DiscoverySnapshot } from "./discover.types";

export const discoverVisualFixture: DiscoverySnapshot = {
  patchVersion: "1.128.90",
  state: "ready",
  detail: "Recommendations are ready for the current game and installed mods.",
  blocker: null,
  prerequisites: {
    gameDetected: true,
    patchKnown: true,
    packsKnown: true,
    installedPackCount: 17,
    modsScanned: true,
    installedModFiles: 324,
    identifiedMods: 28,
    registryAvailable: true
  },
  recommendations: [
    {
      modId: "visual-growing-together",
      releaseId: "visual-growing-together-r1",
      name: "Sim Realist — Organic Hair",
      creatorName: "SimRealist",
      categories: ["cas", "realism"],
      features: ["appearance", "realism"],
      score: 24,
      reason: {
        becauseModId: "visual-rpo",
        becauseModName: "Relationship & Pregnancy Overhaul",
        sharedCategories: ["realism"],
        sharedFeatures: ["appearance", "realism"],
        explanation:
          "Because you use Relationship & Pregnancy Overhaul: Organic Hair shares realism-focused features."
      }
    },
    {
      modId: "visual-tool",
      releaseId: "visual-tool-r1",
      name: "TOOL",
      creatorName: "TwistedMexi",
      categories: ["build-buy"],
      features: ["build-tools"],
      score: 18,
      reason: {
        becauseModId: "visual-bbb",
        becauseModName: "Better BuildBuy",
        sharedCategories: ["build-buy"],
        sharedFeatures: ["build-tools"],
        explanation:
          "Because you use Better BuildBuy: TOOL shares build tools and the Build/Buy category."
      }
    },
    {
      modId: "visual-rpo-addon",
      releaseId: "visual-rpo-addon-r1",
      name: "RPO — Module & Pregnancy Additions",
      creatorName: "Lumpinou",
      categories: ["relationships", "gameplay"],
      features: ["relationship-management"],
      score: 14,
      reason: {
        becauseModId: "visual-rpo",
        becauseModName: "Relationship & Pregnancy Overhaul",
        sharedCategories: ["relationships"],
        sharedFeatures: ["relationship-management"],
        explanation:
          "Because you use Relationship & Pregnancy Overhaul: this candidate shares relationship-management features."
      }
    },
    {
      modId: "visual-parties",
      releaseId: "visual-parties-r1",
      name: "ChippedSim — Party Passion",
      creatorName: "ChippedSim",
      categories: ["gameplay", "social"],
      features: ["social-events"],
      score: 8,
      reason: {
        becauseModId: "visual-mccc",
        becauseModName: "MC Command Center",
        sharedCategories: ["gameplay"],
        sharedFeatures: [],
        explanation:
          "Because you use MC Command Center: Party Passion shares the gameplay category."
      }
    }
  ]
};


export const discoverBlockedVisualFixture: DiscoverySnapshot = {
  patchVersion: null,
  state: "blocked",
  detail: "The Sims 4 program installation has not been detected yet.",
  blocker: "game",
  prerequisites: {
    gameDetected: false,
    patchKnown: false,
    packsKnown: false,
    installedPackCount: null,
    modsScanned: true,
    installedModFiles: 2407,
    identifiedMods: null,
    registryAvailable: null
  },
  recommendations: []
};
