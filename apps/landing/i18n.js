const STORAGE_KEY = "smh-language";

const copy = {
  fr: {
    "Offline Desktop Engine": "Moteur desktop hors ligne",
    "Features": "Fonctionnalités",
    "How it Works": "Comment ça marche",
    "Privacy & Architecture": "Confidentialité & architecture",
    "Guiding Principles": "Principes directeurs",
    "Download Beta": "Télécharger la bêta",
    "Download Windows Beta 1": "Télécharger la bêta Windows 1",
    "Windows Beta 1 • Public prerelease": "Bêta Windows 1 • Préversion publique",
    "Tauri + React Desktop App": "Application desktop Tauri + React",
    "Keep your Sims 4 Mods healthy, organized &": "Gardez vos mods Sims 4 sains, organisés et",
    "crash-free": "sans crash",
    "The offline-first health manager for The Sims 4. Inventories your Mods folder, detects conflicts & duplicates, verifies compatibility, and keeps raw files strictly on your PC.": "Le gestionnaire de santé hors ligne pour Les Sims 4. Il inventorie votre dossier Mods, détecte conflits et doublons, vérifie la compatibilité et garde vos fichiers bruts strictement sur votre PC.",
    "Explore Live Demo & Principles": "Explorer la démo & les principes",
    "100% Offline-First": "100 % hors ligne d'abord",
    "Raw .package Stays Local": "Les .package restent en local",
    "Zero-Execution Safe AST": "AST sûr, zéro exécution",
    "Windows 10 / 11 64-bit": "Windows 10 / 11 64 bits",
    "Online Hub Sync: Idle": "Synchronisation hub : inactive",
    "1,384 Healthy": "1 384 sains",
    "3 Conflicts": "3 conflits",
    "2 Missing Dep": "2 dépendances manquantes",
    "4 Duplicates": "4 doublons",
    "Scanned Active Mod Manifest": "Manifest des mods actifs analysés",
    "SORTED BY DIAGNOSTIC SEVERITY": "TRIÉ PAR SÉVÉRITÉ DU DIAGNOSTIC",
    "by Deaderpool • 4 sub-modules verified (mc_cmd_center.ts4script)": "par Deaderpool • 4 sous-modules vérifiés (mc_cmd_center.ts4script)",
    "AST Parser: Compatible with Patch 1.128.90": "Analyseur AST : compatible avec le patch 1.128.90",
    "Up to date": "À jour",
    "4.2 MB total": "4,2 Mo au total",
    "by KawaiiStacie • Modifies SimBuff Tuning: ID 0x3E1882": "par KawaiiStacie • Modifie le tuning SimBuff : ID 0x3E1882",
    "Explain, do not alarm: Potential overlap with Realistic Reactions v2": "Expliquer sans alarmer : chevauchement potentiel avec Realistic Reactions v2",
    "Tuning Conflict": "Conflit de tuning",
    "Inspect Diff": "Inspecter le diff",
    "Identical binary hash collision across 2 separate subfolders": "Collision de hash binaire identique dans 2 sous-dossiers distincts",
    "Found in: /Mods/Makeup & /Mods/Overrides": "Trouvé dans : /Mods/Makeup & /Mods/Overrides",
    "Duplicate Found": "Doublon trouvé",
    "Prune One Copy": "Supprimer une copie",
    "Targeted by: LGBTQ+ Mod and Relationship & Pregnancy Overhaul": "Requis par : LGBTQ+ Mod et Relationship & Pregnancy Overhaul",
    "Requires lumpinou_toolbox_v4.5.package": "Nécessite lumpinou_toolbox_v4.5.package",
    "Missing Dep": "Dépendance manquante",
    "Download Guide": "Guide de téléchargement",
    "Engine Inspector": "Inspecteur moteur",
    "Safe AST Parsing": "Analyse AST sécurisée",
    "Script mods are strictly parsed for class signatures and injection hooks using Abstract Syntax Tree tokens.": "Les mods script sont analysés strictement pour leurs signatures de classes et hooks d'injection à l'aide de jetons d'arbre syntaxique abstrait.",
    "0 bytes of Python or machine code executed": "0 octet de Python ou de code machine exécuté",
    "on your machine.": "sur votre machine.",
    "Zero-Transmission SHA-256": "SHA-256 sans transmission",
    "Local files stay strictly on your NVMe/SSD drive. Only the 64-character SHA-256 checksum is compared against the verified creator registry to test for out-of-date patch versions.": "Les fichiers locaux restent strictement sur votre NVMe/SSD. Seule l'empreinte SHA-256 de 64 caractères est comparée au registre vérifié des créateurs pour détecter les versions de patch obsolètes.",
    "Auto-Rollback Ready": "Rollback automatique prêt",
    "Before modifying or moving any duplicate package, a timestamped snapshot is saved into": "Avant toute modification ou déplacement d'un package en double, un snapshot horodaté est enregistré dans",
    "Create Instant Snapshot": "Créer un snapshot instantané",
    "Engine Workflow": "Workflow du moteur",
    "How Sims Mod Health diagnoses 50GB folders in under four seconds.": "Comment Sims Mod Health diagnostique des dossiers de 50 Go en moins de quatre secondes.",
    "Built in Rust with native thread pooling. No slow browser engines touching your files, no game launch delays.": "Construit en Rust avec un pool de threads natif. Aucun moteur de navigateur lent ne touche vos fichiers et aucun délai n'est ajouté au lancement du jeu.",
    "Step 01 • Local Parse": "Étape 01 • Analyse locale",
    "Local Scan & AST Parsing": "Scan local & analyse AST",
    "Instant multi-threaded directory crawl. Raw": "Parcours multithreadé instantané des dossiers. Les fichiers",
    "and": "et",
    "inspected on-device with zero code execution.": "sont inspectés sur l'appareil sans aucune exécution de code.",
    ">10,000 files / sec": ">10 000 fichiers / s",
    "Step 02 • Hash Matching": "Étape 02 • Correspondance de hash",
    "Metadata-Only Resolution": "Résolution par métadonnées uniquement",
    "Only cryptographic hashes & creator manifests resolve against the cloud database. Your files never leave your drive or get uploaded anywhere.": "Seuls les hash cryptographiques et manifestes des créateurs sont résolus via la base distante. Vos fichiers ne quittent jamais votre disque et ne sont envoyés nulle part.",
    "Telemetry Off by Default": "Télémétrie désactivée par défaut",
    "Step 03 • Deep Diagnostic": "Étape 03 • Diagnostic approfondi",
    "Intelligent Diagnostics": "Diagnostics intelligents",
    "Pinpoint version mismatch, broken XML tuning tables, duplicate package files, and missing base libraries before the game even launches.": "Repérez les incompatibilités de version, tables XML cassées, packages dupliqués et bibliothèques de base manquantes avant même le lancement du jeu.",
    "XML & Buff Collision Trees": "Arbres de collisions XML & Buff",
    "Step 04 • Reversible Changes": "Étape 04 • Changements réversibles",
    "Safe Reversible Actions": "Actions sûres et réversibles",
    "One-click disable, safe folder sorting, backup creation, and atomic rollback. Never worry about corrupting your save files again.": "Désactivation en un clic, tri sûr des dossiers, création de sauvegarde et rollback atomique. Plus besoin de craindre de corrompre vos sauvegardes.",
    "1-Click Restore Guarantee": "Restauration en 1 clic",
    "Zero-Trust Architectural Model": "Architecture Zero-Trust",
    "Raw .package and .ts4script files never upload. Period.": "Les fichiers .package et .ts4script ne sont jamais envoyés. Point.",
    "Unlike web scanners that demand you drag gigabytes of CC files into an unknown remote cloud, Sims Mod Health treats your storage as an isolated vault. We only compare compact 32-byte cryptographic hashes over encrypted HTTPS.": "Contrairement aux scanners web qui demandent d'envoyer des gigaoctets de CC vers un cloud distant inconnu, Sims Mod Health traite votre stockage comme un coffre isolé. Nous ne comparons que de petits hash cryptographiques de 32 octets via HTTPS chiffré.",
    "Your PC (Local Sandbox)": "Votre PC (sandbox locale)",
    "Full file reading strictly on your CPU": "Lecture complète des fichiers uniquement sur votre CPU",
    "Local Python AST token analyzer": "Analyseur local de jetons AST Python",
    "Encrypted local database of your overrides": "Base locale chiffrée de vos overrides",
    "Instant undo backups on SSD": "Sauvegardes d'annulation instantanées sur SSD",
    "Cloud Registry API": "API du registre cloud",
    "Receives SHA-256 string only": "Reçoit uniquement le hash SHA-256",
    "Returns verified author version tag": "Renvoie la version vérifiée du créateur",
    "Returns patch compatibility flags": "Renvoie les indicateurs de compatibilité du patch",
    "Zero file payloads accepted": "Aucun fichier brut accepté",
    "Engine Security Specifications": "Spécifications de sécurité du moteur",
    "Tauri v2 + Rust Core": "Tauri v2 + cœur Rust",
    "Sub-millisecond file handles with zero Chromium overhead.": "Accès fichiers sub-milliseconde sans surcharge Chromium.",
    "Modern React Front-end": "Frontend React moderne",
    "Fluid 120 FPS interface designed for high-refresh gaming monitors.": "Interface fluide 120 FPS pensée pour les écrans gaming à haut taux de rafraîchissement.",
    "Zero-PAT GitHub OIDC": "GitHub OIDC sans PAT",
    "Decentralized author signatures via secure cryptographic broker.": "Signatures d'auteurs décentralisées via un courtier cryptographique sécurisé.",
    "Audit-Ready Open Manifest": "Manifest ouvert prêt pour audit",
    "Community-governed definitions for known conflict pairs.": "Définitions communautaires des paires de conflits connus.",
    "Ethical Simming": "Simming responsable",
    "Six Guiding Principles Behind Every Diagnostic": "Six principes derrière chaque diagnostic",
    "Built by creators and long-time players who care deeply about creator attribution, user privacy, and mod health.": "Conçu par des créateurs et joueurs de longue date attachés à l'attribution, à la confidentialité et à la santé des mods.",
    "Offline-First Design": "Conception hors ligne d'abord",
    "Scanning and local mod inventory work completely without an internet connection or user account. Play offline, diagnose offline, fix offline.": "Le scan et l'inventaire local fonctionnent entièrement sans connexion Internet ni compte. Jouez hors ligne, diagnostiquez hors ligne, corrigez hors ligne.",
    "Evidence Before Confidence": "Les preuves avant la certitude",
    "Unknown mods are never silently flagged as \"Broken\" or deleted. We only show alerts when explicit binary or XML collisions are verified.": "Les mods inconnus ne sont jamais marqués silencieusement « cassés » ni supprimés. Nous affichons une alerte uniquement lorsqu'une collision binaire ou XML explicite est vérifiée.",
    "Safe Static Inspection": "Inspection statique sécurisée",
    "Python script mods (": "Les mods script Python (",
    ") are parsed purely for module signatures and tuning tables. Code is never evaluated or executed.": ") sont analysés uniquement pour leurs signatures de modules et tables de tuning. Le code n'est jamais évalué ni exécuté.",
    "Explain, Do Not Alarm": "Expliquer, sans alarmer",
    "Resource overlaps are framed as \"potential interactions\" with helpful context, rather than scary red error popups that induce panic.": "Les chevauchements de ressources sont présentés comme des « interactions potentielles » avec du contexte utile, plutôt que comme des erreurs rouges anxiogènes.",
    "Recoverable Changes": "Changements récupérables",
    "Every disable, subfolder move, or batch quarantine creates an automated snapshot. Revert any action at any time with a single click.": "Chaque désactivation, déplacement de sous-dossier ou quarantaine crée automatiquement un snapshot. Revenez sur une action à tout moment en un clic.",
    "Source-Respectful": "Respect des sources",
    "We honor modder release channels, Patreon tiers, and explicit developer preferences. Zero file re-hosting, zero paywall bypassing, zero unauthorized scraping.": "Nous respectons les canaux de publication des moddeurs, leurs niveaux Patreon et leurs préférences explicites. Aucun réhébergement de fichiers, contournement de paywall ou scraping non autorisé.",
    "Ready for Patch 1.128.90": "Prêt pour le patch 1.128.90",
    "Start diagnosing your Sims 4 Mods folder today.": "Commencez à diagnostiquer votre dossier Mods Sims 4 dès aujourd'hui.",
    "Get the Windows Beta 1 MSI. Core scanning works locally, no account is required, and the release workflow validates clean installation and uninstallation.": "Téléchargez le MSI Windows Beta 1. Le scan principal fonctionne en local, aucun compte n'est requis et le workflow de release valide l'installation et la désinstallation propres.",
    "Download Beta 1 (x64 Installer)": "Télécharger la bêta 1 (installateur x64)",
    "Release Notes": "Notes de version",
    "• Requirements: Windows 10/11 64-bit": "• Prérequis : Windows 10/11 64 bits",
    "• File Size:": "• Taille :",
    "SHA-256 checksum": "somme SHA-256",
    "• Code signing: unsigned beta": "• Signature du code : bêta non signée",
    "The private, blazing-fast desktop companion for Simmers. Parses thousands of .package files, script injectors, and tuning overrides locally with diagnostic telemetry off by default.": "Le compagnon desktop privé et rapide pour les joueurs des Sims. Il analyse localement des milliers de fichiers .package, injecteurs de scripts et overrides de tuning, avec la télémétrie de diagnostic désactivée par défaut.",
    "Local Windows App": "Application Windows locale",
    "Engine": "Moteur",
    "Diagnostic Pipeline": "Pipeline de diagnostic",
    "Conflict Resolver Diff": "Diff du résolveur de conflits",
    "Package Hash Database": "Base de hash des packages",
    "Read-only DBPF Parser": "Parseur DBPF en lecture seule",
    "Safety & Specs": "Sécurité & spécifications",
    "Offline Verification": "Vérification hors ligne",
    "Windows Beta Validation": "Validation de la bêta Windows",
    "File Integrity Guard": "Contrôle d'intégrité des fichiers",
    "Open Documentation": "Documentation ouverte",
    "Beta Channel": "Canal bêta",
    "Release": "Version",
    "Patch-scoped health evidence. Validated Windows x64 MSI installer.": "État de santé lié au patch. Installateur MSI Windows x64 validé.",
    "View Checksum Hashes": "Voir les sommes de contrôle",
    "Sims Mod Health. Not affiliated with or endorsed by Electronic Arts or Maxis.": "Sims Mod Health. Non affilié à Electronic Arts ou Maxis et non approuvé par eux.",
    "Architecture Manifest": "Manifeste d'architecture",
    "Security Attestation": "Attestation de sécurité",
    "License & Source": "Licence & source"
  }
};

