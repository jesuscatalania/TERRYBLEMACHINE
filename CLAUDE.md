# TERRYBLEMACHINE — Anweisungen für Claude Code

## Projektbeschreibung

TERRYBLEMACHINE ist ein lokales KI-Designtool für macOS (M3/M4 MacBook Pro), gebaut mit Tauri v2 (Rust-Backend) + React + TypeScript (Frontend). Es erzeugt Websites, 2D-Grafiken, Pseudo-3D-Grafiken, Videos und Typografie/Logos mithilfe verschiedener AI-APIs.

## Architektur

Lies `docs/ARCHITEKTUR.md` für die vollständige technische Architektur. Kurzfassung:

- **Frontend**: React 18+ mit TypeScript, Tailwind CSS, Zustand (State), Fabric.js + Three.js + PixiJS (Canvas)
- **Backend**: Rust via Tauri v2 — AI Router, API Clients, File Management, Taste Engine
- **APIs (Abos)**: Claude Max, Kling, Runway Pro, Higgsfield, Shotstack, Ideogram, Meshy Pro
- **APIs (Pay-per-Use)**: fal.ai, Replicate
- **Lokal/Gratis**: Remotion (Video-Rendering), OpenMontage (Architektur-Referenz), TripoSR, VTracer
- **Datenfluss**: User Input → Taste Engine (meingeschmack/) → AI Router → Model → Post-Processing → Preview → Export

## Entwicklung

- `docs/ENTWICKLUNG-SCHRITT-FUER-SCHRITT.md` — **Hauptanleitung**: 40 Schritte mit exakten Befehlen und Prüfungen
- `docs/ENTWICKLUNGSPLAN.md` — Phasenübersicht und Meilensteine

**Aktuelle Phase**: Phase 0 — Foundation (Projekt-Setup, Tooling, CI/CD)

Lies `docs/ENTWICKLUNG-SCHRITT-FUER-SCHRITT.md` für den nächsten Schritt.

## Coding-Standards

### TypeScript (Frontend)
- Strict Mode aktiviert
- Funktionale Komponenten mit Hooks (keine Klassen)
- Zustand für State Management (kein Redux, kein Context-Overhead)
- Biome als Linter und Formatter (kein ESLint/Prettier)
- Benenne Komponenten PascalCase, Hooks mit use-Prefix, Utils camelCase
- Barrel-Exports (index.ts) pro Feature-Ordner
- Tests mit Vitest + Testing Library, Dateiname: `*.test.tsx`

### Rust (Backend)
- Idiomatisches Rust: Result<T, E> statt Panics, ? Operator
- Async/Await mit Tokio Runtime
- Serde für Serialisierung (JSON)
- Tauri Commands als #[tauri::command] exportiert
- Error-Types pro Modul (thiserror)
- Tests im selben File mit #[cfg(test)] mod tests

### CSS
- Tailwind CSS Utility-First — keine separaten CSS-Dateien außer für Tailwind-Config
- Custom Theme in tailwind.config.js (Farben, Fonts, Spacing)
- Dark Mode als Default (class-basiert)
- Keine !important, keine inline styles

### Dateistruktur
```
src/frontend/
├── components/     # Wiederverwendbare UI-Komponenten
├── modules/        # Feature-Module (website, graphic2d, graphic3d, video, typography)
│   └── [modul]/
│       ├── components/   # Modul-spezifische Komponenten
│       ├── hooks/        # Modul-spezifische Hooks
│       ├── stores/       # Zustand Stores
│       └── index.tsx     # Modul-Export
├── canvas/         # Fabric.js, Three.js, PixiJS Integration
├── stores/         # Globale Stores
├── hooks/          # Globale Hooks
├── utils/          # Helper-Funktionen
├── types/          # TypeScript Type Definitions
└── styles/         # Tailwind Config, globale Styles

src/backend/ (Rust, in src-tauri/src/)
├── ai_router/      # Model Selection, Caching, Budget
├── taste_engine/   # meingeschmack/ Parser
├── website_analyzer/ # Playwright, Style Extraction
├── file_manager/   # Projekte, Assets, Export
├── api_clients/    # Abo-APIs + Pay-per-Use APIs
│   ├── claude.rs       # Claude Max (Anthropic)
│   ├── kling.rs        # Kling AI (Video, Abo)
│   ├── runway.rs       # Runway Pro (Video, Abo)
│   ├── higgsfield.rs   # Higgsfield (Video, Abo)
│   ├── shotstack.rs    # Shotstack (Video-Montage, Abo)
│   ├── ideogram.rs     # Ideogram (Logos/Typo, Abo)
│   ├── meshy.rs        # Meshy Pro (3D, Abo)
│   ├── fal.rs          # fal.ai (Bilder, Pay-per-Use)
│   └── replicate.rs    # Replicate (Spezialmodelle, Pay-per-Use)
├── video_renderer/ # Remotion Integration (lokal)
└── models/         # Shared Rust Types
```

## Wichtige Regeln

1. **Kein API-Key in Code oder Config-Dateien** — immer macOS Keychain via security-framework Crate
2. **meingeschmack/ immer beachten** — jeder generative Output muss durch die Taste Engine laufen
3. **Fehler graceful behandeln** — User-facing Errors als Toast/Notification, nie als Crash
4. **Caching zuerst prüfen** — vor jedem API-Aufruf Cache checken
5. **Budget prüfen** — vor teuren API-Aufrufen (Video, Opus) Budget-Check
6. **Tests zuerst** — TDD-Ansatz: Test schreiben, dann implementieren
7. **Keine unnötigen Dependencies** — jede neue Dependency muss begründet sein
8. **Tauri IPC minimal halten** — große Daten (Bilder, Videos) über File-System tauschen, nicht über IPC

## Commit-Konventionen

```
feat(modul): Beschreibung     # Neues Feature
fix(modul): Beschreibung      # Bugfix
refactor(modul): Beschreibung # Code-Verbesserung
test(modul): Beschreibung     # Tests
docs: Beschreibung            # Dokumentation
chore: Beschreibung           # Build, CI, Dependencies
```

Module: `core-ui`, `ai-router`, `taste-engine`, `website`, `graphic2d`, `graphic3d`, `video`, `typography`, `export`

## Dokumentation

- `docs/PROJEKTUEBERSICHT.md` — Vision und Gesamtbeschreibung
- `docs/ARCHITEKTUR.md` — Technische Architektur und Datenfluss
- `docs/LLM-STRATEGIE.md` — Modell-Auswahl, Kosten, Token-Management
- `docs/CLAUDE-CODE-SETUP.md` — Dev-Setup und Workflow
- `docs/ENTWICKLUNGSPLAN.md` — Phasen und Meilensteine
- `docs/ENTWICKLUNG-SCHRITT-FUER-SCHRITT.md` — Schritt-für-Schritt Befehle mit Prüfungen
- `docs/MEINGESCHMACK-SYSTEM.md` — Taste Engine Spezifikation

Lies die relevanten Docs bevor du ein Feature implementierst.
