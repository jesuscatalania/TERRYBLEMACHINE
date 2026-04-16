# TERRYBLEMACHINE — Entwicklungsplan

## Übersicht der Phasen

```
Phase 0: Foundation        [2 Wochen]   ████░░░░░░░░░░░░░░░░
Phase 1: Core UI           [3 Wochen]   ░░░░████░░░░░░░░░░░░
Phase 2: AI Router         [2 Wochen]   ░░░░░░░░███░░░░░░░░░
Phase 3: Website-Modul     [3 Wochen]   ░░░░░░░░░░░████░░░░░
Phase 4: 2D-Grafik         [2 Wochen]   ░░░░░░░░░░░░░░░██░░░
Phase 5: Pseudo-3D         [2 Wochen]   ░░░░░░░░░░░░░░░░░██░
Phase 6: Video             [3 Wochen]   ░░░░░░░░░░░░░░░░░░░█ ...
Phase 7: Typo & Logos      [2 Wochen]
Phase 8: Polish & Release  [2 Wochen]
─────────────────────────────────────
Gesamt:                    ~21 Wochen
```

**Hinweis**: Die Zeitschätzungen sind optimistisch und gehen von Vollzeit-Entwicklung mit Claude Code aus. Realistisch sollte man 30-50% Puffer einplanen.

---

## Phase 0: Foundation (Woche 1-2)

### Ziel
Projekt-Grundgerüst, Tooling, CI/CD — alles steht, bevor die erste Zeile Feature-Code geschrieben wird.

### Tasks

**0.1 Projekt-Setup**
- Tauri v2 + React + TypeScript Projekt initialisieren
- pnpm Workspace konfigurieren
- Biome (Linter/Formatter) einrichten
- Vitest + Testing Library konfigurieren
- Git-Repository + GitHub Remote erstellen
- Branch-Schutzregeln auf `main` und `develop`

**0.2 Tauri-Konfiguration**
- `tauri.conf.json` mit App-Metadaten, Fenster-Konfiguration
- Permissions für File System, Shell, Dialog
- Auto-Updater vorbereiten (für spätere Updates)
- macOS Code-Signing Setup (Apple Developer Account nötig)

**0.3 CI/CD Pipeline**
- GitHub Actions: Lint → Test → Build auf jedem PR
- Rust-Tests + Frontend-Tests parallel
- macOS Build als Artifact

**0.4 API-Key Management**
- Rust-Modul für macOS Keychain Integration
- Settings-Seite (Skeleton) für Key-Eingabe
- Environment-Variable Fallback für Development

### Meilenstein
- `pnpm tauri dev` öffnet leeres Fenster
- Tests laufen (grün, auch wenn leer)
- CI/CD Pipeline grün auf GitHub
- API-Keys können sicher gespeichert und gelesen werden

---

## Phase 1: Core UI (Woche 3-5)

### Ziel
Die gesamte UI-Shell steht — Navigation, Theme, Layout — ohne Feature-Logik.

### Tasks

**1.1 App-Shell**
- Sidebar-Navigation (Website, 2D, 3D, Video, Typo)
- Header mit Projekt-Name, Settings-Zugang
- Main Content Area (flexibel, modul-abhängig)
- Footer mit Status-Bar (AI-Status, Budget, Fortschritt)

**1.2 Design-System**
- Tailwind Config mit Custom Theme (Farben, Fonts, Spacing)
- Wiederverwendbare Komponenten: Button, Input, Card, Modal, Dropdown, Tabs
- Dark Mode (Standard) + Light Mode
- Animationen: Framer Motion für Page-Transitions

**1.3 State Management**
- Zustand Stores: App-State, Project-State, AI-State, UI-State
- Persistenz: Projekte werden als JSON auf Disk gespeichert
- Undo/Redo System (Command Pattern)

**1.4 Projekt-Management**
- Neues Projekt erstellen (Name, Typ, Beschreibung)
- Projekte öffnen/schließen
- Projekt-Dateien (Assets, Exports) auf Disk verwalten
- Recents-Liste

**1.5 Input-System (universell)**
- Text-Input mit Auto-Resize
- Bild-Upload (Drag&Drop, File Picker) — alle Formate (PNG, JPG, WebP, SVG, TIFF, PSD-Preview)
- URL-Input mit Validation
- Input-History (letzte Prompts)

