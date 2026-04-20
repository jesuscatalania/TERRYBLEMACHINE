#!/usr/bin/env node
/**
 * URL analyzer Node sidecar.
 *
 * Spawned by the Rust backend (`website_analyzer::PlaywrightUrlAnalyzer`)
 * to render an arbitrary URL in headless Chromium and extract enough
 * style + layout + content signal for the code generator to mirror
 * ("copy this site 1:1" prompts).
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
 *
 * Auto-screenshot: even when `--screenshot` is not provided the analyzer
 * saves a full-page PNG under `os.tmpdir()/tm-analyze-<uuid>/screenshot.png`
 * so the code generator always has visual context to reference.
 *
 * Verified against example.com: new fields present —
 *   hero_text, nav_items, section_headings, paragraph_sample, cta_labels,
 *   detected_features (has_canvas/has_video/has_form/has_iframe/has_webgl/
 *   has_three_js), typography, image_urls, color_roles (bg/fg/accent).
 */

import crypto from "node:crypto";
import { promises as fs } from "node:fs";
import os from "node:os";
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
    // round-trips. Returns an object with all the fields we want. Keeping
    // this in one call avoids Playwright's per-eval serialization overhead
    // — the return shape is already shallow (strings/bools/small arrays).
    const extracted = await page.evaluate(() => {
      const all = document.querySelectorAll("*");

      // ── Dominant colors — scan all elements' background-color + color ─
      const colorCounts = new Map();
      function bumpColor(k) {
        if (!k || k === "rgba(0, 0, 0, 0)" || k === "transparent") return;
        colorCounts.set(k, (colorCounts.get(k) || 0) + 1);
      }
      for (const el of all) {
        const cs = getComputedStyle(el);
        bumpColor(cs.backgroundColor);
        bumpColor(cs.color);
      }
      const sortedColors = [...colorCounts.entries()]
        .sort((a, b) => b[1] - a[1])
        .slice(0, 8)
        .map(([k]) => k);

      // ── Fonts ───────────────────────────────────────────────────────
      const fontSet = new Set();
      for (const el of all) {
        const cs = getComputedStyle(el);
        const fam = (cs.fontFamily || "").split(",")[0]?.trim().replace(/"/g, "");
        if (fam) fontSet.add(fam);
      }
      const fonts = [...fontSet].slice(0, 8);

      // ── Spacing values (common padding/margin) ──────────────────────
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

      // ── Layout — crude: count display:grid vs display:flex usage ────
      let grid = 0;
      let flex = 0;
      for (const el of all) {
        const d = getComputedStyle(el).display;
        if (d.includes("grid")) grid++;
        else if (d.includes("flex")) flex++;
      }
      const layout = grid > flex ? "grid" : flex > 0 ? "flex" : "other";

      // ── CSS custom properties on :root ──────────────────────────────
      const rootStyle = getComputedStyle(document.documentElement);
      const customProps = {};
      for (let i = 0; i < rootStyle.length; i++) {
        const name = rootStyle.item(i);
        if (name?.startsWith("--")) {
          customProps[name] = rootStyle.getPropertyValue(name).trim();
        }
      }

      const title = document.title || "";
      const description =
        document.querySelector('meta[name="description"]')?.getAttribute("content") || null;

      // ── Content signals ─────────────────────────────────────────────
      const trim = (s, cap) => (s || "").trim().slice(0, cap);

      const h1 = document.querySelector("h1");
      const hero_text = h1 ? trim(h1.textContent, 200) : "";

      // Nav links — prefer <nav>, fall back to <header>.
      const navAnchors = document.querySelectorAll("nav a, header a");
      const nav_items = [];
      const seenNav = new Set();
      for (const a of navAnchors) {
        const t = trim(a.textContent, 60);
        if (!t || seenNav.has(t)) continue;
        seenNav.add(t);
        nav_items.push(t);
        if (nav_items.length >= 8) break;
      }

      const section_headings = [];
      for (const h of document.querySelectorAll("h2")) {
        const t = trim(h.textContent, 140);
        if (t) section_headings.push(t);
        if (section_headings.length >= 12) break;
      }

      // Paragraph sample — first 2-3 visible <p> with real text.
      const paragraph_sample = [];
      for (const p of document.querySelectorAll("p")) {
        const cs = getComputedStyle(p);
        if (cs.display === "none" || cs.visibility === "hidden") continue;
        const t = trim(p.textContent, 300);
        if (t.length >= 20) paragraph_sample.push(t);
        if (paragraph_sample.length >= 3) break;
      }

      // CTA labels — <button>, role=button, or <a class*=button>.
      const cta_labels = [];
      const seenCta = new Set();
      const ctaNodes = document.querySelectorAll(
        'button, [role="button"], a[class*="button" i], a[class*="btn" i], a[class*="cta" i]',
      );
      for (const el of ctaNodes) {
        const t = trim(el.textContent, 80);
        if (!t || seenCta.has(t)) continue;
        seenCta.add(t);
        cta_labels.push(t);
        if (cta_labels.length >= 6) break;
      }

      // ── Feature detection ───────────────────────────────────────────
      const canvas = document.querySelector("canvas");
      let has_webgl = false;
      if (canvas) {
        try {
          has_webgl =
            !!canvas.getContext("webgl") ||
            !!canvas.getContext("experimental-webgl") ||
            !!canvas.getContext("webgl2");
        } catch {
          has_webgl = false;
        }
      }
      const detected_features = {
        has_canvas: !!canvas,
        has_video: !!document.querySelector("video"),
        has_form: !!document.querySelector("form"),
        has_iframe: !!document.querySelector("iframe"),
        has_webgl,
        has_three_js: typeof window.THREE !== "undefined",
      };

      // ── Typography hierarchy — most-used (size, weight, family) ─────
      const typoCounts = new Map();
      for (const el of all) {
        // Only count elements that actually render text.
        const textLen = (el.textContent || "").trim().length;
        if (textLen < 3) continue;
        const cs = getComputedStyle(el);
        const size = cs.fontSize;
        const weight = cs.fontWeight;
        const family = (cs.fontFamily || "").split(",")[0]?.trim().replace(/"/g, "") || "";
        if (!size || !weight || !family) continue;
        const key = `${size}|${weight}|${family}`;
        typoCounts.set(key, (typoCounts.get(key) || 0) + 1);
      }
      const typography = [...typoCounts.entries()]
        .sort((a, b) => b[1] - a[1])
        .slice(0, 4)
        .map(([k]) => {
          const [size, weight, family] = k.split("|");
          return { size, weight, family };
        });

      // ── Image URLs (absolute) ───────────────────────────────────────
      const image_urls = [];
      const seenImg = new Set();
      for (const img of document.querySelectorAll("img[src]")) {
        const s = img.src;
        if (!s || seenImg.has(s)) continue;
        seenImg.add(s);
        image_urls.push(s);
        if (image_urls.length >= 12) break;
      }

      // ── Color roles (semantic) ──────────────────────────────────────
      const bodyCs = getComputedStyle(document.body || document.documentElement);
      const bg = bodyCs.backgroundColor || null;
      const fg = bodyCs.color || null;
      const skip = new Set([bg, fg, "rgba(0, 0, 0, 0)", "transparent"]);
      const accent = sortedColors.find((c) => !skip.has(c)) || null;
      const color_roles = { bg, fg, accent };

      return {
        title,
        description,
        colors: sortedColors,
        fonts,
        spacing: topSpacing,
        customProperties: customProps,
        layout,
        hero_text,
        nav_items,
        section_headings,
        paragraph_sample,
        cta_labels,
        detected_features,
        typography,
        image_urls,
        color_roles,
      };
    });

    // ── Screenshot ──────────────────────────────────────────────────────
    // Always take a full-page screenshot. If the caller provided an explicit
    // `--screenshot=<path>` we honour it; otherwise we save to a dedicated
    // temp directory so the code generator has a reference even for ad-hoc
    // analyses. Failures fall through to null so a single screenshot hiccup
    // doesn't kill the whole analysis.
    let screenshotPath = null;
    try {
      const target =
        screenshot ||
        path.join(os.tmpdir(), `tm-analyze-${crypto.randomUUID()}`, "screenshot.png");
      await fs.mkdir(path.dirname(target), { recursive: true });
      await page.screenshot({ path: target, fullPage: true });
      screenshotPath = target;
    } catch (err) {
      log("screenshot failed:", err?.message || String(err));
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
