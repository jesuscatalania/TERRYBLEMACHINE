# App-Shell Design — Schritt 1.1

**Date:** 2026-04-16
**Scope:** App-Shell, Sidebar-Navigation, Header, Footer-Statusbar. Kein Modul-Inhalt, kein Routing-Verhalten.
**Mockup:** `/tmp/tm_mockup.png` (Playwright screenshot aus `/tmp/tm_mockup.html`)

## Design Direction

**Aesthetik:** Industrial / Schematic (Option D aus Brainstorm)
- Scharfe Ecken (max 3px radius für strukturelle Elemente)
- 1px dezente Divider (`#2a2a30`)
- Feine Koordinaten/Schema-Deko: Frame-Brackets, `FIG 01 — READY` Frame-Labels, `/ 01`-Indizes, `MOD—01` Tags
- Schematisches 40×40px Background-Grid mit radialer Maske (verblasst zu Rändern)

**Typografie** (Option B): Inter body + IBM Plex Mono labels
- Body: Inter 13px
- Display (H1): Inter Bold, `-0.02em` tracking, 28px
- Labels / Data / Wordmark: IBM Plex Mono, small-caps (`uppercase` + `letter-spacing: 0.08-0.14em`), 10-11px
- Numerals tabular (`font-variant-numeric: tabular-nums`)

**Farben** (Option A + Dark default):
- BG: `#0e0e11` (neutral-dark-900)
- Surface 1: `#17171b` (neutral-dark-800) — active nav, hover
- Divider: `#2a2a30` → `#3a3a42` (hover)
- FG: `#f7f7f8` (primary) / `#a9a9b2` (secondary) / `#72727e` (meta) / `#4a4a52` (labels)
- Accent: `#e85d2d` (Safety Orange) — active indicator, primary CTA, render-bar, frame-brackets, wordmark mark
- Status: green (`#22c55e`) idle, amber (`#eab308`) warn, red (`#ef4444`) error

## Layout

CSS Grid shell:
```
grid-template-columns: 240px 1fr;  /* sidebar | rest */
grid-template-rows:    48px  1fr  28px;  /* header | main | footer */
```

### Sidebar (240 × 100%)
- Brand row (48px, bottom-divider)
  - 20×20 Logo-Mark: orange-bordered square with orange cross (plus-shape inside)
  - `TERRYBLEMACHINE` wordmark (Plex Mono 11.5px, tracking 0.12em)
- Section label "MODULES" (small-caps mono)
- 5 nav items (36px each, grid: `28px 1fr auto`):
  - icon (14×14, 1.5px stroke, line style)
  - label + mono `/ NN` index
  - keyboard shortcut pill (`⌘1`-`⌘5`, mono, bordered)
- Active: `border-left: 2px solid #e85d2d`, bg `#17171b`, orange index
- Section label "PROJECT" then 1 entry (active project, `⌘O`)
- Bottom strip (top-divider): version `v0.1.0` left, collapse button right

**Collapsible state** (defer to sub-step if needed): collapse button toggles sidebar to ~56px icon rail (icons only, no labels). For 1.1 we implement the full sidebar; collapse target is a follow-on ticket. **Note:** implementation of this spec covers the full-width sidebar plus the collapse toggle; the 56px collapsed rail itself is deferred.

### Header (48px)
- Left: breadcrumb chain in mono small-caps, `/` separators, current highlighted
- Right: `NEW` button (secondary/outlined, 28px), `GENERATE` button (primary orange), settings icon-button (28×28)
- All buttons: mono uppercase 11px, 1px border, 3px radius

### Main
- Fills remaining space, overflow hidden
- Background: 40×40px grid lines (`#2a2a30`, opacity 0.22) with radial mask centered at 50% 40%
- Empty-state content centered: schematic frame (520px max-width, 1px border) with:
  - Orange 10×10 corner brackets (4 corners, L-shaped)
  - Top-left frame-label `FIG 01 — READY` in mono 10px
  - Top-right frame-tag in accent orange (module code, e.g. `MOD—01`)
  - H1, body text (with inline orange mono highlight for `meingeschmack/`)
  - 3-button CTA row

### Footer status bar (28px)
- Left group: AI-status (dot + `AI · IDLE|ACTIVE|ERROR`), `CACHE X/Y`, `BUDGET $X / $Y`
- Right group: `QUEUE N`, `RENDER [bar] %` (progress bar 80×4px with orange fill), `⌘K` hint
- All mono small-caps 10.5px, tabular nums, metrics in `#ebebee` (brighter)

## Component Breakdown

One file per component under `src/components/shell/`:

```
src/components/shell/
├── Shell.tsx            # Outer grid + routing glue
├── Sidebar.tsx          # Sidebar container + brand + sections
├── SidebarItem.tsx      # Single nav row (icon, label, index, kbd)
├── ModuleIcon.tsx       # Switch over ModuleId → SVG
├── Header.tsx           # Breadcrumbs + header actions
├── Breadcrumbs.tsx      # Mono breadcrumb chain
├── StatusBar.tsx        # Footer status bar
├── StatusDot.tsx        # Colored dot with glow
└── SchematicFrame.tsx   # Reusable bracket frame (FIG/MOD labels)
```

Plus primitives that will be promoted to `src/components/ui/` in Schritt 1.2:
- `Button` (primary / secondary / icon variants)
- `Kbd` (keyboard-shortcut pill)

## State

- `useAppStore.activeModule` already exists — Sidebar reads + sets
- Sidebar collapsed state: add `sidebarOpen` already in store; default `true` (open)
- Header breadcrumb: derives from `activeModule` + `useProjectStore.currentProject`
- Status bar: reads from `useAiStore` (budget, cache stats, activeRequests for queue) and a new `progress` signal (can be stub for now)

## Testing (TDD)

Per-component tests under `*.test.tsx` covering:
- `Shell` renders sidebar, header, main-children, footer
- `Sidebar` lists 5 modules with correct icons/indices/kbd hints
- `SidebarItem` active styling triggers when `active` prop true
- `Sidebar` click on item → `setActiveModule` called
- `Breadcrumbs` reflects `activeModule` change
- `StatusBar` shows dot colors per status, formats currency/cache correctly
- `SchematicFrame` renders FIG/MOD labels when provided

## YAGNI / deferred

- Sidebar collapsed-rail UI (the 56px icon-only state) — implement after Schritt 1.3 routing
- Actual page-transition animations — Schritt 1.3
- Real budget/queue wiring to AI router — Schritt 2.x
- Module content placeholders beyond the empty state — Schritt 1.3

## Acceptance criteria

- All components render without errors in `pnpm tauri dev`
- Sidebar click switches `activeModule` in `useAppStore`
- `pnpm test` green with new component tests
- `pnpm biome check .` clean
- No inline styles (everything Tailwind utilities)
- Dark mode works, Safety Orange accent visible only where design spec shows it
