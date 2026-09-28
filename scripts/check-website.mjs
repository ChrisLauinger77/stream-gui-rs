import { access, readFile } from "node:fs/promises";
import path from "node:path";

const root = path.resolve("website");
const publicUrl = "https://chrislauinger77.github.io/stream-gui-rs/";
const requiredFiles = [
  "index.html",
  "styles.css",
  "robots.txt",
  "sitemap.xml",
  "assets/app-icon.svg",
  "assets/app-screenshot.png",
];

await Promise.all(requiredFiles.map((file) => access(path.join(root, file))));

const html = await readFile(path.join(root, "index.html"), "utf8");
const robots = await readFile(path.join(root, "robots.txt"), "utf8");
const sitemap = await readFile(path.join(root, "sitemap.xml"), "utf8");
const errors = [];

for (const marker of [
  '<html lang="en">',
  '<meta name="viewport"',
  'name="description"',
  '<link rel="canonical"',
  '<main id="main">',
]) {
  if (!html.includes(marker)) errors.push(`index.html is missing ${marker}`);
}

if ((html.match(/<h1[\s>]/g) ?? []).length !== 1) {
  errors.push("index.html must contain exactly one h1");
}

if (!html.includes(publicUrl) || !robots.includes(publicUrl) || !sitemap.includes(publicUrl)) {
  errors.push("the public GitHub Pages URL must stay aligned across site metadata");
}

const localReferences = [...html.matchAll(/(?:href|src)="([^"]+)"/g)]
  .map((match) => match[1])
  .filter((reference) => !/^(?:[a-z]+:|#)/i.test(reference));

for (const reference of localReferences) {
  if (reference.startsWith("/")) {
    errors.push(`root-relative reference breaks project Pages hosting: ${reference}`);
    continue;
  }

  const resolved = path.resolve(root, reference);
  if (!resolved.startsWith(`${root}${path.sep}`)) {
    errors.push(`reference escapes the website directory: ${reference}`);
    continue;
  }

  try {
    await access(resolved);
  } catch {
    errors.push(`missing local reference: ${reference}`);
  }
}

if (errors.length > 0) {
  throw new Error(`Website check failed:\n- ${errors.join("\n- ")}`);
}

console.log(`Website check passed (${requiredFiles.length} required files, ${localReferences.length} local references).`);