const metadata = {
  en: {
    title: "Sims Mod Health — Offline-First Mod Diagnostics",
    description:
      "Keep your Sims 4 Mods healthy with local scanning, evidence-led diagnostics, compatibility checks, duplicates and safe rollback."
  },
  fr: {
    title: "Sims Mod Health — Diagnostic de mods hors ligne",
    description:
      "Gardez vos mods Sims 4 sains grâce au scan local, aux diagnostics fondés sur des preuves, aux vérifications de compatibilité, aux doublons et au rollback sûr."
  }
};

const originalText = new WeakMap();

document.addEventListener("DOMContentLoaded", () => {
  captureOriginalText();

  const saved = localStorage.getItem(STORAGE_KEY);
  const initial =
    saved === "fr" || saved === "en"
      ? saved
      : navigator.language?.toLowerCase().startsWith("fr")
        ? "fr"
        : "en";

  applyLanguage(initial);

  document.querySelectorAll("[data-lang]").forEach((button) => {
    button.addEventListener("click", () => {
      const language = button.getAttribute("data-lang");
      if (language === "en" || language === "fr") {
        localStorage.setItem(STORAGE_KEY, language);
        applyLanguage(language);
      }
    });
  });
});

function captureOriginalText() {
  const walker = document.createTreeWalker(
    document.body,
    NodeFilter.SHOW_TEXT,
    {
      acceptNode(node) {
        const parent = node.parentElement;
        if (!parent || parent.closest("script, style")) {
          return NodeFilter.FILTER_REJECT;
        }
        return normalize(node.nodeValue) ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT;
      }
    }
  );

  let node;
  while ((node = walker.nextNode())) {
    originalText.set(node, node.nodeValue);
  }
}

