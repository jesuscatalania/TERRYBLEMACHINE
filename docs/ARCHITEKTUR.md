# TERRYBLEMACHINE — Technische Architektur

## Überblick

```
┌─────────────────────────────────────────────────────┐
│                    TERRYBLEMACHINE                    │
│                   (Tauri v2 App)                     │
├──────────────────┬──────────────────────────────────┤
│   FRONTEND       │         RUST BACKEND              │
│   (WebView)      │         (Tauri Core)              │
│                  │                                    │
│  React + TS      │  ┌─ AI Router ──────────────┐    │
│  Tailwind CSS    │  │  Model Selection         │    │
│  Fabric.js       │  │  Token Budget Manager    │    │
│  Three.js        │  │  Cache Layer             │    │
│  PixiJS          │  │  Request Queue           │    │
│                  │  └──────────────────────────┘    │
│  ┌────────────┐  │                                   │
│  │ Module     │  │  ┌─ File System ────────────┐    │
│  │ Switcher   │  │  │  Project Manager         │    │
│  │            │  │  │  Asset Pipeline           │    │
│  │ - Website  │  │  │  meingeschmack/ Reader   │    │
│  │ - 2D       │  │  │  Export Engine            │    │
│  │ - 3D       │  │  └──────────────────────────┘    │
│  │ - Video    │  │                                   │
│  │ - Typo     │  │  ┌─ Website Analyzer ───────┐    │
│  └────────────┘  │  │  Headless Chromium        │    │
│                  │  │  Style Extractor          │    │
│                  │  │  Code Parser              │    │
│                  │  └──────────────────────────┘    │
├──────────────────┴──────────────────────────────────┤
│                   SYSTEM LAYER                       │
│  macOS APIs · Metal GPU · Neural Engine · FileSystem │
└─────────────────────────────────────────────────────┘
          │                    │                │
          ▼                    ▼                ▼
   ┌──────────┐    ┌───────────────┐   ┌──────────────┐
   │Claude Max│    │ Abo-APIs      │   │ Lokale Tools │
   │ (Abo)    │    │ Kling         │   │ Remotion     │
   │          │    │ Runway Pro    │   │ OpenMontage  │
   │ Opus     │    │ Higgsfield    │   │ TripoSR      │
   │ Sonnet   │    │ Ideogram      │   │ VTracer      │
   │ Haiku    │    │ Meshy         │   │              │
   │          │    │ Shotstack     │   │              │
   │          │    ├───────────────┤   │              │
   │          │    │ Pay-per-Use   │   │              │
   │          │    │ fal.ai        │   │              │
   │          │    │ Replicate     │   │              │
   └──────────┘    └───────────────┘   └──────────────┘
```

---

## Stack-Entscheidungen

### Warum Tauri v2 statt Electron/Swift

| Kriterium | Tauri v2 | Electron | Swift/SwiftUI |
|-----------|----------|----------|---------------|
| Bundle-Größe | <10 MB | 100-150 MB | 5-20 MB |
| RAM (idle) | 30-40 MB | 200-300 MB | 50-100 MB |
| Startup | <0.5s | 1-2s | <0.1s |
| GPU-Zugang | Via WebGL + Rust FFI zu Metal | Via WebGL | Direkt Metal |
| Web-Frontend | Ja (React/Vue/Svelte) | Ja | Nein |
| Cross-Platform | macOS/Win/Linux/iOS/Android | macOS/Win/Linux | Nur Apple |
| Security | Rust-Backend, opt-in APIs | Full Node.js | Full native |
| Ecosystem | npm + crates.io | npm | Swift Package Manager |

**Entscheidung**: Tauri v2 bietet den besten Kompromiss aus Performance, kleiner Bundle-Größe und Web-Frontend-Flexibilität. Das Rust-Backend gibt uns sichere, performante API-Aufrufe und File-System-Operationen. Falls wir Metal-GPU direkt brauchen, können wir über Rust FFI darauf zugreifen.

### Warum React + TypeScript

- Größtes Ökosystem für UI-Komponenten
- TypeScript gibt uns Typsicherheit für komplexe State-Management-Logik
- Claude/GPT generieren exzellenten React/TS-Code
- Kompatibel mit allen Canvas-Libraries (Fabric.js, Three.js, PixiJS)
- Tailwind CSS für schnelles, konsistentes Styling

### Rendering-Stack (Dreischichtig)

```
Layer 3: Three.js    → Pseudo-3D Previews, Depth-Effekte, Perspektive
Layer 2: PixiJS      → Hochperformante Sprite-Rendering, Partikel, Animationen  
Layer 1: Fabric.js   → Interaktiver 2D-Editor (Drag&Drop, Objekt-Manipulation, SVG)
Layer 0: HTML/CSS    → UI-Chrome, Panels, Toolbars
```