### Meilenstein
- App sieht professionell aus
- Navigation zwischen allen 5 Modulen funktioniert
- Projekte können erstellt und gespeichert werden
- Bilder können hochgeladen werden
- Dark Mode funktioniert

---

## Phase 2: AI Router & Taste Engine (Woche 6-7)

### Ziel
Das "Gehirn" der App — intelligentes Modell-Routing, Caching, Budget-Tracking und meingeschmack/-Integration.

### Tasks

**2.1 AI Router (Rust)**
- Request-Queue mit Priority-System
- Model-Selection-Logik (Task-Typ → optimales Modell)
- Retry-Logic mit Exponential Backoff
- Error Handling und Fallback-Chains

**2.2 API-Clients (Rust) — 9 Clients**
Abo-APIs:
- Claude Client (Anthropic, für programmatischen Zugriff)
- Kling Client (Video-Generierung)
- Runway Client (Video mit Motion Control)
- Higgsfield Client (Video, Multi-Modell)
- Shotstack Client (Video-Montage via JSON-Timeline)
- Ideogram Client (Text-in-Bild, Logos)
- Meshy Client (3D-Generierung: Text-to-3D, Image-to-3D)

Pay-per-Use-APIs:
- fal.ai Client (Bild-Generierung: Flux, SDXL, Upscaling)
- Replicate Client (Spezial-/Nischenmodelle)

Lokal:
- Remotion Integration (Video-Rendering via Node.js Sidecar)

**2.3 Cache-System**
- Semantic Cache: SHA-256 Hash von (Prompt + Model + Params) → Response
- LRU Cache mit konfigurierbarer Größe
- Cache-Invalidierung nach Zeitablauf
- Cache-Stats im Dashboard

**2.4 Token-Budget-Manager**
- Pro-Session und Pro-Tag Tracking
- Kosten-Berechnung pro Provider (unterschiedliche Preise)
- Budget-Limits mit Warnungen
- Export der Nutzungsdaten (CSV)

**2.5 Taste Engine (meingeschmack/)**
- Ordner-Watcher: Änderungen in `meingeschmack/` erkennen
- Bild-Analyse: Farbpaletten, Stile, Muster extrahieren (via Claude Vision)
- Markdown-Parser: Regeln aus `.md`-Dateien parsen
- Prompt-Enricher: Regeln in Prompts einfügen
- Negative-Prompt-Generator: Ausschlüsse automatisch formulieren

### Meilenstein
- API-Aufrufe funktionieren für alle Provider
- Cache reduziert redundante Aufrufe
- Budget wird live getrackt und angezeigt
- `meingeschmack/`-Regeln beeinflussen Prompt-Generierung

---

## Phase 3: Website-Builder (Woche 8-10)

### Ziel
Vollständiger Website-Erstellungsflow: Text/Bild/URL → Preview → Export.

### Tasks

**3.1 URL-Analyzer**
- Playwright-basiertes Rendering der Ziel-URL
- Style-Extraktion: CSS-Variablen, Farben, Fonts, Spacing
- Layout-Analyse: Grid/Flex-Struktur erkennen
- Asset-Download: Bilder, Icons, Fonts
- Zugänglichkeits-Check

**3.2 Code-Generator**
- Claude Sonnet/Opus Integration für Code-Generierung
- Template-System (Landing Page, Portfolio, Blog, Dashboard, E-Commerce)
- Multi-File-Output (Komponenten, Styles, Utils)
- Responsive by Default

**3.3 Live-Preview**
- Eingebetteter WebView für Preview
- Hot-Reload bei Code-Änderungen
- Device-Simulation (Desktop, Tablet, Mobile)
- Zoom-Kontrolle

**3.4 Code-Editor**
- Syntax-Highlighting (Monaco Editor oder CodeMirror)
- Inline-Editing mit sofortigem Preview-Update
- Claude-Assist: Markierten Code erklären/ändern lassen
- Datei-Baum für Multi-File-Projekte

**3.5 Export**
- Standalone HTML/CSS/JS ZIP
- React-Projekt mit package.json
- Next.js-Projekt
- Deployment-Ready (Vercel/Netlify Config)

### Meilenstein
- URL eingeben → Website wird analysiert → Stil wird extrahiert
- Text eingeben → Website wird generiert → Preview zeigt Ergebnis
- Code kann inline bearbeitet werden
- Export als ZIP funktioniert