function applyLanguage(language) {
  const translations = language === "fr" ? copy.fr : {};

  for (const element of document.querySelectorAll("[data-copy-en][data-copy-fr]")) {
    element.textContent =
      language === "fr"
        ? element.getAttribute("data-copy-fr")
        : element.getAttribute("data-copy-en");
  }

  for (const [node, source] of textEntries()) {
    const normalized = normalize(source);
    const translated = translations[normalized];

    if (language === "fr" && translated) {
      node.nodeValue = preserveOuterWhitespace(source, translated);
    } else {
      node.nodeValue = source;
    }
  }

  document.documentElement.lang = language;

  const pageTitle =
    language === "fr"
      ? document.body?.getAttribute("data-title-fr")
      : document.body?.getAttribute("data-title-en");
  const pageDescription =
    language === "fr"
      ? document.body?.getAttribute("data-description-fr")
      : document.body?.getAttribute("data-description-en");

  document.title = pageTitle || metadata[language].title;

  const description = document.querySelector('meta[name="description"]');
  if (description) {
    description.setAttribute(
      "content",
      pageDescription || metadata[language].description
    );
  }

  document.querySelectorAll("[data-lang]").forEach((button) => {
    const active = button.getAttribute("data-lang") === language;
    button.setAttribute("aria-pressed", String(active));
    button.classList.toggle("bg-primary", active);
    button.classList.toggle("text-on-primary", active);
    button.classList.toggle("text-on-surface-variant", !active);
  });

  const switcher = document.querySelector("[data-lang-switch]");
  if (switcher) {
    switcher.setAttribute(
      "aria-label",
      language === "fr" ? "Choisir la langue" : "Choose language"
    );
  }
}

function* textEntries() {
  const walker = document.createTreeWalker(
    document.body,
    NodeFilter.SHOW_TEXT,
    {
      acceptNode(node) {
        return originalText.has(node)
          ? NodeFilter.FILTER_ACCEPT
          : NodeFilter.FILTER_REJECT;
      }
    }
  );

  let node;
  while ((node = walker.nextNode())) {
    yield [node, originalText.get(node)];
  }
}

function normalize(value) {
  return String(value ?? "").replace(/\s+/g, " ").trim();
}

function preserveOuterWhitespace(source, translated) {
  const leading = source.match(/^\s*/)?.[0] ?? "";
  const trailing = source.match(/\s*$/)?.[0] ?? "";
  return `${leading}${translated}${trailing}`;
}
