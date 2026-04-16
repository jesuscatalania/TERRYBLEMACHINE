# TERRYBLEMACHINE — Entwicklung Schritt für Schritt

## Übersicht

Dieses Dokument enthält die vollständige Entwicklungsanleitung für TERRYBLEMACHINE — jeden Befehl, den du in Claude Code eingibst, und die Prüfung danach. Das Format folgt dem gleichen Muster wie CLAUDE-CODE-SETUP.md.

**Voraussetzung**: CLAUDE-CODE-SETUP.md ist abgeschlossen. Alle Tools, Skills und MCP-Server sind installiert.

### Arbeitsweise

Jeder Schritt folgt dem Superpowers-Workflow:
1. **Brainstorm** — Anforderungen klären
2. **Plan** — Tasks aufteilen
3. **Execute** — TDD: Test → Implementierung → Refactor
4. **Review** — Code prüfen
5. **Verify** — Alles läuft

**Alle Befehle werden innerhalb von Claude Code eingegeben**, nicht im normalen Terminal (außer es steht explizit dabei).

---

## PHASE 0: Foundation (Woche 1-2)

### Ziel
Projekt-Grundgerüst steht. `pnpm tauri dev` öffnet ein leeres Fenster. Tests laufen. CI/CD ist grün.

---

### SCHRITT 0.1: Tauri v2 Projekt initialisieren

```bash
# ── 0.1.1 Im Projektordner starten ──
cd /Users/enfantterryble/Documents/Projekte/TERRYBLEMACHINE
```

Innerhalb von Claude Code:
```
Initialisiere ein Tauri v2 Projekt mit React + TypeScript + Vite.
Nutze pnpm als Package Manager. Der App-Name ist "TERRYBLEMACHINE".
Verwende den rust-engineer und react-expert Skill.
```

**Prüfung:**
```bash
# Tauri Config existiert:
cat src-tauri/tauri.conf.json
# Erwartete Ausgabe: JSON mit "productName": "TERRYBLEMACHINE"

# Frontend existiert:
ls src/
# Erwartete Ausgabe: App.tsx, main.tsx, etc.

# Rust-Backend existiert:
cat src-tauri/src/main.rs
# Erwartete Ausgabe: Rust-Code mit #[cfg_attr(...)]
```

```bash
# ── 0.1.2 Erste Kompilierung testen ──
pnpm install && pnpm tauri dev
# Erwartung: Ein leeres Fenster öffnet sich
# Schließen mit Cmd+Q
```

**Wenn es fehlschlägt:**
```
Der Build schlägt fehl mit folgendem Fehler: [Fehlermeldung einfügen].
Bitte debugge das Problem. Nutze den systematic-debugging Skill.
```

---

### SCHRITT 0.2: Biome (Linter/Formatter) einrichten

Innerhalb von Claude Code:
```
Richte Biome als Linter und Formatter ein. Konfiguriere es für TypeScript + React.
Entferne ESLint/Prettier falls vorhanden. Erstelle eine biome.json mit:
- Strict TypeScript Rules
- React Hooks Rules
- Organize Imports
- Indentation: 2 Spaces
- Semicolons: immer
```

**Prüfung:**
```bash
pnpm biome check .
# Erwartete Ausgabe: Checked X files, no errors
```

```bash
# Testweise einen Fehler provozieren:
echo "const x = 1" >> src/test-lint.ts
pnpm biome check src/test-lint.ts
# Erwartete Ausgabe: Fehler (missing semicolon oder unused variable)
rm src/test-lint.ts
```

---

### SCHRITT 0.3: Vitest + Testing Library konfigurieren

Innerhalb von Claude Code:
```
Richte Vitest mit React Testing Library ein. Konfiguriere:
- jsdom als Test-Environment
- Coverage mit v8
- Setup-File für Testing Library Matchers
- Pfad-Aliases (@/ → src/)
Erstelle einen Beispieltest der prüft, dass die App rendert.
Nutze den test-driven-development Skill.
```

**Prüfung:**
```bash
pnpm test
# Erwartete Ausgabe: ✓ 1 test passed

pnpm test -- --coverage
# Erwartete Ausgabe: Coverage-Report wird angezeigt
```

---

### SCHRITT 0.4: Tailwind CSS konfigurieren

Innerhalb von Claude Code:
```
Richte Tailwind CSS ein mit einem Custom Theme für TERRYBLEMACHINE:
- Dark Mode als Default (class-basiert)
- Custom Farben (neutral-dark Palette, Akzentfarbe frei wählbar)
- Custom Fonts (System-Fonts als Fallback)
- Custom Spacing Scale
Keine separaten CSS-Dateien — alles über Tailwind Utilities.
```

**Prüfung:**
```bash
pnpm tauri dev
# Erwartung: Fenster zeigt dunklen Hintergrund (Dark Mode Default)
# Schließen mit Cmd+Q
```

```bash
cat tailwind.config.js
# Erwartete Ausgabe: darkMode: 'class', Custom Theme definiert
```

---

### SCHRITT 0.5: Zustand State Management einrichten

Innerhalb von Claude Code:
```
Richte Zustand als State Management ein. Erstelle die Grundstruktur:
- src/stores/appStore.ts (App-weiter State: Theme, Sidebar, aktives Modul)
- src/stores/projectStore.ts (Projekt-State: Name, Assets, Settings)
- src/stores/aiStore.ts (AI-State: Budget, Cache-Stats, aktive Requests)
- src/stores/uiStore.ts (UI-State: Modals, Notifications, Loading)
Jeweils mit TypeScript-Types und einem Beispieltest.
```

**Prüfung:**
```bash
pnpm test
# Erwartete Ausgabe: Alle Tests grün (inkl. Store-Tests)
```

```bash
# Store-Dateien existieren:
ls src/stores/
# Erwartete Ausgabe: appStore.ts, projectStore.ts, aiStore.ts, uiStore.ts
```

---

### SCHRITT 0.6: Git-Repository & CI/CD einrichten