**Fabric.js** als primäre Editor-Engine weil:
- Bewährtes Objektmodell (Gruppen, Layer, Transformationen)
- SVG Import/Export
- Text-Editing direkt auf Canvas
- Event-System für Interaktivität
- Serialisierung (JSON save/load)

**PixiJS** für Performance-kritische Rendering-Tasks:
- WebGL-acceleriert (60fps auch bei tausenden Objekten)
- Sprite-Batching
- Filter-Pipeline (Blur, Color, Displacement)

**Three.js** für Pseudo-3D:
- Orthographische Kamera für isometrische Ansichten
- Depth-of-Field, Ambient Occlusion
- Material-System für realistische Oberflächen
- Post-Processing (Bloom, SSAO, Film Grain)

---

## Module (Detailliert)

### Modul 1: Website-Builder

```
Input Parser
  ├── Text → Claude Sonnet (Konzept + Code)
  ├── Bild → Claude Vision (Analyse) → Code-Generierung
  └── URL → Website Analyzer
         ├── Headless Chromium (Playwright)
         ├── Style Extractor (Farben, Fonts, Spacing)
         ├── Layout Analyzer (Grid/Flex-Struktur)
         ├── Asset Downloader (Bilder, Icons)
         └── Code Generator (HTML/CSS/JS → React)

Preview Engine
  ├── Live-Preview im eingebetteten WebView
  ├── Responsive-Ansichten (Desktop, Tablet, Mobile)
  └── Hot-Reload bei Code-Änderungen

Export
  ├── Standalone HTML/CSS/JS
  ├── React-Projekt (mit package.json)
  ├── Next.js-Projekt
  └── ZIP-Download
```

### Modul 2: 2D-Grafik & Bild-Erstellung

```
Input Parser
  ├── Text → Prompt Engineering → Image API
  ├── Bild → Image-to-Image Pipeline
  └── Sketch → ControlNet/Canny → Refined Output

Generation Pipeline
  ├── meingeschmack/ Regeln laden
  ├── Prompt Construction (Text + Style Rules + Negative Prompts)
  ├── Model Selection (Budget vs Quality)
  ├── Generation (fal.ai / Replicate / Lokal)
  ├── Upscaling (optional, Real-ESRGAN)
  └── Format-Konvertierung (PNG, JPEG, SVG, WebP, PDF, GIF)

Editor (Fabric.js)
  ├── Layer-Management
  ├── Masken & Selections
  ├── Inpainting (Region markieren → neu generieren)
  ├── Text-Overlay
  └── Filter & Adjustments
```

### Modul 3: Pseudo-3D-Grafik & -Bilder

```
Input Parser
  ├── Text → Stil-Erkennung (isometrisch, perspektivisch, flat-3D)
  ├── Bild → Depth Estimation (Depth-Anything v2)
  └── Beschreibung → 3D-Scene-Prompt

Generation Pipeline
  ├── Image Generation mit 3D-optimierten Prompts
  ├── Depth Map Extraction (Depth-Anything v2)
  ├── Meshy Text-to-3D / Image-to-3D (PBR Texturen, GLB Export)
  ├── TripoSR für schnelle Mesh-Previews (Open Source, lokal)
  └── Three.js Compositing (Depth + Image + Mesh + Lighting)

Preview (Three.js)
  ├── Interaktive Kamera (Orbit Controls)
  ├── Lighting Presets
  ├── Material-Anpassungen (PBR von Meshy direkt nutzbar)
  └── Export als PNG, JPEG, WebP, PDF, GIF mit gewählter Perspektive
```

### Modul 4: Video

```
Input Parser
  ├── Text → Storyboard Generator (Claude)
  ├── Bild → Image-to-Video
  ├── Referenzvideo → Style-Analyse
  └── Storyboard → Frame-by-Frame Plan

Generation Pipeline
  ├── Stil-Router:
  │   ├── Echtfilm → Kling / Runway
  │   ├── Animation → Kling
  │   ├── Motion Graphics → Custom Pipeline (Lottie + PixiJS)
  │   ├── Kinetic Typography → Custom (Three.js + Fabric.js)
  │   └── Pseudo-3D → Three.js Animation Export
  ├── Segment-Generation (5-10s Clips)
  ├── Transition-Handling
  └── Audio-Sync (optional)

Post-Production (Shotstack API)
  ├── Clip-Montage: Segmente zusammenfügen via JSON-Timeline
  ├── Text-Overlays & Transitions programmatisch
  ├── Audio-Track hinzufügen
  └── Architektur-Pattern nach OpenMontage (Agent-Orchestrierung)

Export
  ├── MP4 (H.264/H.265)
  ├── WebM (VP9)
  ├── GIF (für kurze Loops)
  └── Frame-Sequenz (PNG)
```

### Modul 5: Typografie & Logo

