import { readFile } from "node:fs/promises";

const capabilityPath = new URL("../../src-tauri/capabilities/default.json", import.meta.url);
const configPath = new URL("../../src-tauri/tauri.conf.json", import.meta.url);

const capability = JSON.parse(await readFile(capabilityPath, "utf8"));
const config = JSON.parse(await readFile(configPath, "utf8"));

const permissions = capability.permissions ?? [];
const expectedPermissions = ["core:default"];

if (JSON.stringify(permissions) !== JSON.stringify(expectedPermissions)) {
  throw new Error(
    "WebView capability drift detected. Expected only core:default, got: " +
      JSON.stringify(permissions)
  );
}

const broadPermissionPrefixes = [
  "fs:",
  "shell:",
  "http:",
  "process:",
  "upload:",
  "dialog:allow-open"
];

for (const permission of permissions) {
  if (broadPermissionPrefixes.some((prefix) => permission.startsWith(prefix))) {
    throw new Error("Broad WebView permission is forbidden: " + permission);
  }
}

const csp = config?.app?.security?.csp;
if (typeof csp !== "string" || csp.trim() === "") {
  throw new Error("Production CSP must be explicit and non-empty.");
}

const requiredDirectives = [
  "default-src 'self'",
  "connect-src ipc: http://ipc.localhost",
  "script-src 'self'",
  "object-src 'none'",
  "base-uri 'none'",
  "frame-ancestors 'none'"
];

for (const directive of requiredDirectives) {
  if (!csp.includes(directive)) {
    throw new Error("Required CSP directive missing: " + directive);
  }
}

if (csp.includes("'unsafe-eval'")) {
  throw new Error("CSP must not allow unsafe-eval.");
}

const scriptDirective = csp
  .split(";")
  .map((part) => part.trim())
  .find((part) => part.startsWith("script-src "));

if (!scriptDirective || scriptDirective.includes("'unsafe-inline'") || scriptDirective.includes("*")) {
  throw new Error("script-src must stay self-only without unsafe-inline or wildcard.");
}

console.log("Tauri capability and CSP audit passed.");
