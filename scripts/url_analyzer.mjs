#!/usr/bin/env node
/**
 * URL analyzer Node sidecar.
 *
 * Spawned by the Rust backend (`website_analyzer::PlaywrightUrlAnalyzer`)
 * to render an arbitrary URL in headless Chromium and extract enough
 * style + layout signal for the code generator to mirror.
 *
 * Usage:
 *   node scripts/url_analyzer.mjs <url> [--screenshot=<path>] [--assets-dir=<dir>]
 *
 * Emits a single JSON object on stdout. Any diagnostic logging goes to
 * stderr so the Rust caller can parse stdout cleanly.
 *
 * When `--assets-dir` is provided, referenced images / icons / @font-face
 * sources are downloaded into that directory (created if missing). Each
 * download is capped at a safe filename length; failures are swallowed so
 * a single broken asset never breaks the overall analysis.
 */

import { promises as fs } from "node:fs";
import path from "node:path";

import { chromium } from "playwright";

const MAX_ASSETS = 50;

function log(...args) {
  console.error("[url_analyzer]", ...args);
}

function parseArgs(argv) {
  const args = { url: null, screenshot: null, assetsDir: null };
  for (const a of argv.slice(2)) {
    if (!a.startsWith("--") && !args.url) {
      args.url = a;
    } else if (a.startsWith("--screenshot=")) {
      args.screenshot = a.slice("--screenshot=".length);
    } else if (a.startsWith("--assets-dir=")) {
      args.assetsDir = a.slice("--assets-dir=".length);
    }
  }
  return args;
}

/**
 * Derive a filesystem-safe filename for the given URL.
 * Strips the scheme, keeps only `[A-Za-z0-9._-]`, and clamps the
 * trailing 160 characters (preserving whatever extension the URL has).
 */