Innerhalb von Claude Code:
```
Erstelle eine .gitignore für Tauri + React + Rust.
Erstelle eine GitHub Actions CI/CD Pipeline (.github/workflows/ci.yml):
- Trigger: Push auf main/develop, PRs
- Jobs parallel: Lint (Biome) → Test (Vitest) → Build (Tauri)
- macOS Runner (macos-latest)
- Rust-Cache + pnpm-Cache für Geschwindigkeit
```

**Prüfung:**
```bash
cat .gitignore
# Erwartete Ausgabe: node_modules/, target/, dist/, .DS_Store, etc.

cat .github/workflows/ci.yml
# Erwartete Ausgabe: YAML mit jobs: lint, test, build
```

```bash
# Initial Commit:
git add -A && git commit -m "feat(core): Phase 0 — Foundation Setup"
git push
```

```bash
# CI prüfen:
gh run list --limit 1
# Erwartete Ausgabe: ✓ workflow run completed
```

---

### SCHRITT 0.7: API-Key Management (macOS Keychain)

Innerhalb von Claude Code:
```
Erstelle ein Rust-Modul für API-Key Management über macOS Keychain.
Nutze das security-framework Crate. Implementiere:
- store_key(service: &str, key: &str) → Result<()>
- get_key(service: &str) → Result<String>
- delete_key(service: &str) → Result<()>
- list_keys() → Result<Vec<String>>
Tauri Commands für Frontend-Zugriff.
Environment-Variable Fallback für Development.
TDD: Tests zuerst, dann Implementierung.
Nutze den rust-engineer Skill.
```

**Prüfung:**
```bash
cargo test -p terryblemachine
# Erwartete Ausgabe: test keychain::tests ... ok
```

Innerhalb von Claude Code:
```
Teste den Keychain-Zugriff: Speichere einen Test-Key "test_service" mit Wert "test123",
lies ihn aus, und lösche ihn wieder. Zeige mir die Ausgaben.
```

---

### MEILENSTEIN Phase 0 — Gesamtprüfung

```bash
# Alles zusammen prüfen:
pnpm install && pnpm test && pnpm biome check . && pnpm tauri build
```

Erwartung:
- ✅ Dependencies installiert
- ✅ Alle Tests grün
- ✅ Linter findet keine Fehler
- ✅ Build erzeugt eine .app-Datei

```bash
# Build-Artefakt prüfen:
ls src-tauri/target/release/bundle/macos/
# Erwartete Ausgabe: TERRYBLEMACHINE.app
```

```bash
git add -A && git commit -m "feat(core): Phase 0 abgeschlossen — Foundation komplett"
git push
```

---

## PHASE 1: Core UI (Woche 3-5)

### Ziel
Die gesamte UI-Shell steht — Navigation, Theme, Layout — ohne Feature-Logik. App sieht professionell aus.

---

### SCHRITT 1.1: App-Shell & Sidebar-Navigation

Innerhalb von Claude Code:
```
Erstelle die App-Shell für TERRYBLEMACHINE mit dem frontend-design Skill.
Nutze den Superpowers brainstorming Skill, um zuerst das Design zu klären.

Anforderungen:
- Sidebar-Navigation links (collapsible) mit 5 Modulen:
  1. Website (Icon: Globe)
  2. 2D-Grafik & Bild (Icon: Image)
  3. Pseudo-3D (Icon: Box/Cube)
  4. Video (Icon: Film)
  5. Typografie & Logo (Icon: Type)
- Header: App-Name, aktives Projekt, Settings-Button
- Main Content Area: Flexibel, nimmt den Rest ein
- Footer: Status-Bar (AI-Status, Budget-Anzeige, Fortschritt)
- Dark Mode als Default
- Distinctive Ästhetik — kein generisches Admin-Dashboard
Lies vorher die docs/PROJEKTUEBERSICHT.md und meingeschmack/ für Kontext.
```

**Prüfung:**
```bash
pnpm tauri dev
# Erwartung: Professionelles UI mit Sidebar, Header, Content-Area, Footer
# Alle 5 Module in der Sidebar klickbar
# Dark Mode aktiv
```

```bash
pnpm test
# Erwartete Ausgabe: Alle Tests grün (inkl. Navigation-Tests)
```

---

### SCHRITT 1.2: Design-System (Komponenten-Bibliothek)

Innerhalb von Claude Code:
```
Erstelle ein Design-System für TERRYBLEMACHINE. Nutze den frontend-design Skill.

Wiederverwendbare Komponenten (src/components/):
- Button (Primary, Secondary, Ghost, Danger — alle Größen)
- Input (Text, Textarea mit Auto-Resize, Number)
- Card (mit Header, Body, Footer Slots)
- Modal (mit Backdrop, Animation)
- Dropdown (mit Search)
- Tabs (mit Icons)
- Toast/Notification (Success, Error, Warning, Info)
- Tooltip
- Skeleton (Loading-Placeholder)
- Badge

Alles mit:
- Tailwind CSS (keine separaten CSS-Dateien)
- Framer Motion für Animationen
- TypeScript Props mit JSDoc
- Storybook-artiger Preview (eine Demo-Route /design-system)
- Tests für jede Komponente
```

**Prüfung:**
```bash
pnpm tauri dev
# Navigiere zu /design-system → Alle Komponenten werden angezeigt
```

```bash
pnpm test
# Erwartete Ausgabe: Alle Komponenten-Tests grün
```

```bash
ls src/components/
# Erwartete Ausgabe: Button/, Input/, Card/, Modal/, Dropdown/, Tabs/, Toast/, etc.
```

---

### SCHRITT 1.3: Modul-Switcher & Routing

Innerhalb von Claude Code:
```
Implementiere das Modul-Routing:
- Sidebar-Klick wechselt das aktive Modul
- Jedes Modul hat eine eigene Route (/website, /graphic2d, /graphic3d, /video, /typography)
- Placeholder-Komponente pro Modul ("Coming soon: [Modul-Name]")
- Page-Transition-Animationen (Framer Motion, subtle fade/slide)
- Zustand appStore trackt aktives Modul
- URL-Sync: Browser-URL spiegelt aktives Modul
```

