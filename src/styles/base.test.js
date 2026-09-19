// @vitest-environment jsdom
import { expect, test } from "vitest";
import { readFileSync } from "node:fs";
const css = readFileSync("src/styles/base.css", "utf8");

function rules() {
  const element = document.createElement("style"); element.textContent = css; document.head.append(element);
  const rules = [...element.sheet.cssRules].filter(rule => "selectorText" in rule);
  element.remove(); return rules;
}
function luminance(hex) {
  const full = hex.length === 4 ? [...hex.slice(1)].map(digit => digit + digit).join("") : hex.slice(1);
  return [0.2126, 0.7152, 0.0722].reduce((total, weight, index) => {
    const channel = parseInt(full.slice(index * 2, index * 2 + 2), 16) / 255;
    return total + weight * (channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4);
  }, 0);
}

test("primary hover explicitly retains a readable palette in dark and light themes", () => {
  const styles = rules(); const hover = styles.find(rule => rule.selectorText.split(",").some(selector => selector.trim() === "button.primary:hover:enabled"));
  expect(hover, "primary hover needs its own palette above the generic hover rule").toBeDefined();
  expect(hover.style.background).toBe("var(--accent)"); expect(hover.style.color).toBe("var(--accent-text)");
  const dark = styles.find(rule => rule.selectorText === ":root");
  const light = styles.find(rule => rule.selectorText === ':root[data-theme="light"]');
  for (const theme of [dark, light]) {
    const background = luminance(theme.style.getPropertyValue("--accent").trim());
    const foreground = luminance(theme.style.getPropertyValue("--accent-text").trim());
    expect((Math.max(background, foreground) + 0.05) / (Math.min(background, foreground) + 0.05)).toBeGreaterThanOrEqual(4.5);
  }
});

test("button hover keeps non-primary styling, disabled opacity and visible focus", () => {
  const styles = rules();
  expect(styles.find(rule => rule.selectorText === "button:hover:enabled").style.background).toBe("var(--raised)");
  expect(Number(styles.find(rule => rule.selectorText === "button:disabled").style.opacity)).toBe(0.5);
  const focus = styles.find(rule => rule.selectorText === ":focus-visible");
  expect(focus.style.outline).toBe("2px solid var(--focus)"); expect(focus.style.getPropertyValue("outline-offset")).toBe("3px");
});

test("light and dark text and focus tokens remain readable across application surfaces", () => {
  const styles = rules();
  const palettes = [styles.find(rule => rule.selectorText === ":root"), styles.find(rule => rule.selectorText === ':root[data-theme="light"]')];
  for (const palette of palettes) {
    for (const surface of ["--bg", "--surface", "--raised", "--danger-bg"]) {
      const background = luminance(palette.style.getPropertyValue(surface).trim());
      for (const token of ["--text", "--muted", "--focus"]) {
        const foreground = luminance(palette.style.getPropertyValue(token).trim());
        expect((Math.max(background, foreground) + 0.05) / (Math.min(background, foreground) + 0.05), `${surface}/${token}`).toBeGreaterThanOrEqual(token === "--focus" ? 3 : 4.5);
      }
    }
  }
});