---

## Phase 4: 2D-Grafik & Bild-Erstellung (Woche 11-12)

### Ziel
Vollständiger 2D-Grafik- und Bild-Erstellungs-Workflow mit Fabric.js-Editor.

### Tasks

**4.1 Generation-Pipeline**
- Text-to-Image via fal.ai (Flux 2 Pro)
- Image-to-Image Pipeline
- Varianten-Generation (4-8 Vorschläge parallel)
- Upscaling-Pipeline (Real-ESRGAN)

**4.2 Fabric.js Editor**
- Canvas mit Layer-Management
- Objekt-Manipulation (Move, Scale, Rotate)
- Selection-Tools (Lasso, Rectangle, Magic Wand)
- Inpainting: Region markieren → KI füllt

**4.3 Nachbearbeitung**
- Filter (Blur, Sharpen, Color Adjust)
- Text-Overlay mit Font-Auswahl
- Crop & Resize
- Format-Konvertierung (PNG, JPEG, SVG, WebP, PDF, GIF)

**4.4 Asset-Management**
- Generierte Bilder in Projekt-Library
- Tagging und Suche
- Version History pro Asset

### Meilenstein
- Text → Bild-Generierung funktioniert
- Editor erlaubt Nachbearbeitung
- Inpainting funktioniert
- Export in allen Formaten (PNG, JPEG, SVG, WebP, PDF, GIF)

---

## Phase 5: Pseudo-3D (Woche 13-14)

### Tasks

**5.1 Depth-Pipeline**
- Depth-Anything v2 Integration (Tiefenkarten aus Bildern)
- Three.js Scene-Setup (Orthographische Kamera, Lighting)
- Depth-Map → 3D-Displacement auf Plane

**5.2 Meshy-Integration**
- Meshy API Client (Text-to-3D, Image-to-3D)
- GLB-Import in Three.js Scene
- PBR-Material-Rendering (Texturen von Meshy direkt nutzbar)
- TripoSR als schnelle lokale Alternative für Previews

**5.3 Isometric Generator**
- Spezialisierte Prompts für isometrische Ansichten
- LoRA-Management für 3D-Stile
- Preset-Szenen (Room, City Block, Product Shot)

**5.4 Preview & Export**
- Interaktive Three.js-Ansicht (Orbit, Zoom)
- Rendering mit gewählter Perspektive
- Export als PNG, JPEG, WebP, PDF, GIF mit Transparenz-Option

### Meilenstein
- Bild → Depth Map → 3D-Ansicht funktioniert
- Text/Bild → Meshy → 3D-Mesh → Three.js funktioniert
- Isometrische Grafiken können generiert werden
- Export in allen Formaten (PNG, JPEG, WebP, PDF, GIF)

---

## Phase 6: Video-Produktion (Woche 15-17)

### Tasks

**6.1 Storyboard-Generator**
- Claude generiert Shot-by-Shot Breakdown
- Jeder Shot: Beschreibung, Stil, Dauer, Transition
- Editierbares Storyboard-UI

**6.2 KI-Video-Generation-Pipeline**
- Text-to-Video: Kling (Echtfilm, Qualität), Runway Pro (Motion Control, 4K)
- Image-to-Video: Kling / Higgsfield (Multi-Modell)
- Segment-basiert: Storyboard → einzelne Clips → zusammenführen
- Architektur-Pattern nach OpenMontage: Orchestrator koordiniert alle Video-Tools

**6.3 Remotion-Integration (Lokales Rendering)**
- Remotion als Node.js Sidecar-Prozess in Tauri einbinden
- Kinetic Typography: React-Komponenten → animierter Text → MP4
- Motion Graphics: Programmatische Animationen, Datenvisualisierungen
- 3D-Animationen: @remotion/three (Three.js + React Three Fiber)
- GPU-Beschleunigung auf M3/M4 aktivieren (--gl=angle)
- Remotion Claude Code Skill für Code-Generierung nutzen

**6.4 Shotstack-Integration (Cloud-Assembly)**
- JSON-Timeline-API für programmatische Clip-Montage
- Automatische Übergänge zwischen KI-generierten Segmenten
- Text-Overlays, Wasserzeichen, Branding per API
- Audio-Track-Integration