**Prüfung:**
```bash
pnpm tauri dev
# Klicke durch alle 5 Module → Smooth Transitions
# URL ändert sich: /website, /graphic2d, /graphic3d, /video, /typography
```

```bash
pnpm test
# Erwartete Ausgabe: Routing-Tests grün
```

---

### SCHRITT 1.4: Projekt-Management

Innerhalb von Claude Code:
```
Implementiere das Projekt-Management-System:
- "Neues Projekt" Dialog (Name, Typ/Modul, Beschreibung)
- Projekte werden als JSON auf Disk gespeichert (über Tauri FS API)
- Projekt öffnen / schließen
- Recents-Liste (letzte 10 Projekte)
- Projekt-Ordner-Struktur: ~/Documents/TERRYBLEMACHINE/projects/[name]/
- Zustand projectStore speichert aktives Projekt
TDD: Tests für Store und Tauri Commands.
Nutze rust-engineer für die Tauri-Commands.
```

**Prüfung:**
```bash
pnpm tauri dev
# 1. "Neues Projekt" erstellen → Dialog öffnet
# 2. Name eingeben, Typ wählen → Projekt wird erstellt
# 3. App schließen und neu öffnen → Projekt erscheint in Recents
```

```bash
pnpm test && cargo test -p terryblemachine
# Erwartete Ausgabe: Alle Tests grün (Frontend + Rust)
```

```bash
ls ~/Documents/TERRYBLEMACHINE/projects/
# Erwartete Ausgabe: Ordner für das erstellte Projekt
```

---

### SCHRITT 1.5: Input-System (universell)

Innerhalb von Claude Code:
```
Erstelle das universelle Input-System (wird von allen Modulen genutzt):
- Text-Input: Auto-Resize Textarea mit Prompt-History (letzte 20)
- Bild-Upload: Drag&Drop Zone + File Picker
  - Akzeptiert: PNG, JPG, WebP, SVG, TIFF, PSD (Preview)
  - Preview-Thumbnail nach Upload
  - Max-Größe Warnung (>50MB)
- URL-Input: Validation, Favicon-Preview, Auto-Detect
- Kombinierter Input: Text + optionaler Bild-Anhang (wie ChatGPT-Style)
- Input-History Dropdown
Tests für alle Input-Typen.
```

**Prüfung:**
```bash
pnpm tauri dev
# 1. Text eingeben → Auto-Resize funktioniert
# 2. Bild per Drag&Drop reinziehen → Preview wird angezeigt
# 3. URL eingeben → Wird validiert, Favicon erscheint
```

```bash
pnpm test
# Erwartete Ausgabe: Input-Komponenten-Tests grün
```

---

### SCHRITT 1.6: Undo/Redo System

Innerhalb von Claude Code:
```
Implementiere ein Undo/Redo System basierend auf dem Command Pattern:
- Jede Zustandsänderung wird als Command aufgezeichnet
- Cmd+Z für Undo, Cmd+Shift+Z für Redo
- History-Limit: 50 Steps
- Persistenz: History wird mit Projekt gespeichert
- Integration mit allen Zustand Stores
Tests für Command-Stack.
```

**Prüfung:**
```bash
pnpm tauri dev
# 1. Aktion ausführen → Cmd+Z → Aktion wird rückgängig gemacht
# 2. Cmd+Shift+Z → Aktion wird wiederhergestellt
```

```bash
pnpm test
# Erwartete Ausgabe: Undo/Redo-Tests grün
```

---

### MEILENSTEIN Phase 1 — Gesamtprüfung

```bash
pnpm test && pnpm biome check . && cargo test -p terryblemachine
```

Innerhalb von Claude Code:
```
Führe eine vollständige Verifikation von Phase 1 durch.
Nutze den verification-before-completion Skill.
Prüfe:
1. Navigation zwischen allen 5 Modulen funktioniert mit Transitions
2. Design-System Komponenten sind alle getestet
3. Projekte können erstellt, geöffnet, geschlossen werden
4. Input-System (Text, Bild, URL) funktioniert
5. Dark Mode ist durchgängig
6. Undo/Redo funktioniert
7. Keine Lint-Fehler
```

```bash
git add -A && git commit -m "feat(core-ui): Phase 1 abgeschlossen — Core UI komplett"
git push
```

---

## PHASE 2: AI Router & Taste Engine (Woche 6-7)

### Ziel
Alle API-Clients funktionieren. Cache reduziert Aufrufe. Budget wird getrackt. meingeschmack/ beeinflusst Prompts.

---

### SCHRITT 2.1: AI Router Grundstruktur (Rust)

Innerhalb von Claude Code:
```
Implementiere den AI Router als Rust-Modul (src-tauri/src/ai_router/).
Nutze rust-engineer und architecture-designer Skills.

Struktur:
- ai_router/mod.rs — Hauptmodul, Request-Queue
- ai_router/router.rs — Routing-Logik (Task-Typ → Modell)
- ai_router/models.rs — Provider/Modell Enums und Konfiguration
- ai_router/queue.rs — Priority Queue mit Tokio (async)
- ai_router/errors.rs — Error Types (thiserror)

Der Router soll:
1. Requests in eine Priority Queue einreihen
2. Basierend auf Task-Typ das optimale Modell wählen
3. Retry-Logic mit Exponential Backoff (3 Versuche)
4. Fallback-Chains (z.B. Kling → Runway → Higgsfield für Video)
5. Tauri Commands exportieren: route_request, get_queue_status

TDD: Tests zuerst. Lies docs/ARCHITEKTUR.md für die Router-Logik.
```

**Prüfung:**
```bash
cargo test -p terryblemachine -- ai_router
# Erwartete Ausgabe: alle ai_router tests ... ok
```

```bash
ls src-tauri/src/ai_router/
# Erwartete Ausgabe: mod.rs, router.rs, models.rs, queue.rs, errors.rs
```

---

### SCHRITT 2.2: API-Clients (9 Clients)

Dieser Schritt ist groß — wir nutzen Parallel Agents:

Innerhalb von Claude Code:
```
Erstelle alle 9 API-Clients als Rust-Module unter src-tauri/src/api_clients/.
Nutze den dispatching-parallel-agents Skill um die Arbeit aufzuteilen.
Nutze den rust-engineer Skill.

Lies docs/LLM-STRATEGIE.md für Details zu jedem Service.

Jeder Client bekommt:
- Eigene Datei (claude.rs, kling.rs, runway.rs, etc.)
- Async Trait ApiClient mit: send_request(), check_health(), get_usage()
- Serde-Structs für Request/Response
- API-Key aus Keychain laden (Schritt 0.7)
- Rate Limiting
- Unit Tests mit Mock-Responses

Die 9 Clients:
1. claude.rs — Anthropic API (Messages API, Vision)
2. kling.rs — Kling AI (Text-to-Video, Image-to-Video)
3. runway.rs — Runway Pro (Gen-3, Motion Brush)
4. higgsfield.rs — Higgsfield (Multi-Modell Video)
5. shotstack.rs — Shotstack (JSON Timeline API)
6. ideogram.rs — Ideogram v3 (Text-in-Image, Logos)
7. meshy.rs — Meshy Pro (Text-to-3D, Image-to-3D)
8. fal.rs — fal.ai (Flux, SDXL, Upscaling)
9. replicate.rs — Replicate (Predictions API)
```

**Prüfung:**
```bash
cargo test -p terryblemachine -- api_clients
# Erwartete Ausgabe: 9 Client-Module, alle Tests grün
```

```bash
ls src-tauri/src/api_clients/
# Erwartete Ausgabe: mod.rs, claude.rs, kling.rs, runway.rs, higgsfield.rs,
#   shotstack.rs, ideogram.rs, meshy.rs, fal.rs, replicate.rs
```

Innerhalb von Claude Code:
```
Teste den Claude API-Client mit einem echten API-Aufruf.
Sende eine einfache Nachricht "Sage nur Hallo" an Claude Haiku.
Zeige mir die Response.
```

---

### SCHRITT 2.3: Cache-System

Innerhalb von Claude Code:
```
Implementiere das Cache-System (src-tauri/src/ai_router/cache.rs):
- Semantic Cache: SHA-256 Hash von (Prompt + Model + Params) → Response
- LRU Cache (max 500 Einträge, konfigurierbar)
- TTL: Einträge verfallen nach 24h (konfigurierbar)
- Cache-Stats: Hit/Miss Ratio, Größe, ältester Eintrag
- Tauri Command: get_cache_stats
- Persistenz: Cache wird als JSON auf Disk gespeichert
TDD mit rust-engineer Skill.
```

**Prüfung:**
```bash
cargo test -p terryblemachine -- cache
# Erwartete Ausgabe: cache tests ... ok
```

Innerhalb von Claude Code:
```
Teste den Cache: Sende den gleichen Request zweimal an den AI Router.
Der zweite Aufruf sollte aus dem Cache kommen (< 1ms).
Zeige mir die Cache-Stats.
```

---

### SCHRITT 2.4: Token-Budget-Manager

Innerhalb von Claude Code:
```
Implementiere den Budget-Manager (src-tauri/src/ai_router/budget.rs):
- Pro-Session und Pro-Tag Tracking
- Kosten-Tabelle pro Provider (aus docs/LLM-STRATEGIE.md)
- Budget-Limits: Warnung bei 80%, Block bei 100%
- Usage-Export als CSV
- Tauri Commands: get_budget_status, set_budget_limit, export_usage
- Frontend-Widget: Budget-Anzeige in der Footer Status-Bar
TDD.
```

**Prüfung:**
```bash
cargo test -p terryblemachine -- budget
# Erwartete Ausgabe: budget tests ... ok
```

```bash
pnpm tauri dev
# Footer zeigt Budget-Anzeige: "$0.00 / $50.00 heute"
```

---

### SCHRITT 2.5: Taste Engine (meingeschmack/)

Innerhalb von Claude Code:
```
Implementiere die Taste Engine (src-tauri/src/taste_engine/).
Lies docs/MEINGESCHMACK-SYSTEM.md für die vollständige Spezifikation.
Nutze rust-engineer Skill.

Module:
- taste_engine/mod.rs — Hauptmodul
- taste_engine/watcher.rs — Ordner-Watcher (notify Crate)
- taste_engine/parser.rs — Markdown-Regeln parsen
- taste_engine/analyzer.rs — Bild-Analyse via Claude Vision
- taste_engine/enricher.rs — Prompts mit Stil-Regeln anreichern
- taste_engine/negative.rs — Negative-Prompts generieren

Flow:
1. Watcher erkennt Änderungen in meingeschmack/
2. Parser liest .md Regeln → strukturierte TasteRegeln
3. Analyzer analysiert Bilder → Farbpaletten, Stile (via Claude Vision API)
4. Enricher fügt Regeln in Prompts ein
5. Negative Generator erstellt Ausschlüsse

TDD: Tests mit Mock-meingeschmack/-Ordner.
```

**Prüfung:**
```bash
cargo test -p terryblemachine -- taste_engine
# Erwartete Ausgabe: alle taste_engine tests ... ok
```

Innerhalb von Claude Code:
```
Teste die Taste Engine end-to-end:
1. Lege eine Test-Regel in meingeschmack/regeln.md an
2. Lege ein Test-Bild in meingeschmack/ ab
3. Sende einen Prompt "Erstelle ein Logo" durch die Engine
4. Zeige mir den angereicherten Prompt (vorher/nachher)
```

---

### MEILENSTEIN Phase 2 — Gesamtprüfung

```bash
cargo test -p terryblemachine && pnpm test && pnpm biome check .
```

Innerhalb von Claude Code:
```
Führe eine vollständige Verifikation von Phase 2 durch.
Nutze den verification-before-completion Skill.
Prüfe:
1. Alle 9 API-Clients können Health-Checks durchführen
2. AI Router routet Requests korrekt (Text→Claude, Bild→fal.ai, Video→Kling)
3. Cache funktioniert (zweiter identischer Request kommt aus Cache)
4. Budget-Manager trackt Kosten korrekt
5. Taste Engine reichert Prompts mit meingeschmack/-Regeln an
6. Alle Tests grün, kein Lint-Fehler
```