```
Input Parser
  ├── Text → Logo-Konzept (Claude)
  ├── Referenzen → Style-Analyse
  └── Brand-Brief → Design-Regeln

Generation Pipeline
  ├── Ideogram v3 (Text-in-Image, einziges zuverlässiges Modell)
  ├── Varianten-Generation (5-10 Vorschläge)
  ├── Vektorisierung (Potrace / VTracer)
  └── SVG-Cleanup & Optimierung

Editor (Fabric.js + Spezialisiert)
  ├── Buchstaben-Spacing (Kerning)
  ├── Pfad-Bearbeitung (Bezier-Kurven)
  ├── Farb-Varianten
  ├── Größen-Test (Favicon → Billboard)
  └── Export: SVG, PNG, PDF, EPS
```

---

## AI Router (Kernkomponente)

```rust
// Pseudo-Code für die Router-Logik
struct AIRouter {
    api_clients: HashMap<Provider, APIClient>,  // Claude, Kling, Runway, Higgsfield, etc.
    cache: SemanticCache,
    budget: TokenBudgetManager,
    taste_engine: TasteEngine,
}

impl AIRouter {
    fn route_request(&self, request: AIRequest) -> AIResponse {
        // 1. Cache prüfen
        if let Some(cached) = self.cache.lookup(&request) {
            return cached;
        }
        
        // 2. Budget prüfen
        let budget = self.budget.remaining();
        
        // 3. Modell auswählen basierend auf:
        //    - Task-Typ (code, image, video, text)
        //    - Komplexität (simple → cheap model, complex → premium)
        //    - Budget-Constraints
        //    - Verfügbarkeit
        let model = self.select_model(&request, budget);
        
        // 4. meingeschmack/ Regeln als Context anhängen
        let enriched = self.taste_engine.enrich(request);
        
        // 5. Request senden
        let response = self.execute(model, enriched);
        
        // 6. Cachen
        self.cache.store(&request, &response);
        
        response
    }
}
```

---

## Datenfluss

```
User Input (Text/Bild/URL)
       │
       ▼
┌─── Input Parser ───┐
│  Typ erkennen       │
│  Modul zuordnen     │
│  Kontext aufbauen   │
└────────┬────────────┘
         │
         ▼
┌─── Taste Engine ───┐
│  meingeschmack/    │
│  laden & parsen    │
│  Regeln extrahieren│
│  Prompt erweitern  │
└────────┬───────────┘
         │
         ▼
┌─── AI Router ──────┐
│  Modell wählen      │
│  Cache prüfen       │
│  Budget prüfen      │
│  Request senden     │
└────────┬───────────┘
         │
         ▼
┌─── Post-Processing ┐
│  Format-Konversion  │
│  Quality Check      │
│  Thumbnail erzeugen │
└────────┬───────────┘
         │
         ▼
┌─── Preview/Editor ──┐
│  Canvas rendern      │
│  Interaktion         │
│  Iterieren           │
└────────┬────────────┘
         │
         ▼
      Export
```

---

## Dateistruktur des Projekts

```
TERRYBLEMACHINE/
├── CLAUDE.md                    # Anweisungen für Claude Code
├── docs/
│   ├── PROJEKTUEBERSICHT.md     # Dieses Dokument (Vision)
│   ├── ARCHITEKTUR.md           # Technische Architektur
│   ├── LLM-STRATEGIE.md        # Modell-Auswahl & Token-Management
│   ├── CLAUDE-CODE-SETUP.md     # Dev-Environment Setup
│   ├── ENTWICKLUNGSPLAN.md      # Phasen & Meilensteine
│   └── MEINGESCHMACK-SYSTEM.md  # Taste-Engine Spezifikation
├── meingeschmack/               # Style-Referenzen (Bilder, Videos, .md)
│   └── README.md                # Anleitung zum Befüllen
├── src/
│   ├── frontend/                # React + TypeScript
│   │   ├── components/          # UI-Komponenten
│   │   ├── modules/             # Website, 2D, 3D, Video, Typo
│   │   ├── canvas/              # Fabric.js, Three.js, PixiJS Integration
│   │   ├── stores/              # Zustand State Management
│   │   └── styles/              # Tailwind Config, Theme
│   └── backend/                 # Rust (Tauri)
│       ├── ai_router/           # Model Selection, Caching, Budget
│       ├── taste_engine/        # meingeschmack/ Parser & Regeln
│       ├── website_analyzer/    # Playwright, Style Extraction
│       ├── file_manager/        # Projekte, Assets, Export
│       └── api_clients/         # Claude, Kling, Runway, Higgsfield, Shotstack, Ideogram, Meshy, fal.ai, Replicate
├── src-tauri/                   # Tauri-Konfiguration
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       └── main.rs
├── package.json
├── tsconfig.json
├── tailwind.config.js
└── vite.config.ts
```
