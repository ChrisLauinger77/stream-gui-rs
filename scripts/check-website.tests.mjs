import assert from "node:assert/strict";
import test from "node:test";

import { metadataUrlErrors, PUBLIC_URL } from "./check-website.mjs";

const validSources = {
  html: `<link rel="canonical" href="${PUBLIC_URL}" /><meta property="og:url" content="${PUBLIC_URL}" />`,
  robots: `User-agent: *\nSitemap: ${PUBLIC_URL}sitemap.xml\n`,
  sitemap: `<urlset><url><loc>${PUBLIC_URL}</loc></url></urlset>`,
};

test("website metadata accepts the exact public URLs", () => {
  assert.deepEqual(metadataUrlErrors(validSources), []);
});

for (const [name, sources] of Object.entries({
  "canonical URL with a hostile prefix": {
    ...validSources,
    html: validSources.html.replace(`href="${PUBLIC_URL}"`, `href="https://example.com/${PUBLIC_URL}"`),
  },
  "Open Graph URL with a hostile suffix": {
    ...validSources,
    html: validSources.html.replace(`content="${PUBLIC_URL}"`, `content="${PUBLIC_URL}.example.com/"`),
  },
  "robots sitemap URL with a hostile prefix": {
    ...validSources,
    robots: `Sitemap: https://example.com/${PUBLIC_URL}sitemap.xml\n`,
  },
  "sitemap location with a hostile suffix": {
    ...validSources,
    sitemap: `<urlset><url><loc>${PUBLIC_URL}.example.com/</loc></url></urlset>`,
  },
})) {
  test(`website metadata rejects ${name}`, () => {
    assert.notDeepEqual(metadataUrlErrors(sources), []);
  });
}