```bash
git add -A && git commit -m "feat(ai-router): Phase 2 abgeschlossen — AI Router & Taste Engine"
git push
```

---

## PHASE 3: Website-Builder (Woche 8-10)

### Ziel
Text/Bild/URL → Website-Preview → Export als ZIP.

---

### SCHRITT 3.1: URL-Analyzer (Playwright)

Innerhalb von Claude Code:
```
Implementiere den URL-Analyzer mit Playwright.
Nutze rust-engineer für Tauri-Commands und den playwright-expert Skill.

Funktion:
- URL eingeben → Headless Chromium rendert die Seite
- Style-Extraktion: CSS Custom Properties, Farben, Fonts, Spacing
- Layout-Analyse: Grid/Flex-Struktur erkennen
- Asset-Download: Bilder, Icons, Fonts in Projekt-Ordner
- Screenshot der Seite als Referenz
- Tauri Command: analyze_url(url: String) → AnalysisResult

Hinweis: Playwright läuft als Node.js Sidecar (nicht in Rust direkt).
```

**Prüfung:**
Innerhalb von Claude Code:
```
Teste den URL-Analyzer mit https://stripe.com
Zeige mir:
1. Extrahierte Farben
2. Erkannte Fonts
3. Layout-Struktur
4. Screenshot
```

---

### SCHRITT 3.2: Code-Generator (Claude-basiert)

Innerhalb von Claude Code:
```
Implementiere den Website-Code-Generator:
- Input: Text-Beschreibung + optional URL-Analyse + optional Bild
- Claude Sonnet generiert den Code (React + Tailwind)
- Template-System: Landing Page, Portfolio, Blog, Dashboard, E-Commerce
- meingeschmack/-Regeln werden in den Prompt eingebaut
- Multi-File Output (Komponenten, Styles, Utils)
- Responsive by Default
TDD.
```

**Prüfung:**
Innerhalb von Claude Code:
```
Generiere eine Landing Page für "TERRYBLEMACHINE — AI Design Tool".
Zeige mir die generierten Dateien und ihre Struktur.
```

---

### SCHRITT 3.3: Live-Preview & Code-Editor

Innerhalb von Claude Code:
```
Implementiere die Live-Preview und den Code-Editor:
- Eingebetteter WebView zeigt generierte Website
- Hot-Reload bei Code-Änderungen (< 500ms)
- Device-Simulation: Desktop (1920px), Tablet (768px), Mobile (375px)
- Code-Editor: Monaco Editor mit Syntax-Highlighting
- Inline-Editing: Code ändern → Preview aktualisiert sofort
- Claude-Assist: Code markieren → "Ändere das zu..." → Live-Update
```

**Prüfung:**
```bash
pnpm tauri dev
# 1. Website generieren → Preview zeigt Ergebnis
# 2. Responsive-Buttons klicken → Layout wechselt
# 3. Code im Editor ändern → Preview aktualisiert live
```

---

### SCHRITT 3.4: Export

Innerhalb von Claude Code:
```
Implementiere den Website-Export:
- Standalone HTML/CSS/JS als ZIP
- React-Projekt (mit package.json, README)
- Next.js-Projekt (mit App Router)
- Optional: Vercel/Netlify Deploy-Config
Export-Dialog mit Format-Auswahl.
```

**Prüfung:**
```bash
pnpm tauri dev
# 1. Website generieren → Export-Button klicken
# 2. "ZIP" wählen → Datei wird gespeichert
# 3. ZIP entpacken → index.html im Browser öffnen → Website funktioniert
```

```bash
git add -A && git commit -m "feat(website): Phase 3 abgeschlossen — Website-Builder"
git push
```

---

## PHASE 4: 2D-Grafik & Bild-Erstellung (Woche 11-12)

### Ziel
Text → Bild generieren → im Editor bearbeiten → Export in allen Formaten.

---

### SCHRITT 4.1: Generation-Pipeline

Innerhalb von Claude Code:
```
Implementiere die Bild-Generations-Pipeline:
- Text-to-Image via fal.ai (Flux 2 Pro) als Default
- Replicate als Fallback für Spezialmodelle
- Image-to-Image Pipeline (Stil-Transfer, Variationen)
- Varianten-Generation: 4 Vorschläge parallel (dispatching-parallel-agents)
- Upscaling via Real-ESRGAN (fal.ai)
- meingeschmack/-Regeln werden automatisch eingebaut
Alle API-Aufrufe gehen durch den AI Router (Cache + Budget).
TDD.
```

**Prüfung:**
Innerhalb von Claude Code:
```
Generiere 4 Varianten eines Bildes mit dem Prompt:
"Minimalistisches Logo für ein Tech-Startup, schwarz auf weiß"
Zeige mir die generierten Bilder.
```

---

### SCHRITT 4.2: Fabric.js Editor

Innerhalb von Claude Code:
```
Implementiere den 2D-Editor mit Fabric.js:
- Canvas mit Layer-Management (z-order, visibility, lock)
- Objekt-Manipulation: Move, Scale, Rotate, Flip
- Selection-Tools: Lasso, Rectangle
- Inpainting: Region markieren → Maske an API → neuer Inhalt
- Text-Overlay: Font-Auswahl (Google Fonts), Farbe, Größe
- Filter: Blur, Sharpen, Brightness, Contrast, Saturation
- Crop & Resize
Nutze react-expert für die Integration.
```

**Prüfung:**
```bash
pnpm tauri dev
# 1. Bild generieren → Erscheint auf Canvas
# 2. Bild verschieben, skalieren, rotieren → Funktioniert
# 3. Region markieren → Inpainting-Prompt → Neuer Inhalt
# 4. Text hinzufügen → Font wählen → Text erscheint
```

---

### SCHRITT 4.3: Export in allen Formaten

Innerhalb von Claude Code:
```
Implementiere den 2D-Export:
- PNG (mit/ohne Transparenz)
- JPEG (Qualitätsstufe wählbar: 70-100%)
- SVG (via Fabric.js toSVG)
- WebP (mit Qualitätsstufe)
- PDF (via pdf-lib oder jspdf)
- GIF (animiert falls mehrere Frames, sonst statisch)
Export-Dialog mit Format-Auswahl und Einstellungen pro Format.
```

