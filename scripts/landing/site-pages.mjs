import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

export const productNav = [
  ["/", "Home", "Accueil"],
  ["/download/", "Download", "Télécharger"],
  ["/docs/", "Docs", "Documentation"],
  ["/security/", "Security", "Sécurité"],
  ["/support/", "Support", "Assistance"]
];

export function renderFaviconLinks() {
  return `<link rel="icon" type="image/svg+xml" href="/logo.svg" />`;
}

const sections = {
  download: {
    titleEn: "Download Sims Mod Health",
    titleFr: "Télécharger Sims Mod Health",
    eyebrowEn: "WINDOWS BETA",
    eyebrowFr: "BÊTA WINDOWS",
    introEn:
      "Get the validated Windows x64 MSI from the current public beta channel. The release pipeline builds, installs, uninstalls, checksums and publishes the same artifact.",
    introFr:
      "Téléchargez le MSI Windows x64 validé depuis le canal bêta public. Le pipeline construit, installe, désinstalle, vérifie et publie exactement le même artefact.",
    content: `
      <div class="grid lg:grid-cols-[1.2fr_.8fr] gap-6">
        <section class="rounded-3xl border border-outline-variant/50 bg-surface-container-lowest p-7 md:p-9 shadow-sm">
          <div class="flex flex-wrap items-center gap-3 mb-6">
            <span class="px-3 py-1.5 rounded-full bg-primary-container text-on-primary-container text-xs font-extrabold" data-release-version>v0.1.0-beta.1</span>
            <span class="px-3 py-1.5 rounded-full bg-surface-container text-on-surface-variant text-xs font-bold">MSI · x64 · <span data-download-size>5.79 MB</span></span>
          </div>
          <h2 class="text-2xl md:text-3xl font-extrabold tracking-tight mb-3" data-copy-en="Windows 10 / 11 installer" data-copy-fr="Installateur Windows 10 / 11">Windows 10 / 11 installer</h2>
          <p class="text-on-surface-variant leading-7 mb-7" data-copy-en="No account is required. Core scanning remains local. The beta installer is currently unsigned, so Windows may display an unknown-publisher warning." data-copy-fr="Aucun compte n'est requis. Le scan principal reste local. L'installateur bêta n'est pas encore signé, Windows peut donc afficher un avertissement d'éditeur inconnu.">No account is required. Core scanning remains local. The beta installer is currently unsigned, so Windows may display an unknown-publisher warning.</p>
          <a data-binary-download-link href="https://github.com/Trigenys/sims-mod-health/releases/download/v0.1.0-beta.1/Sims-Mod-Health-0.1.0-beta.1-x64.msi" class="inline-flex items-center gap-3 rounded-full bg-primary text-on-primary px-6 py-3.5 font-extrabold shadow-md hover:shadow-lg transition">
            <span class="material-symbols-outlined">download</span>
            <span data-copy-en="Download the MSI" data-copy-fr="Télécharger le MSI">Download the MSI</span>
          </a>
          <div class="flex flex-wrap gap-4 mt-6 text-sm font-bold">
            <a href="/verify/" class="text-primary hover:underline" data-copy-en="Verify SHA-256" data-copy-fr="Vérifier le SHA-256">Verify SHA-256</a>
            <a href="/release-notes/" class="text-primary hover:underline" data-copy-en="Read release notes" data-copy-fr="Lire les notes de version">Read release notes</a>
          </div>
        </section>
        <aside class="rounded-3xl bg-primary text-on-primary p-7 md:p-9">
          <p class="text-xs font-extrabold tracking-[.15em] uppercase opacity-80 mb-4" data-copy-en="Before installing" data-copy-fr="Avant l'installation">Before installing</p>
          <div class="space-y-5">
            <div><strong class="block mb-1" data-copy-en="1. Verify the checksum" data-copy-fr="1. Vérifiez la somme de contrôle">1. Verify the checksum</strong><span class="text-sm opacity-80" data-copy-en="Compare the downloaded MSI against the published SHA-256 file." data-copy-fr="Comparez le MSI téléchargé au fichier SHA-256 publié.">Compare the downloaded MSI against the published SHA-256 file.</span></div>
            <div><strong class="block mb-1" data-copy-en="2. Expect a beta warning" data-copy-fr="2. Attendez-vous à un avertissement bêta">2. Expect a beta warning</strong><span class="text-sm opacity-80" data-copy-en="Beta 1 is not code-signed yet. Checksum verification is important." data-copy-fr="La bêta 1 n'est pas encore signée. La vérification du checksum est importante.">Beta 1 is not code-signed yet. Checksum verification is important.</span></div>
            <div><strong class="block mb-1" data-copy-en="3. Keep your Mods folder backed up" data-copy-fr="3. Gardez une sauvegarde de votre dossier Mods">3. Keep your Mods folder backed up</strong><span class="text-sm opacity-80" data-copy-en="The app is designed to be conservative, but a beta should still be treated as beta software." data-copy-fr="L'application est conçue pour être prudente, mais une bêta reste un logiciel en phase bêta.">The app is designed to be conservative, but a beta should still be treated as beta software.</span></div>
          </div>
        </aside>
      </div>`
  },
  "release-notes": {
    titleEn: "Beta 1 release notes",
    titleFr: "Notes de version Bêta 1",
    eyebrowEn: "CURRENT RELEASE",
    eyebrowFr: "VERSION ACTUELLE",
    introEn:
      "What is in the current public beta, what is intentionally limited, and what the release pipeline validated before publication.",
    introFr:
      "Ce que contient la bêta publique actuelle, ses limites volontaires et ce que le pipeline a validé avant publication.",
    content: `
      <div class="grid md:grid-cols-2 gap-6">
        ${card("Included", "Inclus", [
          ["Local installation discovery and incremental Mods scanning.", "Détection locale de l'installation et scan incrémental du dossier Mods."],
          ["Read-only DBPF and TS4Script inspection.", "Inspection DBPF et TS4Script en lecture seule."],
          ["Duplicate, conflict, compatibility and dependency evidence.", "Preuves de doublons, conflits, compatibilité et dépendances."],
          ["Evidence-linked exception diagnostics.", "Diagnostics d'exceptions reliés aux preuves."],
          ["Single-artifact staged update with restore point and rollback.", "Mise à jour d'un artefact avec staging, point de restauration et rollback."]
        ])}
        ${card("Known limitations", "Limites connues", [
          ["Windows only for Beta 1.", "Windows uniquement pour la Bêta 1."],
          ["Unsigned MSI installer.", "Installateur MSI non signé."],
          ["Registry-backed features require a reachable Registry API.", "Les fonctions liées au registre nécessitent une API Registry accessible."],
          ["Bulk updates and multi-file archive extraction are disabled.", "Les mises à jour en masse et l'extraction d'archives multi-fichiers sont désactivées."],
          ["Diagnostic correlation is not proof of causality.", "Une corrélation de diagnostic ne prouve pas la causalité."]
        ])}
      </div>
      <div class="mt-6 rounded-3xl border border-outline-variant/50 bg-surface-container-low p-7">
        <div class="flex flex-wrap items-center justify-between gap-4">
          <div><div class="text-xs font-extrabold tracking-[.15em] uppercase text-primary mb-2" data-copy-en="Release traceability" data-copy-fr="Traçabilité de la version">Release traceability</div><div class="font-extrabold text-xl" data-release-version>v0.1.0-beta.1</div></div>
          <div class="flex flex-wrap gap-3"><a href="/download/" class="pill-primary" data-copy-en="Download" data-copy-fr="Télécharger">Download</a><a href="/verify/" class="pill-secondary" data-copy-en="Verify checksum" data-copy-fr="Vérifier le checksum">Verify checksum</a></div>
        </div>
      </div>`
  },
  verify: {
    titleEn: "Verify your download",
    titleFr: "Vérifier votre téléchargement",
    eyebrowEn: "FILE INTEGRITY",
    eyebrowFr: "INTÉGRITÉ DU FICHIER",
    introEn:
      "Confirm that the MSI on your PC is byte-for-byte the artifact published by the release workflow before you install it.",
    introFr:
      "Confirmez que le MSI présent sur votre PC correspond exactement à l'artefact publié par le workflow avant de l'installer.",
    content: `
      <div class="grid lg:grid-cols-[.9fr_1.1fr] gap-6">
        ${card("PowerShell", "PowerShell", [
          ["Open PowerShell in the folder containing the MSI.", "Ouvrez PowerShell dans le dossier contenant le MSI."],
          ["Run the command shown on the right.", "Exécutez la commande affichée à droite."],
          ["Compare the result with SHA256SUMS.txt.", "Comparez le résultat avec SHA256SUMS.txt."]
        ])}
        <section class="rounded-3xl bg-[#0c1722] text-white p-7 md:p-9 overflow-hidden">
          <div class="text-xs uppercase tracking-[.14em] text-emerald-300 font-extrabold mb-4">SHA-256</div>
          <code class="block rounded-2xl bg-black/30 p-5 text-sm leading-7 overflow-x-auto">Get-FileHash .\Sims-Mod-Health-0.1.0-beta.1-x64.msi -Algorithm SHA256</code>
          <a data-external-checksum-link href="https://github.com/Trigenys/sims-mod-health/releases/download/v0.1.0-beta.1/SHA256SUMS.txt" class="inline-flex mt-5 items-center gap-2 text-emerald-300 font-bold hover:underline"><span class="material-symbols-outlined text-base">verified</span><span data-copy-en="Open the official checksum file" data-copy-fr="Ouvrir le fichier checksum officiel">Open the official checksum file</span></a>
        </section>
      </div>`
  },
  architecture: {
    titleEn: "Architecture",
    titleFr: "Architecture",
    eyebrowEn: "HOW THE PRODUCT IS BUILT",
    eyebrowFr: "COMMENT LE PRODUIT EST CONSTRUIT",
    introEn:
      "A native desktop boundary for local files, a registry for evidence, and explicit separation between local certainty and remote metadata.",
    introFr:
      "Une frontière desktop native pour les fichiers locaux, un registre pour les preuves et une séparation explicite entre certitude locale et métadonnées distantes.",
    content: `
      <div class="grid md:grid-cols-3 gap-5">
        ${miniCard("Desktop", "Desktop", "Tauri + Rust owns filesystem access, scanning, parsing, diagnostics and mutation.", "Tauri + Rust gère l'accès fichiers, le scan, les parseurs, diagnostics et mutations.")}
        ${miniCard("UI", "Interface", "React renders health evidence and actions without receiving broad filesystem capabilities.", "React affiche les preuves de santé et les actions sans disposer de permissions fichiers étendues.")}
        ${miniCard("Registry", "Registre", "The API resolves canonical identity, compatibility and relationship evidence. Local files are not uploaded.", "L'API résout l'identité canonique, la compatibilité et les relations. Les fichiers locaux ne sont pas envoyés.")}
      </div>
      <div class="mt-6 grid md:grid-cols-2 gap-5">
        ${miniCard("Identity pipeline", "Pipeline d'identité", "Fingerprints, archive metadata and source adapters map local artifacts to canonical mods and releases.", "Empreintes, métadonnées d'archives et adaptateurs de sources relient les artefacts locaux aux mods et versions canoniques.")}
        ${miniCard("Safe mutation pipeline", "Pipeline de mutation sûr", "Supported updates are staged, verified, dependency-checked and protected by restore points.", "Les mises à jour prises en charge sont préparées, vérifiées, contrôlées côté dépendances et protégées par des points de restauration.")}
      </div>`
  },
  security: {
    titleEn: "Security & privacy",
    titleFr: "Sécurité & confidentialité",
    eyebrowEn: "BETA HARDENING",
    eyebrowFr: "DURCISSEMENT DE LA BÊTA",
    introEn:
      "The beta release gate covers parser abuse cases, WebView permissions, dependency advisories, privacy defaults and safe update recovery.",
    introFr:
      "Le gate de la bêta couvre les cas d'abus des parseurs, les permissions WebView, les vulnérabilités de dépendances, les valeurs de confidentialité et la récupération sûre.",
    content: `
      <div class="grid md:grid-cols-2 gap-5">
        ${miniCard("Raw files stay local", "Les fichiers bruts restent locaux", "Package and script contents are inspected locally. Registry resolution uses constrained metadata and fingerprints.", "Le contenu des packages et scripts est inspecté localement. La résolution via le registre utilise des métadonnées et empreintes limitées.")}
        ${miniCard("Telemetry is opt-in", "La télémétrie est opt-in", "Diagnostic telemetry is off by default. Invalid stored consent fails closed.", "La télémétrie de diagnostic est désactivée par défaut. Un consentement local invalide échoue de façon sûre.")}
        ${miniCard("Parsers are bounded", "Les parseurs sont bornés", "DBPF, TS4Script and diagnostic parsing enforce size/resource limits and deterministic adversarial tests.", "Les parseurs DBPF, TS4Script et diagnostics imposent des limites et des tests adversariaux déterministes.")}
        ${miniCard("Updates are recoverable", "Les mises à jour sont récupérables", "A restore point is created before supported mutation; rollback refuses to overwrite an independently changed target.", "Un point de restauration est créé avant mutation ; le rollback refuse d'écraser une cible modifiée indépendamment.")}
      </div>
      <div class="mt-6 flex flex-wrap gap-3"><a href="/architecture/" class="pill-secondary" data-copy-en="Architecture" data-copy-fr="Architecture">Architecture</a><a href="/verify/" class="pill-secondary" data-copy-en="File verification" data-copy-fr="Vérification des fichiers">File verification</a></div>`
  },
  docs: {
    titleEn: "Documentation",
    titleFr: "Documentation",
    eyebrowEn: "PRODUCT GUIDE",
    eyebrowFr: "GUIDE PRODUIT",
    introEn:
      "Start with the product pages below. Technical source documents remain available from the source page when you need implementation-level detail.",
    introFr:
      "Commencez par les pages produit ci-dessous. Les documents techniques sources restent accessibles depuis la page Source pour les détails d'implémentation.",
    content: `
      <div class="grid md:grid-cols-2 lg:grid-cols-3 gap-5">
        ${linkCard("/download/", "Install the beta", "Installer la bêta", "Requirements, download and installation context.", "Prérequis, téléchargement et contexte d'installation.")}
        ${linkCard("/release-notes/", "Release notes", "Notes de version", "Current capabilities and known limitations.", "Fonctionnalités actuelles et limites connues.")}
        ${linkCard("/verify/", "Verify the MSI", "Vérifier le MSI", "SHA-256 integrity verification before install.", "Vérification d'intégrité SHA-256 avant installation.")}
        ${linkCard("/architecture/", "Architecture", "Architecture", "Desktop, registry, identity and mutation boundaries.", "Frontières desktop, registre, identité et mutation.")}
        ${linkCard("/security/", "Security & privacy", "Sécurité & confidentialité", "Hardening posture and privacy defaults.", "Posture de durcissement et valeurs de confidentialité.")}
        ${linkCard("/support/", "Support", "Assistance", "Beta troubleshooting and issue reporting.", "Dépannage bêta et signalement de problèmes.")}
      </div>`
  },
  support: {
    titleEn: "Beta support",
    titleFr: "Assistance bêta",
    eyebrowEn: "TROUBLESHOOTING",
    eyebrowFr: "DÉPANNAGE",
    introEn:
      "Start with the checks below. If the problem is reproducible, the issue tracker is the final escalation path for the public beta.",
    introFr:
      "Commencez par les vérifications ci-dessous. Si le problème est reproductible, le suivi d'incidents est la dernière étape d'escalade pour la bêta publique.",
    content: `
      <div class="grid md:grid-cols-3 gap-5">
        ${miniCard("Installer warning", "Avertissement installateur", "Beta 1 is unsigned. Verify SHA-256 before continuing through an unknown-publisher warning.", "La Bêta 1 n'est pas signée. Vérifiez le SHA-256 avant de poursuivre malgré l'avertissement d'éditeur inconnu.")}
        ${miniCard("Registry offline", "Registre hors ligne", "Local scanning remains useful. Registry-dependent identity and health evidence can show offline or partial states.", "Le scan local reste utile. L'identité et les preuves dépendant du registre peuvent apparaître hors ligne ou partielles.")}
        ${miniCard("Interrupted update", "Mise à jour interrompue", "Do not delete restore-point or staging data. Preserve the app data directory and use the rollback path.", "Ne supprimez pas les données de restauration ou de staging. Préservez les données de l'application et utilisez le rollback.")}
      </div>
      <section class="mt-6 rounded-3xl border border-outline-variant/50 bg-surface-container-lowest p-7">
        <h2 class="text-xl font-extrabold mb-2" data-copy-en="Still stuck?" data-copy-fr="Toujours bloqué ?">Still stuck?</h2>
        <p class="text-on-surface-variant mb-5" data-copy-en="Use the public issue tracker for reproducible beta defects. Include the app version, Windows version, expected behavior and the smallest safe diagnostic evidence you can share." data-copy-fr="Utilisez le suivi d'incidents public pour les défauts reproductibles. Indiquez la version de l'app, de Windows, le comportement attendu et la plus petite preuve de diagnostic que vous pouvez partager en sécurité.">Use the public issue tracker for reproducible beta defects. Include the app version, Windows version, expected behavior and the smallest safe diagnostic evidence you can share.</p>
        <a href="https://github.com/Trigenys/sims-mod-health/issues/new" target="_blank" rel="noreferrer" class="pill-primary"><span data-copy-en="Open the issue tracker" data-copy-fr="Ouvrir le suivi d'incidents">Open the issue tracker</span><span class="material-symbols-outlined text-base">open_in_new</span></a>
      </section>`
  },
  source: {
    titleEn: "Source & transparency",
    titleFr: "Source & transparence",
    eyebrowEn: "PUBLIC ENGINEERING",
    eyebrowFr: "INGÉNIERIE PUBLIQUE",
    introEn:
      "The product repository and technical records are public. The website keeps product navigation first; source hosting remains a secondary engineering destination.",
    introFr:
      "Le dépôt produit et les documents techniques sont publics. Le site privilégie d'abord la navigation produit ; l'hébergement du code reste une destination technique secondaire.",
    content: `
      <div class="grid md:grid-cols-2 gap-5">
        ${miniCard("Repository", "Dépôt", "Desktop, registry, landing, CI workflows and technical documentation live in one public repository.", "Desktop, registre, landing, workflows CI et documentation technique vivent dans un dépôt public unique.")}
        ${miniCard("Traceability", "Traçabilité", "Release assets include SHA-256 checksums and build provenance tied to the validated source commit.", "Les artefacts de release incluent les sommes SHA-256 et la provenance de build liée au commit source validé.")}
      </div>
      <div class="mt-6 flex flex-wrap gap-3">
        <a href="https://github.com/Trigenys/sims-mod-health" target="_blank" rel="noreferrer" class="pill-primary"><span data-copy-en="Open source repository" data-copy-fr="Ouvrir le dépôt source">Open source repository</span><span class="material-symbols-outlined text-base">open_in_new</span></a>
        <a href="/docs/" class="pill-secondary" data-copy-en="Product documentation" data-copy-fr="Documentation produit">Product documentation</a>
      </div>`
  }
};

