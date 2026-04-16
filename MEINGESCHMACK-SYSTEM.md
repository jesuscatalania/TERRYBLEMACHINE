# TERRYBLEMACHINE — meingeschmack/ System (Taste Engine)

## Konzept

Der `meingeschmack/`-Ordner ist das stilistische Gedächtnis von TERRYBLEMACHINE. Er enthält Referenzmaterial und Regeln, die als Filter auf JEDEN generativen Output angewendet werden. Das System lernt deinen Geschmack und stellt sicher, dass alle Outputs konsistent deinem Stil entsprechen.

---

## Ordnerstruktur

```
meingeschmack/
├── README.md                  # Diese Anleitung
├── regeln/                    # Textbasierte Stilregeln
│   ├── farben.md              # Farbwelt: erlaubt, verboten, Paletten
│   ├── typografie.md          # Font-Präferenzen, Hierarchien
│   ├── layout.md              # Layout-Prinzipien, Whitespace, Grid
│   ├── effekte.md             # Schatten, Verläufe, Texturen, Animationen
│   ├── stimmung.md            # Allgemeine Ästhetik, Mood, Ton
│   └── verboten.md            # Explizite No-Gos (wird immer beachtet)
├── referenzen/                # Visuelle Referenzen
│   ├── bilder/                # Bilder die den gewünschten Stil zeigen
│   ├── videos/                # Video-Referenzen für Motion-Style
│   ├── websites/              # Screenshots von Referenz-Websites
│   └── logos/                 # Logo-Referenzen und Typografie-Beispiele
└── paletten/                  # Exportierte Farbpaletten
    ├── primaer.json           # Hauptfarben
    ├── sekundaer.json         # Akzentfarben
    └── verboten.json          # Verbotene Farben
```

---

## Regel-Dateien (Markdown-Format)

Jede `.md`-Datei in `regeln/` folgt einem einfachen Format:

### Beispiel: `farben.md`

```markdown
# Farbregeln

## Bevorzugt
- Warme, erdige Töne: Terracotta, Ocker, Sand
- Tiefes Dunkelblau als Kontrast (#1a1a2e)
- Goldakzente sparsam einsetzen

## Verboten
- Neonfarben jeglicher Art
- Knalliges Pink oder Magenta
- Reine Grundfarben (Rot #FF0000, Blau #0000FF, etc.)

## Paletten
- Primär: #2D3436, #D4A373, #FAEDCD, #FEFAE0
- Akzent: #E9C46A, #264653
- Text: #1A1A2E auf hellem Grund, #FEFAE0 auf dunklem Grund

## Kontext-Regeln
- Für Websites: Dunkler Hintergrund bevorzugt (Dark Mode first)
- Für Logos: Maximal 3 Farben
- Für Videos: Kinematische Color Grading (Teal & Orange erlaubt)
```

### Beispiel: `verboten.md`

```markdown
# Absolute No-Gos

Diese Regeln werden IMMER beachtet, unabhängig vom Kontext:

## Visuelle Stile
- Keine Clipart-Ästhetik
- Keine Stock-Photo-Looks (generisch lächelnde Menschen)
- Kein Skeuomorphismus (außer explizit angefragt)
- Keine überladenen Designs mit zu vielen Elementen

## Farben
- Keine Neonfarben
- Kein reines Weiß (#FFFFFF) als großflächiger Hintergrund
- Kein reines Schwarz (#000000) — stattdessen Off-Black (#1a1a1a)

## Typografie
- Keine Comic Sans, Papyrus, oder ähnliche
- Keine mehr als 2 verschiedene Font-Familien pro Design
- Kein Text unter 14px auf Websites

## Effekte
- Keine Bevel & Emboss
- Keine Lens Flares
- Kein übermäßiger Drop Shadow (subtil ist ok)

## Video
- Keine Slide-Transitions (Wipe, Dissolve etc. — nur Cut und Fade)
- Keine Stock-Music die nach Aufzug klingt
```

---

## Wie die Taste Engine funktioniert

### 1. Parsing-Phase (beim App-Start und bei Ordner-Änderungen)

```
meingeschmack/
    │
    ▼
┌─ File Watcher ─────────────────────────────┐
│  Erkennt neue/geänderte Dateien             │
│  Triggert Re-Analyse                        │
└────────────┬────────────────────────────────┘
             │
             ▼
┌─ Content Analyzer ──────────────────────────┐
│                                              │
│  .md Dateien → Regel-Parser                  │
│    Extrahiert: DO/DON'T/BEVORZUGT/VERBOTEN  │
│    Strukturiert als JSON-Regelwerk           │
│                                              │
│  Bilder → Claude Vision Analyse              │
│    Extrahiert: Farbpalette, Stil-Tags,       │
│    Komposition, Mood, Textur                 │
│                                              │
│  Videos → Frame-Sampling + Analyse           │
│    Extrahiert: Bewegungsstil, Pacing,        │
│    Color Grading, Transition-Typen           │
│                                              │
│  .json Paletten → Direkte Übernahme          │
│                                              │
└────────────┬────────────────────────────────┘
             │
             ▼
┌─ Style Profile (JSON) ─────────────────────┐
│  {                                          │
│    "colors": {                              │
│      "preferred": [...],                    │
│      "forbidden": [...],                    │
│      "palettes": { ... }                    │
│    },                                       │
│    "typography": { ... },                   │
│    "layout": { ... },                       │
│    "effects": {                             │
│      "allowed": [...],                      │
│      "forbidden": [...]                     │
│    },                                       │
│    "mood": ["minimalist", "warm", ...],     │
│    "negative_prompts": [...]                │
│  }                                          │
└────────────┬────────────────────────────────┘
             │
             ▼
        Cached im RAM
```