function safeFilename(u) {
  return u
    .replace(/^https?:\/\//, "")
    .replace(/[^a-z0-9._-]/gi, "_")
    .slice(-160);
}

async function downloadAssets(page, assetsDir) {
  await fs.mkdir(assetsDir, { recursive: true });

  const urls = await page.evaluate(() => {
    const out = new Set();
    document.querySelectorAll("img[src]").forEach((img) => {
      if (img.src) out.add(img.src);
    });
    document.querySelectorAll("link[rel~='icon'][href]").forEach((l) => {
      if (l.href) out.add(l.href);
    });
    // Font URLs from loaded @font-face resources.
    for (const sheet of Array.from(document.styleSheets)) {
      try {
        for (const rule of Array.from(sheet.cssRules || [])) {
          if (rule.constructor.name === "CSSFontFaceRule") {
            const src = rule.style?.getPropertyValue("src");
            if (src) {
              const matches = src.match(/url\(["']?([^"')]+)["']?\)/g) || [];
              for (const u of matches) {
                const cleaned = u.replace(/^url\(["']?|["']?\)$/g, "");
                if (cleaned) {
                  try {
                    out.add(new URL(cleaned, sheet.href || location.href).href);
                  } catch {
                    /* ignore malformed */
                  }
                }
              }
            }
          }
        }
      } catch {
        /* cross-origin stylesheet — cssRules throws; skip it */
      }
    }
    return Array.from(out);
  });

  const saved = [];
  for (const u of urls.slice(0, MAX_ASSETS)) {
    try {
      const r = await fetch(u);
      if (!r.ok) continue;
      const buf = Buffer.from(await r.arrayBuffer());
      const safe = safeFilename(u);
      if (!safe) continue;
      await fs.writeFile(path.join(assetsDir, safe), buf);
      saved.push({ url: u, saved_as: safe });
    } catch (err) {
      log("asset download failed:", u, err?.message || String(err));
    }
  }
  return saved;
}

async function main() {
  const { url, screenshot, assetsDir } = parseArgs(process.argv);
  if (!url) {
    console.error("usage: url_analyzer.mjs <url> [--screenshot=<path>] [--assets-dir=<dir>]");
    process.exit(2);
  }

  const browser = await chromium.launch({ headless: true });
  try {
    const context = await browser.newContext({
      viewport: { width: 1440, height: 900 },
      userAgent: "Mozilla/5.0 TERRYBLEMACHINE/0.1 (url analyzer; +https://localhost)",
    });
    const page = await context.newPage();
    log("navigating to", url);
    const resp = await page.goto(url, {
      waitUntil: "networkidle",
      timeout: 30_000,
    });
    const status = resp ? resp.status() : 0;

    // Extract everything we need in a single page.evaluate to minimise
    // round-trips. Returns an object with all the fields we want.
    const extracted = await page.evaluate(() => {
      // Dominant colors — scan all elements' background-color + color.
      const counts = new Map();
      function bump(k) {
        if (!k || k === "rgba(0, 0, 0, 0)" || k === "transparent") return;
        counts.set(k, (counts.get(k) || 0) + 1);
      }
      const all = document.querySelectorAll("*");
      for (const el of all) {
        const cs = getComputedStyle(el);
        bump(cs.backgroundColor);
        bump(cs.color);
      }
      const sortedColors = [...counts.entries()]
        .sort((a, b) => b[1] - a[1])
        .slice(0, 8)
        .map(([k]) => k);

      // Fonts
      const fontSet = new Set();
      for (const el of all) {
        const cs = getComputedStyle(el);
        const fam = (cs.fontFamily || "").split(",")[0]?.trim().replace(/"/g, "");
        if (fam) fontSet.add(fam);
      }
      const fonts = [...fontSet].slice(0, 8);

      // Spacing values (common padding/margin)
      const spacing = new Map();
      for (const el of all) {
        const cs = getComputedStyle(el);
        for (const prop of ["paddingTop", "paddingBottom", "marginTop", "marginBottom"]) {
          const v = cs[prop];
          if (v && v !== "0px") {
            spacing.set(v, (spacing.get(v) || 0) + 1);
          }
        }
      }
      const topSpacing = [...spacing.entries()]
        .sort((a, b) => b[1] - a[1])
        .slice(0, 8)
        .map(([k]) => k);

      // Layout — crude: count display:grid vs display:flex usage
      let grid = 0;
      let flex = 0;
      for (const el of all) {
        const d = getComputedStyle(el).display;
        if (d.includes("grid")) grid++;
        else if (d.includes("flex")) flex++;
      }
      const layout = grid > flex ? "grid" : flex > 0 ? "flex" : "other";

      // CSS custom properties on :root
      const rootStyle = getComputedStyle(document.documentElement);
      const customProps = {};
      // `rootStyle` is not iterable by default for custom props, but we
      // can scan through its length using item() for "--*" names.
      for (let i = 0; i < rootStyle.length; i++) {
        const name = rootStyle.item(i);
        if (name?.startsWith("--")) {
          customProps[name] = rootStyle.getPropertyValue(name).trim();
        }
      }

      const title = document.title || "";
      const description =
        document.querySelector('meta[name="description"]')?.getAttribute("content") || null;

      return {
        title,
        description,
        colors: sortedColors,
        fonts,
        spacing: topSpacing,
        customProperties: customProps,
        layout,
      };
    });

    let screenshotPath = null;
    if (screenshot) {
      await page.screenshot({ path: screenshot, fullPage: false });
      screenshotPath = screenshot;
    }

    let assets = [];
    if (assetsDir) {
      try {
        assets = await downloadAssets(page, assetsDir);
      } catch (err) {
        log("asset pipeline failed:", err?.message || String(err));
      }
    }

    const result = {
      url,
      status,
      ...extracted,
      screenshotPath,
      assets,
    };
    // Single JSON line on stdout — easy to parse.
    process.stdout.write(`${JSON.stringify(result)}\n`);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    log("analysis failed:", msg);
    process.stdout.write(`${JSON.stringify({ error: msg })}\n`);
    process.exitCode = 1;
  } finally {
    await browser.close();
  }
}

main().catch((err) => {
  log("fatal:", err);
  process.exit(1);
});