**Prüfung:**
```bash
pnpm tauri dev
# 1. Bild auf Canvas → Export als PNG → Datei existiert
# 2. Export als JPEG → Datei existiert, Größe < PNG
# 3. Export als SVG → Datei existiert, in Browser öffenbar
# 4. Export als PDF → Datei existiert, in Preview öffenbar
```

```bash
git add -A && git commit -m "feat(graphic2d): Phase 4 abgeschlossen — 2D-Grafik & Bild-Erstellung"
git push
```

---

## PHASE 5: Pseudo-3D (Woche 13-14)

### Ziel
Bild → Depth Map → 3D-Ansicht. Text → Meshy → 3D-Mesh → Three.js.

---

### SCHRITT 5.1: Three.js Integration

Innerhalb von Claude Code:
```
Integriere Three.js als 3D-Renderer:
- React Three Fiber für deklarative Three.js-Nutzung
- @react-three/drei für Helper-Komponenten
- Orthographische + Perspektivische Kamera
- Orbit Controls (Drehen, Zoomen, Pannen)
- Basis-Lighting-Presets (Studio, Outdoor, Dramatic)
- Post-Processing (Bloom, SSAO, optional Film Grain)
Nutze react-expert Skill.
```

**Prüfung:**
```bash
pnpm tauri dev
# 3D-Modul öffnen → Leere Three.js-Szene mit Orbit Controls
# Maus: Drehen, Zoomen, Pannen funktioniert
```

---

### SCHRITT 5.2: Depth-Pipeline + Meshy

Innerhalb von Claude Code:
```
Implementiere die Pseudo-3D-Pipelines:

Pipeline A — Depth Map:
- Bild → Depth-Anything v2 (via Replicate) → Tiefenkarte
- Tiefenkarte → Three.js Displacement auf Plane → Pseudo-3D-Effekt

Pipeline B — Meshy (echte 3D-Meshes):
- Text-to-3D: Beschreibung → Meshy API → GLB-Datei
- Image-to-3D: Bild → Meshy API → GLB-Datei
- GLB in Three.js laden (GLTFLoader)
- PBR-Materialien von Meshy direkt nutzen

Pipeline C — TripoSR (lokale Previews):
- Schnelle lokale Mesh-Generierung als Preview
- Vor Meshy-API-Aufruf für schnelles Feedback

Isometrische Presets: Room, City Block, Product Shot.
```

**Prüfung:**
Innerhalb von Claude Code:
```
1. Lade ein Bild und generiere eine Depth Map. Zeige das Ergebnis in Three.js.
2. Generiere ein 3D-Mesh mit Meshy: "ein minimalistischer Schreibtisch". Zeige in Three.js.
```

---

### SCHRITT 5.3: 3D-Export

Innerhalb von Claude Code:
```
Implementiere den 3D-Export:
- Rendering als PNG/JPEG/WebP/PDF/GIF mit aktueller Kamera-Perspektive
- Transparenz-Option für PNG
- GLB-Export (für Meshy-generierte Meshes)
- Animated GIF: Kamera dreht sich langsam 360° (30 Frames)
```

**Prüfung:**
```bash
pnpm tauri dev
# 3D-Szene mit Mesh → Export als PNG → Datei mit korrekter Perspektive
# Export als GIF → Animierte 360°-Rotation
```

```bash
git add -A && git commit -m "feat(graphic3d): Phase 5 abgeschlossen — Pseudo-3D"
git push
```

---

## PHASE 6: Video-Produktion (Woche 15-17)

### Ziel
Text → Storyboard → KI-Video-Clips + Remotion-Animations → montiertes Video.

---

### SCHRITT 6.1: Storyboard-Generator

Innerhalb von Claude Code:
```
Implementiere den Storyboard-Generator:
- Text-Input → Claude generiert Shot-by-Shot Breakdown
- Jeder Shot: Beschreibung, Stil, Dauer (Sekunden), Kamera, Transition
- Editierbares Storyboard-UI (Drag&Drop Reihenfolge, Shot bearbeiten)
- Template-Storyboards: Commercial, Explainer, Social Media, Music Video
- meingeschmack/-Regeln beeinflussen den visuellen Stil
```

**Prüfung:**
```bash
pnpm tauri dev
# Input: "30-Sekunden Produkt-Spot für ein AI Design Tool"
# → Storyboard mit 5-6 Shots erscheint
# → Shots können verschoben und bearbeitet werden
```

---

### SCHRITT 6.2: KI-Video-Generation

Innerhalb von Claude Code:
```
Implementiere die KI-Video-Pipeline:
- Stil-Router entscheidet: Kling / Runway Pro / Higgsfield
  (Lies docs/LLM-STRATEGIE.md für die Decision Tree)
- Text-to-Video: Storyboard-Shot → API → Video-Clip
- Image-to-Video: Bild → API → Video-Clip (z.B. für Übergänge)
- Segment-basiert: Jeder Shot = ein Clip (5-10 Sekunden)
- Progress-Tracking: Video-Generation dauert oft 1-2 Minuten pro Clip
- Architektur-Pattern nach OpenMontage
```

**Prüfung:**
Innerhalb von Claude Code:
```
Generiere einen 5-Sekunden Video-Clip mit Kling:
"Kamerafahrt über einen minimalistischen Schreibtisch mit Laptop, warm beleuchtet"
Zeige mir das Ergebnis.
```

---

### SCHRITT 6.3: Remotion-Integration (Lokales Rendering)

Innerhalb von Claude Code:
```
Integriere Remotion als Node.js Sidecar in Tauri:
- Remotion-Projekt unter src/remotion/ (oder eigenes Paket)
- Kinetic Typography Komponenten (animierter Text)
- Motion Graphics Komponenten (Datenvisualisierungen, Shapes)
- 3D-Animationen via @remotion/three
- GPU-Beschleunigung: --gl=angle für M3/M4
- Tauri Command: render_remotion(composition, output_path) → Video-Datei
Nutze den remotion-best-practices Skill.
```

