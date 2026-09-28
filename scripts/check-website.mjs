import { access, readFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

const root = path.resolve("website");
export const PUBLIC_URL = "https://chrislauinger77.github.io/stream-gui-rs/";
const requiredFiles = [
  "index.html",
  "styles.css",
  "robots.txt",
  "sitemap.xml",
  "assets/app-icon.svg",
  "assets/app-screenshot.svg",
];

export function metadataUrlErrors({ html, robots, sitemap }) {
  const canonicalUrls = [...html.matchAll(/<link\s+rel="canonical"\s+href="([^"]+)"\s*\/?>/g)].map(
    (match) => match[1],
  );
  const openGraphUrls = [...html.matchAll(/<meta\s+property="og:url"\s+content="([^"]+)"\s*\/?>/g)].map(
    (match) => match[1],
  );
  const robotsSitemaps = [...robots.matchAll(/^Sitemap:\s*(\S+)\s*$/gm)].map((match) => match[1]);
  const sitemapUrls = [...sitemap.matchAll(/<loc>\s*([^<\s]+)\s*<\/loc>/g)].map((match) => match[1]);
  const expectedSitemapUrl = `${PUBLIC_URL}sitemap.xml`;
  const errors = [];

  if (canonicalUrls.length !== 1 || canonicalUrls[0] !== PUBLIC_URL) {
    errors.push(`canonical URL must be exactly ${PUBLIC_URL}`);
  }
  if (openGraphUrls.length !== 1 || openGraphUrls[0] !== PUBLIC_URL) {
    errors.push(`Open Graph URL must be exactly ${PUBLIC_URL}`);
  }
  if (robotsSitemaps.length !== 1 || robotsSitemaps[0] !== expectedSitemapUrl) {
    errors.push(`robots.txt sitemap URL must be exactly ${expectedSitemapUrl}`);
  }
  if (sitemapUrls.length !== 1 || sitemapUrls[0] !== PUBLIC_URL) {
    errors.push(`sitemap location must be exactly ${PUBLIC_URL}`);
  }

  return errors;
}

async function checkWebsite() {
  await Promise.all(requiredFiles.map((file) => access(path.join(root, file))));

  const html = await readFile(path.join(root, "index.html"), "utf8");
  const robots = await readFile(path.join(root, "robots.txt"), "utf8");
  const sitemap = await readFile(path.join(root, "sitemap.xml"), "utf8");
  const errors = metadataUrlErrors({ html, robots, sitemap });

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

  console.log(
    `Website check passed (${requiredFiles.length} required files, ${localReferences.length} local references).`,
  );
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;
if (isMain) await checkWebsite();
