#!/usr/bin/env node

import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const args = process.argv.slice(2);
const checkOnly = args[0] === "--check";
const version = checkOnly ? args[1] : args[0];

if (!version || args.length !== (checkOnly ? 2 : 1)) {
  console.error("Usage: set-version.mjs [--check] MAJOR.MINOR.PATCH");
  process.exit(2);
}

if (!/^\d+\.\d+\.\d+$/.test(version)) {
  throw new Error("Version must use numeric major.minor.patch form.");
}

const root = process.cwd();
const paths = {
  packageJson: resolve(root, "package.json"),
  packageLock: resolve(root, "package-lock.json"),
  cargoToml: resolve(root, "src-tauri/Cargo.toml"),
  cargoLock: resolve(root, "src-tauri/Cargo.lock"),
  tauriConfig: resolve(root, "src-tauri/tauri.conf.json"),
  changelog: resolve(root, "CHANGELOG.md"),
};

const packageJson = JSON.parse(readFileSync(paths.packageJson, "utf8"));
const packageLock = JSON.parse(readFileSync(paths.packageLock, "utf8"));
const tauriConfig = JSON.parse(readFileSync(paths.tauriConfig, "utf8"));
const cargoToml = readFileSync(paths.cargoToml, "utf8");
const cargoLock = readFileSync(paths.cargoLock, "utf8");
const changelog = readFileSync(paths.changelog, "utf8");

function sectionVersion(content, heading) {
  const start = content.indexOf(`${heading}\n`);
  if (start < 0) throw new Error(`Missing ${heading} section.`);
  const next = content.indexOf("\n[", start + heading.length + 1);
  const section = content.slice(start, next < 0 ? content.length : next);
  const match = section.match(/^version = "([^"]+)"$/m);
  if (!match) throw new Error(`Missing version in ${heading} section.`);
  return match[1];
}

function replaceSectionVersion(content, heading, nextVersion) {
  const current = sectionVersion(content, heading);
  const start = content.indexOf(`${heading}\n`);
  const next = content.indexOf("\n[", start + heading.length + 1);
  const end = next < 0 ? content.length : next;
  const section = content.slice(start, end);
  const updated = section.replace(
    `version = "${current}"`,
    `version = "${nextVersion}"`,
  );
  return `${content.slice(0, start)}${updated}${content.slice(end)}`;
}

const cargoLockPattern =
  /(\[\[package\]\]\nname = "stream-gui-rs"\nversion = ")[^"]+("\n)/;
const cargoLockMatch = cargoLock.match(cargoLockPattern);
if (!cargoLockMatch) {
  throw new Error("Missing stream-gui-rs package version in Cargo.lock.");
}

const versions = new Map([
  ["package.json", packageJson.version],
  ["package-lock.json", packageLock.version],
  ["package-lock.json root package", packageLock.packages?.[""]?.version],
  ["src-tauri/Cargo.toml", sectionVersion(cargoToml, "[package]")],
  ["src-tauri/Cargo.lock", cargoLockMatch[0].match(/version = "([^"]+)"/)[1]],
  ["src-tauri/tauri.conf.json", tauriConfig.version],
]);

for (const [file, value] of versions) {
  if (typeof value !== "string" || !/^\d+\.\d+\.\d+$/.test(value)) {
    throw new Error(`${file} has an invalid version.`);
  }
}

if (checkOnly) {
  for (const [file, value] of versions) {
    if (value !== version) {
      throw new Error(`${file} is ${value}; expected ${version}.`);
    }
  }
  const escaped = version.replaceAll(".", "\\.");
  if (!new RegExp(`^## ${escaped} - \\d{4}-\\d{2}-\\d{2}$`, "m").test(changelog)) {
    throw new Error(`CHANGELOG.md has no dated ${version} release heading.`);
  }
  console.log(`Release metadata consistently identifies ${version}.`);
  process.exit(0);
}

const distinctVersions = new Set(versions.values());
if (distinctVersions.size !== 1) {
  throw new Error(
    `Current versions disagree: ${[...versions].map(([file, value]) => `${file}=${value}`).join(", ")}`,
  );
}

const currentVersion = versions.values().next().value;
const numeric = (value) => value.split(".").map(Number);
const [current, next] = [numeric(currentVersion), numeric(version)];
const comparison = next.findIndex((part, index) => part !== current[index]);
if (comparison < 0) {
  throw new Error(`Version ${version} is already current.`);
}
if (next[comparison] < current[comparison]) {
  throw new Error(`Version ${version} must be newer than ${currentVersion}.`);
}

if (!/^## Unreleased$/m.test(changelog)) {
  throw new Error("CHANGELOG.md must contain a level-two Unreleased heading.");
}
if (new RegExp(`^## ${version.replaceAll(".", "\\.")} - `, "m").test(changelog)) {
  throw new Error(`CHANGELOG.md already contains ${version}.`);
}

packageJson.version = version;
packageLock.version = version;
packageLock.packages[""].version = version;
tauriConfig.version = version;

const today = new Date().toISOString().slice(0, 10);
const updatedChangelog = changelog.replace(
  /^## Unreleased$/m,
  `## Unreleased\n\n## ${version} - ${today}`,
);

writeFileSync(paths.packageJson, `${JSON.stringify(packageJson, null, 2)}\n`);
writeFileSync(paths.packageLock, `${JSON.stringify(packageLock, null, 2)}\n`);
writeFileSync(paths.tauriConfig, `${JSON.stringify(tauriConfig, null, 2)}\n`);
writeFileSync(
  paths.cargoToml,
  replaceSectionVersion(cargoToml, "[package]", version),
);
writeFileSync(
  paths.cargoLock,
  cargoLock.replace(cargoLockPattern, `$1${version}$2`),
);
writeFileSync(paths.changelog, updatedChangelog);

console.log(`Updated release metadata from ${currentVersion} to ${version}.`);