**Prüfung:**
Innerhalb von Claude Code:
```
Rendere eine 10-Sekunden Kinetic Typography Animation mit Remotion:
Text: "TERRYBLEMACHINE — Design. Powered by AI."
Zeige mir das fertige MP4.
```

```bash
# Remotion Studio starten (im Remotion-Unterordner):
npx remotion studio
# Browser öffnet → Preview der Komposition sichtbar
```

---

### SCHRITT 6.4: Shotstack (Cloud-Assembly)

Innerhalb von Claude Code:
```
Implementiere die Shotstack-Integration:
- JSON-Timeline-Builder: Clips → Timeline → Render
- Automatische Übergänge (Fade, Dissolve, Wipe)
- Text-Overlays und Branding per API
- Audio-Track hinzufügen (Hintergrundmusik)
- Render-Status-Polling (Shotstack rendert async)
```

**Prüfung:**
Innerhalb von Claude Code:
```
Montiere 3 Test-Clips (Farbflächen als Platzhalter) zu einem 15-Sekunden Video
mit Fade-Übergängen und einem Text-Overlay "TERRYBLEMACHINE".
Nutze Shotstack. Zeige mir das Ergebnis.
```

---

### SCHRITT 6.5: Video-Compositing UI

Innerhalb von Claude Code:
```
Erstelle die Video-Compositing-Oberfläche:
- Segment-Liste: Drag&Drop Reihenfolge
- Per Segment: Preview-Thumbnail, Dauer, Typ (KI/Remotion/Shotstack)
- Routing-Auswahl: Lokal (Remotion) vs Cloud (Shotstack)
- Export-Settings: Auflösung (720p/1080p/4K), FPS (24/30/60), Format (MP4/WebM/GIF)
- Render-Button mit Progress-Bar
- Frame-Sequenz Export (PNG)
```

**Prüfung:**
```bash
pnpm tauri dev
# 1. Storyboard erstellen → Clips generieren → Segments erscheinen
# 2. Reihenfolge per Drag&Drop ändern → Funktioniert
# 3. Export als MP4 → Video wird gerendert
```

```bash
git add -A && git commit -m "feat(video): Phase 6 abgeschlossen — Video-Produktion"
git push
```

---

## PHASE 7: Typografie & Logos (Woche 18-19)

### Ziel
Logo-Erstellung von Text-Input bis fertiges SVG-Asset-Paket.

---

### SCHRITT 7.1: Logo-Generation (Ideogram)

Innerhalb von Claude Code:
```
Implementiere die Logo-Generation:
- Ideogram v3 API Integration (bereits in api_clients/ideogram.rs)
- Text-Input → Varianten-Generation (5-10 Vorschläge)
- Style-Kontrolle: Minimalistisch, Wortmarke, Emblem, Maskottchen
- Farbpaletten-Vorschläge basierend auf meingeschmack/
- Auswahl-Gallery mit Favoriten-Funktion
```

**Prüfung:**
Innerhalb von Claude Code:
```
Generiere 5 Logo-Varianten für "TERRYBLEMACHINE" mit Ideogram.
Stil: Minimalistisch, Tech, Monochrom.
Zeige mir die Ergebnisse.
```

---

### SCHRITT 7.2: Vektorisierung & SVG-Editor

Innerhalb von Claude Code:
```
Implementiere die Vektorisierung und den SVG-Editor:
- Raster → SVG via VTracer (lokal, Open Source)
- SVG-Cleanup: Pfade vereinfachen, Gruppen aufräumen
- Fabric.js SVG-Editor: Pfade bearbeiten, Farben ändern
- Kerning & Tracking Controls für Text-Logos
- Bezier-Pfad-Editing für manuelle Anpassungen
- Font-Browser (Google Fonts + lokale System-Fonts)
```

**Prüfung:**
```bash
pnpm tauri dev
# 1. Logo generieren → "Vektorisieren" → SVG erscheint
# 2. SVG im Editor: Farben ändern, Pfade bearbeiten → Funktioniert
# 3. Text hinzufügen → Kerning anpassen → Sieht gut aus
```

---

### SCHRITT 7.3: Brand-Asset-Export

Innerhalb von Claude Code:
```
Implementiere den Brand-Asset-Export:
- Logo in allen Größen: Favicon (16/32/64px), Web (128/256/512px), Print (1024/2048px)
- Farbvarianten: Original, Schwarz/Weiß, Invertiert
- Formate: SVG, PNG, PDF
- Automatischer Style Guide als HTML/PDF:
  - Logo-Varianten, Mindestgrößen, Schutzzone
  - Farbpalette mit HEX/RGB/CMYK
  - Typografie-Spezifikation
- Alles als ZIP-Paket
```

**Prüfung:**
```bash
pnpm tauri dev
# 1. Logo fertig → "Brand Kit exportieren"
# 2. ZIP wird erstellt mit allen Varianten
# 3. ZIP entpacken → Alle Dateien vorhanden
```

```bash
git add -A && git commit -m "feat(typography): Phase 7 abgeschlossen — Typografie & Logos"
git push
```

---

## PHASE 8: Polish & Release (Woche 20-21)

### Ziel
App ist stabil, performant, und bereit für v1.0.

---

### SCHRITT 8.1: Performance-Optimierung

Innerhalb von Claude Code:
```
Optimiere die Performance. Nutze den verification-before-completion Skill.

1. Bundle-Analyse: pnpm run build && npx vite-bundle-visualizer
2. Tree Shaking: Ungenutzte Imports entfernen
3. Lazy Loading: Jedes Modul wird erst bei Navigation geladen
4. Web Worker: Schwere Berechnungen (Bild-Processing, SVG-Parsing)
5. Rust-Profiling: cargo flamegraph für Hot-Paths
6. Startup-Zeit messen und optimieren (Ziel: < 1s)
```