**6.5 Video-Compositing (UI)**
- Segment-Reihenfolge ändern
- Routing: Remotion (lokal, gratis) vs. Shotstack (Cloud) je nach Aufgabe
- Export-Settings (Auflösung, FPS, Format)

### Meilenstein
- Text → Storyboard → KI-Video-Clips (Kling/Runway) funktioniert
- Remotion rendert Kinetic Typography und Motion Graphics lokal
- Shotstack montiert Clips automatisch zu fertigem Video
- Video-Export als MP4

---

## Phase 7: Typografie & Logos (Woche 18-19)

### Tasks

**7.1 Logo-Generation**
- Ideogram v3 Integration
- Varianten-Generation mit Style-Kontrolle
- Farbpaletten-Vorschläge basierend auf meingeschmack/

**7.2 Vektorisierung**
- Raster → SVG (VTracer oder Potrace)
- SVG-Cleanup und -Optimierung
- Pfad-Vereinfachung

**7.3 Typography-Editor**
- Font-Browser (Google Fonts + lokale Fonts)
- Kerning & Tracking Controls
- Bezier-Pfad-Editing für Logo-Formen
- Farb-Varianten-Generator

**7.4 Brand-Asset-Export**
- Logo in allen Größen (Favicon → Billboard)
- Farbvarianten (Color, B&W, Inverted)
- SVG, PNG, PDF, EPS Export
- Style Guide Auto-Generation

### Meilenstein
- Logo-Erstellung von Text-Input bis fertigem SVG
- Typografie-Kompositionen mit Feinsteuerung
- Export als Brand-Asset-Paket

---

## Phase 8: Polish & Release (Woche 20-21)

### Tasks

**8.1 Performance-Optimierung**
- Bundle-Analyse und Tree Shaking
- Lazy Loading für Module
- Web Worker für schwere Berechnungen
- Rust-seitiges Profiling

**8.2 UX-Polish**
- Onboarding-Flow (erster Start, API-Keys, meingeschmack/)
- Keyboard-Shortcuts für alle Aktionen
- Tooltips und Kontexthilfe
- Loading-States und Progress-Indikatoren

**8.3 Testing**
- Unit Tests (Vitest): >80% Coverage auf kritische Pfade
- Integration Tests: API-Clients mit Mock-Servern
- E2E Tests (Playwright): Hauptflows pro Modul
- Manual QA: Edge Cases, Error States

**8.4 Distribution**
- macOS Code Signing
- DMG Installer erstellen
- Auto-Update System (Tauri Updater)
- Landing Page für das Projekt (optional)

### Meilenstein
- App läuft stabil auf M3/M4
- Alle Module funktionieren End-to-End
- DMG Installer erstellt
- Version 1.0 Release

---

## Risiken & Mitigationen

| Risiko | Wahrscheinlichkeit | Impact | Mitigation |
|--------|-------------------|--------|------------|
| API-Kosten eskalieren | Mittel | Hoch | Budget-Limits, aggressives Caching, lokale Modelle als Fallback |
| Video-APIs instabil/teuer | Hoch | Mittel | Multiple Fallback-Provider, Custom-Pipelines für einfache Stile |
| Tauri v2 Bugs/Limitierungen | Niedrig | Mittel | Active Community, Electron als Fallback-Plan |
| Canvas-Performance bei komplexen Szenen | Mittel | Mittel | WebGL-Fallback (PixiJS), Offscreen Rendering |
| meingeschmack/-Analyse unzuverlässig | Mittel | Niedrig | Manuelle Override-Option, iterative Verbesserung |
| Sora API Sunset | Sicher | Niedrig | War nie primär geplant, Kling als Hauptprovider |

---

## Post-v1 Roadmap (Ideen)

- **Lokale Modelle**: SDXL/Flux Dev lokal für Offline und kostenloses Iterieren
- **Plugin-System**: Erweiterbare Module von Drittanbietern
- **Kollaboration**: Projekte teilen und zusammen bearbeiten
- **Template-Marktplatz**: Vorgefertigte Designs kaufen/teilen
- **Audio-Integration**: Musik und Sounddesign für Videos
- **Print-Export**: CMYK-Unterstützung, PDF/X für Druckproduktion
- **Agent-Mode**: Autonome Multi-Step-Aufgaben (z.B. "Erstelle eine komplette Brand-Identity")