### 2. Anwendungs-Phase (bei jedem generativen Request)

```
User Prompt: "Erstelle ein Hero-Banner für eine Kaffee-Website"
       │
       ▼
┌─ Prompt Enricher ──────────────────────────┐
│                                             │
│  Original-Prompt                            │
│  + Style Profile (relevante Teile)          │
│  + Kontext-spezifische Regeln               │
│  + Negative Prompts                         │
│                                             │
│  Ergebnis (für Image-API):                  │
│  "Hero banner for a coffee website.         │
│   Style: warm, earthy, minimalist.          │
│   Colors: terracotta, ocher, deep navy.     │
│   Typography: modern sans-serif, max 2.     │
│   Mood: sophisticated, artisanal.           │
│   Negative: neon, clipart, stock photo,     │
│   bevel, lens flare, pure white bg"         │
│                                             │
│  Ergebnis (für Code-API):                   │
│  System-Prompt-Erweiterung mit CSS-         │
│  Variablen, Font-Empfehlungen,              │
│  Dark-Mode-First Anweisung                  │
│                                             │
└─────────────────────────────────────────────┘
```

### 3. Validierungs-Phase (nach Generation)

```
Generierter Output
       │
       ▼
┌─ Style Validator ──────────────────────────┐
│                                             │
│  Prüft (via Claude Vision für Bilder):      │
│  - Enthält keine verbotenen Farben?         │
│  - Passt zur definierten Stimmung?          │
│  - Keine verbotenen Effekte?                │
│                                             │
│  Bei Code prüft:                            │
│  - CSS-Variablen gegen Farbregeln           │
│  - Font-Familien gegen Erlaubte             │
│  - Kein #FFFFFF / #000000 wo verboten       │
│                                             │
│  Ergebnis:                                  │
│  ✅ PASS → Output an User                   │
│  ⚠️ WARN → Output + Hinweis an User        │
│  ❌ FAIL → Automatisch re-generieren       │
│            (max 2 Versuche)                 │
│                                             │
└─────────────────────────────────────────────┘
```

---

## Befüllung des Ordners

### Schnellstart

1. **3-5 Referenzbilder** in `referenzen/bilder/` legen, die deinen Stil repräsentieren
2. **`regeln/verboten.md`** mit den wichtigsten No-Gos füllen
3. **`regeln/farben.md`** mit bevorzugten Farbpaletten füllen
4. Fertig — die Engine baut daraus ein erstes Style Profile

### Fortgeschritten

- Eigene JSON-Paletten in `paletten/` für präzise Farbkontrolle
- Video-Referenzen für konsistente Motion-Stile
- Separate Regel-Dateien pro Modul (z.B. `regeln/websites.md`, `regeln/videos.md`)
- Kontext-Regeln: "Für Logos immer X, für Websites immer Y"

### Iteration

Der Ordner ist lebendig — du kannst jederzeit:
- Neue Referenzen hinzufügen
- Regeln ändern oder verschärfen
- Die Engine analysiert Änderungen automatisch
- Das Style Profile wird aktualisiert

---

## Technische Implementation

### Bild-Analyse (Claude Vision)

```
Prompt an Claude Vision:
"Analysiere dieses Bild und extrahiere folgende Stil-Informationen als JSON:
- dominant_colors: Die 5 dominantesten Farben als HEX
- mood: 3-5 Adjektive die die Stimmung beschreiben
- style_tags: Stilistische Kategorien (minimalist, brutalist, etc.)
- composition: Layout-Beschreibung (zentriert, asymmetrisch, etc.)
- textures: Erkannte Texturen (glatt, rau, organisch, etc.)
- lighting: Licht-Charakter (warm, kalt, dramatisch, flach, etc.)
- typography_style: Falls Text vorhanden, Beschreibung des Typo-Stils"
```

### Regel-Parsing

Regeln in `.md`-Dateien werden anhand von Keywords geparst:

- `## Bevorzugt` / `## Preferred` → Positive Prompt-Erweiterung
- `## Verboten` / `## Forbidden` → Negative Prompts
- `## Kontext-Regeln` → Modul-spezifische Anweisungen
- HEX-Codes (#XXXXXX) → Automatisch als Farbwerte erkannt
- Aufzählungszeichen → Einzelne Regeln

### Performance

- Style Profile wird gecacht und nur bei Ordner-Änderungen neu berechnet
- Bild-Analyse (teuerste Operation) wird nur einmal pro Bild durchgeführt
- Ergebnis wird als `.analysis.json` neben dem Bild gespeichert
- Bei 20 Referenzbildern: einmalig ~$0.50 für Analyse, danach gratis
