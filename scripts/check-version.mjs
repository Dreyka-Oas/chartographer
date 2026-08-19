/**
 * Les trois fichiers qui portent un numéro de version doivent dire la même
 * chose : `package.json`, `src-tauri/tauri.conf.json` et `src-tauri/Cargo.toml`.
 *
 * Ce n'est pas de la cosmétique. La version installée que l'application compare
 * à celle de la release vient de `tauri.conf.json` ; le tag Git et le nom des
 * installeurs, du même endroit. Un écart entre ces fichiers passe inaperçu
 * jusqu'au jour où une mise à jour se propose en boucle, ou ne se propose
 * jamais.
 *
 * Lancé sans argument, le script compare. Avec un numéro, il l'écrit partout.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const PACKAGE = join(root, "package.json");
const TAURI = join(root, "src-tauri", "tauri.conf.json");
const CARGO = join(root, "src-tauri", "Cargo.toml");

/** Version d'un fichier JSON, lue sur sa clé `version` de premier niveau. */
function fromJson(path) {
  return JSON.parse(readFileSync(path, "utf8")).version;
}

/**
 * Version du manifeste Cargo, lue dans la seule section `[package]`. La
 * recherche s'arrête à la section suivante : une dépendance porte elle aussi
 * une clé `version`, et la première trouvée ne serait pas la bonne.
 */
function fromCargo(path) {
  const body = readFileSync(path, "utf8");
  const start = body.indexOf("[package]");
  if (start === -1) return null;
  const rest = body.slice(start + "[package]".length);
  const end = rest.indexOf("\n[");
  const section = end === -1 ? rest : rest.slice(0, end);
  return section.match(/^\s*version\s*=\s*"([^"]+)"/m)?.[1] ?? null;
}

/** Réécrit la version dans un fichier, sans toucher au reste de sa mise en
 * forme : un `JSON.parse` suivi d'un `stringify` reformaterait tout le fichier. */
function replace(path, pattern, next) {
  const body = readFileSync(path, "utf8");
  const updated = body.replace(pattern, next);
  if (updated === body) throw new Error(`version introuvable dans ${path}`);
  writeFileSync(path, updated);
}

const wanted = process.argv[2] ?? null;

if (wanted !== null) {
  if (!/^\d+\.\d+\.\d+$/.test(wanted)) {
    console.error(`Version attendue sous la forme 1.2.3, reçu : ${wanted}`);
    process.exit(2);
  }
  replace(PACKAGE, /("version"\s*:\s*")[^"]+(")/, `$1${wanted}$2`);
  replace(TAURI, /("version"\s*:\s*")[^"]+(")/, `$1${wanted}$2`);
  replace(CARGO, /(\[package\][\s\S]*?\n\s*version\s*=\s*")[^"]+(")/, `$1${wanted}$2`);
  console.log(`Version portée à ${wanted}. Reste à poser le tag v${wanted}.`);
  process.exit(0);
}

const found = {
  "package.json": fromJson(PACKAGE),
  "src-tauri/tauri.conf.json": fromJson(TAURI),
  "src-tauri/Cargo.toml": fromCargo(CARGO),
};

const versions = new Set(Object.values(found));
if (versions.size !== 1 || versions.has(null) || versions.has(undefined)) {
  console.error("Les versions ne concordent pas :");
  for (const [file, version] of Object.entries(found)) {
    console.error(`  ${file} : ${version ?? "absente"}`);
  }
  console.error("Corrige avec : node scripts/check-version.mjs <version>");
  process.exit(1);
}

console.log(`Version ${[...versions][0]}, identique dans les trois fichiers.`);
