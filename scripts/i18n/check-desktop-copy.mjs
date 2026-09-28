import { readdir, readFile } from "node:fs/promises";
import { join, relative } from "node:path";
import ts from "typescript";

const root = new URL("../../src/", import.meta.url);
const monitoredAttributes = new Set([
  "aria-label",
  "placeholder",
  "title",
  "label",
  "detail"
]);

const allowlistedLiteralCopy = new Set([
  "Sims Mod Health",
  "THE SIMS 4",
  "SMH",
  "Windows",
  "EA app",
  "Steam",
  "EN",
  "FR"
]);

const failures = [];

for (const file of await walk(root)) {
  if (!file.pathname.endsWith(".tsx")) continue;
  if (file.pathname.endsWith(".test.tsx")) continue;
  if (file.pathname.includes("/i18n/")) continue;

  const sourceText = await readFile(file, "utf8");
  const sourceFile = ts.createSourceFile(
    file.pathname,
    sourceText,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX
  );

  visit(sourceFile, sourceFile, file);
}

if (failures.length > 0) {
  console.error("Desktop i18n guard found untranslated JSX copy:");
  for (const failure of failures) {
    console.error(
      `- ${failure.file}:${failure.line} ${JSON.stringify(failure.value)}`
    );
  }
  console.error(
    "Route user-facing copy through useI18n().t()/tx(), or explicitly document a justified brand/proper-noun allowlist entry."
  );
  process.exit(1);
}

console.log("Desktop i18n guard: no untranslated literal JSX copy found.");

function visit(node, sourceFile, file) {
  if (ts.isJsxText(node)) {
    const value = node.getText(sourceFile).replace(/\s+/g, " ").trim();
    if (isUserFacing(value) && !allowlistedLiteralCopy.has(value)) {
      addFailure(node, sourceFile, file, value);
    }
  }

  if (ts.isJsxAttribute(node)) {
    const name = node.name.getText(sourceFile);
    if (
      monitoredAttributes.has(name) &&
      node.initializer &&
      ts.isStringLiteral(node.initializer)
    ) {
      const value = node.initializer.text.trim();
      if (isUserFacing(value) && !allowlistedLiteralCopy.has(value)) {
        addFailure(node, sourceFile, file, `${name}=${value}`);
      }
    }
  }

  ts.forEachChild(node, (child) => visit(child, sourceFile, file));
}

function addFailure(node, sourceFile, file, value) {
  const { line } = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
  failures.push({
    file: relative(new URL("../..", import.meta.url).pathname, file.pathname),
    line: line + 1,
    value
  });
}

function isUserFacing(value) {
  return /[A-Za-zÀ-ÿ]/.test(value);
}

async function walk(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const child = new URL(entry.name + (entry.isDirectory() ? "/" : ""), directory);
    if (entry.isDirectory()) files.push(...(await walk(child)));
    else files.push(child);
  }

  return files;
}
