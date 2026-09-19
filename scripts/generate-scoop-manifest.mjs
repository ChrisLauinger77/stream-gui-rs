#!/usr/bin/env node

import { createHash } from "node:crypto";
import { basename } from "node:path";
import { readFileSync, writeFileSync } from "node:fs";

const [version, archivePath, outputPath] = process.argv.slice(2);

if (!version || !archivePath || !outputPath) {
  console.error(
    "Usage: generate-scoop-manifest.mjs VERSION PORTABLE_ZIP OUTPUT_JSON",
  );
  process.exit(2);
}

if (!/^\d+\.\d+\.\d+$/.test(version)) {
  throw new Error("Version must use numeric major.minor.patch form.");
}

const archiveName = basename(archivePath);
const expectedName = `Stream-GUI-RS_${version}_windows_x86_64.zip`;
if (archiveName !== expectedName) {
  throw new Error(`Expected portable archive name ${expectedName}.`);
}

const hash = createHash("sha256").update(readFileSync(archivePath)).digest("hex");
const url =
  `https://github.com/ChrisLauinger77/stream-gui-rs/releases/download/` +
  `v${version}/${archiveName}`;
const manifest = {
  version,
  description: "Browse Twitch and watch streams through Streamlink",
  homepage: "https://github.com/ChrisLauinger77/stream-gui-rs",
  license: "GPL-3.0-only",
  architecture: {
    "64bit": {
      url,
      hash,
    },
  },
  shortcuts: [["stream-gui-rs.exe", "Stream GUI RS"]],
  notes: [
    "Install Streamlink 8.0+ and a compatible media player separately.",
  ],
  checkver: "github",
  autoupdate: {
    architecture: {
      "64bit": {
        url:
          "https://github.com/ChrisLauinger77/stream-gui-rs/releases/download/" +
          "v$version/Stream-GUI-RS_$version_windows_x86_64.zip",
      },
    },
  },
};

writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`);