**Prüfung:**
```bash
# Bundle-Größe prüfen:
pnpm run build && du -sh dist/
# Ziel: < 5 MB (ohne Tauri-Runtime)

# Startup-Zeit messen:
time pnpm tauri dev
# Ziel: Fenster erscheint in < 2s (Dev-Modus)
```

---

### SCHRITT 8.2: Onboarding & UX-Polish

Innerhalb von Claude Code:
```
Erstelle den Onboarding-Flow und UX-Polish:
1. First-Start Wizard:
   - Willkommen → API-Keys eingeben → meingeschmack/ erklären → Fertig
2. Keyboard-Shortcuts für alle Aktionen (Cmd+N, Cmd+S, Cmd+Z, etc.)
3. Tooltips auf allen Buttons
4. Loading-States: Skeleton-Loader, Progress-Bars
5. Error-States: Benutzerfreundliche Fehlermeldungen als Toast
6. Empty-States: Hilfreiche Hinweise wenn ein Modul noch leer ist
Nutze frontend-design Skill.
```

**Prüfung:**
```bash
pnpm tauri dev
# 1. App-Daten löschen → Neu starten → Onboarding-Wizard erscheint
# 2. Durch alle Schritte klicken → API-Keys → meingeschmack → Fertig
# 3. Keyboard-Shortcuts testen (Cmd+N für neues Projekt)
```

---

### SCHRITT 8.3: Comprehensive Testing

Innerhalb von Claude Code:
```
Erstelle umfassende Tests für die gesamte App:
Nutze den test-master Skill.

1. Unit Tests (Vitest): Ziel >80% Coverage auf kritische Pfade
2. Integration Tests: API-Clients mit Mock-Servern
3. E2E Tests (Playwright): Hauptflows pro Modul
   - Website: Input → Generate → Preview → Export
   - 2D: Input → Generate → Edit → Export
   - 3D: Input → Generate → Preview → Export
   - Video: Input → Storyboard → Generate → Montage → Export
   - Typo: Input → Generate → Vectorize → Export
4. Error-Handling Tests: API-Fehler, Netzwerk-Timeout, Disk-Full
```

**Prüfung:**
```bash
pnpm test -- --coverage
# Ziel: > 80% Coverage

cargo test -p terryblemachine
# Alle Rust-Tests grün

pnpm exec playwright test
# Alle E2E-Tests grün
```

---

### SCHRITT 8.4: Distribution (macOS App bauen)

Innerhalb von Claude Code:
```
Bereite die Distribution vor:
1. macOS Code Signing (Apple Developer Certificate)
2. tauri.conf.json: Version 1.0.0, App-Icon, Bundle-ID
3. DMG Installer konfigurieren
4. Auto-Update System (Tauri Updater Plugin)
5. Final Build erzeugen
```

**Prüfung:**
```bash
# Release-Build:
pnpm tauri build
# Erwartete Ausgabe: Build erfolgreich

# DMG existiert:
ls src-tauri/target/release/bundle/dmg/
# Erwartete Ausgabe: TERRYBLEMACHINE_1.0.0_aarch64.dmg

# App installieren und testen:
open src-tauri/target/release/bundle/dmg/TERRYBLEMACHINE_1.0.0_aarch64.dmg
# App installiert sich → Öffnen → Alles funktioniert
```

---

### MEILENSTEIN Phase 8 — FINALE Prüfung

Innerhalb von Claude Code:
```
Führe die finale Verifikation durch.
Nutze den verification-before-completion Skill.

Komplett-Check:
1. ✅ App startet < 1s, keine Konsolen-Fehler
2. ✅ Onboarding-Wizard funktioniert
3. ✅ Alle 5 Module: Input → Generate → Edit → Export
4. ✅ AI Router: Caching, Budget, Fallbacks
5. ✅ meingeschmack/ beeinflusst alle Outputs
6. ✅ Dark Mode durchgängig
7. ✅ Undo/Redo in allen Modulen
8. ✅ Alle Tests grün (Unit + Integration + E2E)
9. ✅ Bundle-Größe < 15 MB (inkl. Tauri)
10. ✅ DMG installiert und funktioniert
```

```bash
git add -A && git commit -m "feat(release): v1.0.0 — TERRYBLEMACHINE Release"
git tag v1.0.0
git push && git push --tags
```

---

## Zusammenfassung aller Git-Commits

| Phase | Commit | Inhalt |
|-------|--------|--------|
| 0 | `feat(core): Phase 0 — Foundation` | Tauri + React + Biome + Vitest + Zustand + CI/CD + Keychain |
| 1 | `feat(core-ui): Phase 1 — Core UI` | App-Shell, Design-System, Routing, Projekt-Management, Input, Undo/Redo |
| 2 | `feat(ai-router): Phase 2 — AI Router` | 9 API-Clients, Router, Cache, Budget, Taste Engine |
| 3 | `feat(website): Phase 3 — Website-Builder` | URL-Analyzer, Code-Gen, Preview, Export |
| 4 | `feat(graphic2d): Phase 4 — 2D-Grafik` | Generation, Fabric.js Editor, Multi-Format-Export |
| 5 | `feat(graphic3d): Phase 5 — Pseudo-3D` | Depth-Pipeline, Meshy, Three.js, Export |
| 6 | `feat(video): Phase 6 — Video` | Storyboard, KI-Video, Remotion, Shotstack, Compositing |
| 7 | `feat(typography): Phase 7 — Typografie` | Ideogram, SVG-Editor, Brand-Kit-Export |
| 8 | `feat(release): v1.0.0` | Performance, Onboarding, Tests, Distribution |

---

## Skill-Zuordnung pro Phase

| Phase | Primäre Skills |
|-------|---------------|
| 0 | rust-engineer, react-expert, test-driven-development |
| 1 | frontend-design, react-expert, brainstorming |
| 2 | rust-engineer, architecture-designer, dispatching-parallel-agents |
| 3 | playwright-expert, react-expert, rust-engineer |
| 4 | react-expert, rust-engineer |
| 5 | react-expert (Three.js), rust-engineer |
| 6 | remotion-best-practices, rust-engineer, react-expert |
| 7 | react-expert, rust-engineer |
| 8 | test-master, verification-before-completion, frontend-design |