export async function buildSitePages(outDir) {
  for (const [slug, section] of Object.entries(sections)) {
    const dir = join(outDir, slug);
    await mkdir(dir, { recursive: true });
    await writeFile(
      join(dir, "index.html"),
      page(section, { activeHref: activeNavHrefForSlug(slug) }),
      "utf8"
    );
  }

  await writeFile(join(outDir, "404.html"), page({
    titleEn: "Page not found",
    titleFr: "Page introuvable",
    eyebrowEn: "404",
    eyebrowFr: "404",
    introEn: "That route does not exist. Return to the product home or open the documentation hub.",
    introFr: "Cette page n'existe pas. Revenez à l'accueil ou ouvrez la documentation.",
    content: '<div class="flex flex-wrap gap-3"><a class="pill-primary" href="/" data-copy-en="Back home" data-copy-fr="Retour à l’accueil">Back home</a><a class="pill-secondary" href="/docs/" data-copy-en="Documentation" data-copy-fr="Documentation">Documentation</a></div>'
  }, { activeHref: null }), "utf8");
}

function page(
  { titleEn, titleFr, eyebrowEn, eyebrowFr, introEn, introFr, content },
  { activeHref = null } = {}
) {
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta name="description" content="${escapeAttr(introEn)}" />
  <title>${escapeHtml(titleEn)} — Sims Mod Health</title>
  ${renderFaviconLinks()}
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
  <link href="https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:wght,FILL@100..700,0..1&display=swap" rel="stylesheet" />
  <link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;600;700;800&display=swap" rel="stylesheet" />
  <script src="https://cdn.tailwindcss.com"></script>
  <script>
    tailwind.config = {
      theme: { extend: {
        colors: {
          primary: "#007A50",
          "on-primary": "#ffffff",
          "primary-container": "#b7f3d5",
          "on-primary-container": "#00412a",
          surface: "#f8f9ff",
          "on-surface": "#121b25",
          "on-surface-variant": "#475467",
          "surface-container": "#edf1f8",
          "surface-container-low": "#f3f6fb",
          "surface-container-lowest": "#ffffff",
          "outline-variant": "#cfd8e3"
        },
        fontFamily: { sans: ["Plus Jakarta Sans", "sans-serif"] }
      }}
    };
  </script>
  <style>
    body{font-family:"Plus Jakarta Sans",sans-serif;background:#f8f9ff;color:#121b25}
    .pill-primary{display:inline-flex;align-items:center;gap:.5rem;border-radius:9999px;background:#007A50;color:#fff;padding:.75rem 1.2rem;font-weight:800}
    .pill-secondary{display:inline-flex;align-items:center;gap:.5rem;border-radius:9999px;background:#edf1f8;color:#121b25;padding:.75rem 1.2rem;font-weight:800}
  </style>
</head>
<body
  class="min-h-screen"
  data-title-en="${escapeAttr(titleEn)} — Sims Mod Health"
  data-title-fr="${escapeAttr(titleFr)} — Sims Mod Health"
  data-description-en="${escapeAttr(introEn)}"
  data-description-fr="${escapeAttr(introFr)}"
>
  ${renderProductHeader(activeHref)}

  <main class="mx-auto max-w-[1480px] px-5 md:px-8 py-16 md:py-24">
    <div class="max-w-4xl mb-12">
      <div class="text-xs font-extrabold tracking-[.18em] text-primary uppercase mb-4" data-copy-en="${escapeAttr(eyebrowEn)}" data-copy-fr="${escapeAttr(eyebrowFr)}">${escapeHtml(eyebrowEn)}</div>
      <h1 class="text-4xl md:text-6xl font-extrabold tracking-[-.045em] leading-[1.02] mb-5" data-copy-en="${escapeAttr(titleEn)}" data-copy-fr="${escapeAttr(titleFr)}">${escapeHtml(titleEn)}</h1>
      <p class="text-lg md:text-xl text-on-surface-variant leading-8" data-copy-en="${escapeAttr(introEn)}" data-copy-fr="${escapeAttr(introFr)}">${escapeHtml(introEn)}</p>
    </div>
    ${content}
  </main>

  <footer class="border-t border-outline-variant/50 bg-surface-container-low">
    <div class="mx-auto max-w-[1600px] px-5 md:px-8 py-8 flex flex-col md:flex-row gap-5 items-start md:items-center justify-between">
      <div><strong class="block">Sims Mod Health</strong><small class="text-on-surface-variant" data-copy-en="Unofficial community software. Not affiliated with Electronic Arts or Maxis." data-copy-fr="Logiciel communautaire non officiel. Non affilié à Electronic Arts ou Maxis.">Unofficial community software. Not affiliated with Electronic Arts or Maxis.</small></div>
      <div class="flex flex-wrap gap-4 text-sm font-bold text-on-surface-variant"><a href="/source/" data-copy-en="Source" data-copy-fr="Source">Source</a><a href="/security/" data-copy-en="Security" data-copy-fr="Sécurité">Security</a><a href="/support/" data-copy-en="Support" data-copy-fr="Assistance">Support</a></div>
    </div>
  </footer>
  <script src="/release.js" defer></script>
  <script src="/i18n.js" defer></script>
</body>
</html>`;
}

function activeNavHrefForSlug(slug) {
  if (["download", "release-notes", "verify"].includes(slug)) return "/download/";
  if (["docs", "architecture", "source"].includes(slug)) return "/docs/";
  if (slug === "security") return "/security/";
  if (slug === "support") return "/support/";
  return null;
}

export function renderProductHeader(
  activeHref = null,
  { logoSrc = "/logo.svg" } = {}
) {
  const links = productNav
    .map(([href, en, fr]) => {
      const active = href === activeHref;
      const classes = active
        ? "bg-surface-container text-on-surface"
        : "text-on-surface-variant hover:bg-surface-container";
      const current = active ? ' aria-current="page"' : "";

      return `<a href="${href}"${current} class="px-3 py-2 rounded-full text-sm font-bold transition-colors ${classes}" data-copy-en="${escapeAttr(en)}" data-copy-fr="${escapeAttr(fr)}">${escapeHtml(en)}</a>`;
    })
    .join("");

  return `<header data-product-header class="sticky top-0 z-40 border-b border-outline-variant/40 bg-surface/95 backdrop-blur">
    <div class="mx-auto max-w-[1600px] px-5 md:px-8 h-20 flex items-center gap-6">
      <a href="/" class="flex items-center gap-3 shrink-0" aria-label="Sims Mod Health">
        <img src="${escapeAttr(logoSrc)}" alt="" class="w-9 h-9" />
        <div>
          <strong class="block text-sm">Sims Mod Health</strong>
          <small class="block text-[10px] font-bold text-primary uppercase tracking-wide" data-copy-en="Offline desktop engine" data-copy-fr="Moteur desktop hors ligne">Offline desktop engine</small>
        </div>
      </a>
      <nav class="hidden lg:flex items-center gap-1 ml-auto" aria-label="Primary">
        ${links}
      </nav>
      ${languageSwitch()}
    </div>
  </header>`;
}

function languageSwitch() {
  return `<div data-lang-switch class="inline-flex items-center rounded-full bg-surface-container-low p-1 border border-outline-variant/40" aria-label="Choose language"><button type="button" data-lang="en" aria-pressed="true" class="px-2.5 py-1.5 rounded-full text-xs font-extrabold transition-colors">EN</button><button type="button" data-lang="fr" aria-pressed="false" class="px-2.5 py-1.5 rounded-full text-xs font-extrabold transition-colors">FR</button></div>`;
}

function card(en, fr, rows) {
  return `<section class="rounded-3xl border border-outline-variant/50 bg-surface-container-lowest p-7"><h2 class="text-xl font-extrabold mb-5" data-copy-en="${escapeAttr(en)}" data-copy-fr="${escapeAttr(fr)}">${escapeHtml(en)}</h2><ul class="space-y-4">${rows.map(([a,b])=>`<li class="flex gap-3 text-on-surface-variant"><span class="material-symbols-outlined text-primary text-lg">check_circle</span><span data-copy-en="${escapeAttr(a)}" data-copy-fr="${escapeAttr(b)}">${escapeHtml(a)}</span></li>`).join("")}</ul></section>`;
}

function miniCard(en, fr, bodyEn, bodyFr) {
  return `<article class="rounded-3xl border border-outline-variant/50 bg-surface-container-lowest p-7"><div class="w-10 h-10 rounded-2xl bg-primary-container text-on-primary-container grid place-items-center mb-5"><span class="material-symbols-outlined">verified_user</span></div><h2 class="text-lg font-extrabold mb-2" data-copy-en="${escapeAttr(en)}" data-copy-fr="${escapeAttr(fr)}">${escapeHtml(en)}</h2><p class="text-sm leading-6 text-on-surface-variant" data-copy-en="${escapeAttr(bodyEn)}" data-copy-fr="${escapeAttr(bodyFr)}">${escapeHtml(bodyEn)}</p></article>`;
}

function linkCard(href, en, fr, bodyEn, bodyFr) {
  return `<a href="${href}" class="block rounded-3xl border border-outline-variant/50 bg-surface-container-lowest p-7 hover:shadow-md transition"><span class="material-symbols-outlined text-primary mb-5">arrow_outward</span><h2 class="text-lg font-extrabold mb-2" data-copy-en="${escapeAttr(en)}" data-copy-fr="${escapeAttr(fr)}">${escapeHtml(en)}</h2><p class="text-sm leading-6 text-on-surface-variant" data-copy-en="${escapeAttr(bodyEn)}" data-copy-fr="${escapeAttr(bodyFr)}">${escapeHtml(bodyEn)}</p></a>`;
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (char) => ({ "&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#039;" })[char]);
}
function escapeAttr(value) { return escapeHtml(value); }
