# TERRYBLEMACHINE — LLM-Strategie & Kosten-Management

## Das Tool-Paket (11 Services)

TERRYBLEMACHINE nutzt 11 spezialisierte Services. Jeder hat eine klare Rolle — kein Service ist redundant.

### Abos (Fixkosten)

| # | Service | Plan | Monat | Jahr | Rolle |
|---|---------|------|------:|-----:|-------|
| 1 | **Claude Max** | Max | $100 | $1.200 | Code-Gen, Reasoning, Website-Erstellung, Analyse, Vision |
| 2 | **Kling AI** | 1.000-Unit-Paket | ~$98 | ~$1.176 | Video-Generierung (Qualitätsführer) |
| 3 | **Runway** | Pro | $28 | $336 | Video mit Motion Control, 4K Export |
| 4 | **Higgsfield** | Plus | $39 | $468 | Video Multi-Modell-Zugang (Kling, Veo, etc.) |
| 5 | **Shotstack** | Subscription | $39 | $468 | Programmatische Video-Montage (Clip-Assembly) |
| 6 | **Ideogram** | Plus | $15 | $180 | Logos & Typografie (Text in Bildern) |
| 7 | **Meshy** | Pro | ~$10 | ~$114 | 3D-Generierung (Text/Image-to-3D, PBR) |

### Pay-per-Use (variable Kosten, kein Abo verfügbar)

| # | Service | Geschätzt/Mo | Jahr | Rolle |
|---|---------|------------:|-----:|-------|
| 8 | **fal.ai** | ~$16 | ~$192 | Bild-Generierung (Flux 2 Pro, SDXL, Upscaling) |
| 9 | **Replicate** | ~$9 | ~$108 | Spezial-/Nischenmodelle, Experimente |

### Kostenlos (Open Source)

| # | Service | Rolle |
|---|---------|-------|
| 10 | **Remotion** | Lokales Video-Rendering (React → MP4), Kinetic Typography |
| 11 | **OpenMontage** | Architektur-Referenz für Video-Pipeline-Orchestrierung |

### Gesamtkosten bei täglicher Nutzung

| Kategorie | Monat | Jahr |
|-----------|------:|-----:|
| Fix (Abos) | $329 | $3.942 |
| Variabel (Pay-per-Use) | ~$25 | ~$300 |
| Kostenlos | $0 | $0 |
| **TOTAL** | **~$354/Mo** | **~$4.242/Jahr** |
| **Pro Tag** | **~$11.80** | |

---

## Modell-Zuordnung nach Aufgabe

### Code-Generierung (Website-Builder)

| Aufgabe | Modell | Kosten |
|---------|--------|--------|
| Komplexe Website von Grund auf | Claude Opus 4.6 (via Claude Max) | Inkl. im Abo |
| Einfache Komponenten | Claude Sonnet 4.6 (via Claude Max) | Inkl. im Abo |
| Quick Fixes, CSS-Anpassungen | Claude Haiku 4.5 (via Claude Max) | Inkl. im Abo |
| Website-Analyse (Vision) | Claude Sonnet 4.6 Vision (via Claude Max) | Inkl. im Abo |

**Routing-Logik**: Komplexität wird geschätzt anhand von Token-Zahl des Inputs, Anzahl der Dateien/Komponenten, und ob neue Architektur oder Iteration.

### Bild-Generierung (2D-Grafik & Bild-Erstellung)

| Aufgabe | Modell | Provider | Kosten/Bild |
|---------|--------|----------|-------------|
| Hochqualitativ, Standard | Flux 2 Pro | fal.ai | ~$0.03 |
| Budget/Iteration/Entwürfe | SDXL | fal.ai | ~$0.003 |
| Spezial-/Nischenmodelle | Diverse | Replicate | ~$0.025–0.05 |
| Text im Bild (Logos etc.) | Ideogram v3 | Ideogram (Abo) | Inkl. im Abo |
| Upscaling | Real-ESRGAN | fal.ai | ~$0.01 |
| Inpainting | Flux Fill | fal.ai | ~$0.03 |
| Image-to-Image | Flux Redux | fal.ai | ~$0.03 |

### Pseudo-3D-Grafik & -Bilder

| Aufgabe | Modell | Provider | Kosten |
|---------|--------|----------|--------|
| 3D-Style Bild | Flux 2 Pro + LoRA | fal.ai | ~$0.03 |
| Depth Map | Depth-Anything v2 | fal.ai | ~$0.01 |
| Quick 3D Mesh Preview | TripoSR | Lokal (Open Source) | Gratis |
| Text-to-3D mit PBR Texturen | Meshy Text-to-3D | Meshy (Abo) | Inkl. im Abo |
| Image-to-3D mit Texturen | Meshy Image-to-3D | Meshy (Abo) | Inkl. im Abo |
| 3D Asset Rigging/Animation | Meshy Rigging | Meshy (Abo) | Inkl. im Abo |

**Meshy Details**: Export als GLB, FBX, OBJ, USDZ, STL, Blend. Pro (~$10/Mo) gibt 1.000 Credits. Ideal für Pseudo-3D weil generierte Meshes direkt in Three.js geladen werden können.

### Video-Generierung

| Aufgabe | Modell | Provider | Kosten |
|---------|--------|----------|--------|
| Hochqualität (Echtfilm-Look) | Kling 2.0 | Kling (Paket) | ~$0.084–0.168/Unit |
| Motion Control nötig | Runway Pro | Runway (Abo) | Inkl. im Abo (Credits) |
| Multi-Modell (Kling, Veo etc.) | Diverse | Higgsfield (Abo) | Inkl. im Abo (Credits) |
| Kinetic Typography | Remotion | Lokal | Gratis |
| Motion Graphics | Remotion | Lokal | Gratis |
| 3D-Animation | Remotion + Three.js | Lokal | Gratis |
| Clip-Montage & Assembly | Shotstack | Shotstack (Abo) | Inkl. im Abo ($0.20/Min) |

**Video-Pipeline (OpenMontage-Pattern)**: Das Video-Modul folgt dem OpenMontage-Architekturmuster — ein Orchestrator-Agent der spezialisierte Tools koordiniert: Kling/Runway/Higgsfield für KI-Generation, Remotion für lokales Rendering, Shotstack für Cloud-Assembly.

**Remotion** übernimmt alles was programmatisch renderbar ist (Kinetic Typography, Motion Graphics, Datenvisualisierungen, React-basierte Animationen) — kostenlos und lokal. KI-generierte Video-Clips (Echtfilm, komplexe Animationen) kommen von Kling/Runway/Higgsfield.

### Typografie & Logos

| Aufgabe | Modell | Provider | Kosten |
|---------|--------|----------|--------|
| Logo mit Text | Ideogram v3 | Ideogram (Abo) | Inkl. im Abo |
| Varianten generieren | Ideogram v3 | Ideogram (Abo) | Inkl. im Abo |
| SVG-Vektorisierung | VTracer / Potrace | Lokal | Gratis |

### Reasoning & Planung

| Aufgabe | Modell | Provider | Kosten |
|---------|--------|----------|--------|
| Storyboard-Erstellung | Claude Sonnet 4.6 | Claude Max | Inkl. im Abo |
| Style-Analyse | Claude Sonnet 4.6 Vision | Claude Max | Inkl. im Abo |
| Komplexe Konzeptplanung | Claude Opus 4.6 | Claude Max | Inkl. im Abo |
| Einfache Klassifizierung | Claude Haiku 4.5 | Claude Max | Inkl. im Abo |

---

## Wann welchen Video-Service nutzen

TERRYBLEMACHINE hat vier Video-Engines — hier die Entscheidungslogik:

```
User will Video erstellen
       │
       ├─ Kinetic Typography, Motion Graphics, Datenvis?
       │   → REMOTION (lokal, gratis, React-basiert)
       │
       ├─ KI-generierter Echtfilm-Look, höchste Qualität?
       │   → KLING AI (Qualitätsführer)
       │
       ├─ Präzise Motion Control oder Kamera-Steuerung nötig?
       │   → RUNWAY PRO (Motion Brush, 4K)
       │
       ├─ Verschiedene Modelle testen, experimentieren?
       │   → HIGGSFIELD (Multi-Modell: Kling, Veo, etc.)
       │
       └─ Clips zusammenschneiden, Overlays, Audio?
           → SHOTSTACK (JSON-API für Assembly)
           → oder REMOTION (wenn React-basiert renderbar)
```

---

## Wann fal.ai vs. Replicate

```
User will Bild generieren
       │
       ├─ Standard-Workflow (Flux, SDXL, Upscaling, Inpainting)?
       │   → FAL.AI (schneller, günstiger, optimierte Infrastruktur)
       │
       └─ Spezialmodell, experimentelles LoRA, Nischen-Use-Case?
           → REPLICATE (tausende Modelle, inkl. Community-Uploads)
```

---

## Token-Management-System

### 5-Stufen-Optimierung

#### Stufe 1: Prompt Caching (60-90% Ersparnis)
```
Implementierung:
- System-Prompts und meingeschmack/-Regeln als cached prefix
- Claude: Automatisches Caching bei wiederholten Prefixes
- Semantic Cache im Rust-Backend: Ähnliche Requests → cached Response
- TTFT sinkt von ~5s auf <200ms bei Cache-Hit

Erwartete Ersparnis: 60-90% auf wiederkehrende Patterns
```

#### Stufe 2: Model Routing (40-70% Ersparnis)
```
Implementierung:
- Classifier (Haiku) bestimmt Komplexität (~$0.001 pro Entscheidung)
- Simple Tasks → Haiku
- Medium Tasks → Sonnet
- Complex Tasks → Opus

Routing-Kriterien:
- Prompt-Länge und Aufgabentyp
- Erwartete Output-Komplexität
- User-Feedback (wenn Ergebnis schlecht → nächsthöheres Modell)

Erwartete Ersparnis: 40-70%
```

#### Stufe 3: Prompt-Optimierung (20-40% Ersparnis)
```
Implementierung:
- Templates pro Aufgabentyp mit minimaler Redundanz
- Strukturierte Outputs (JSON) statt Freitext wo möglich
- Komprimierte Context-Summaries statt volle Konversation

Erwartete Ersparnis: 20-40% Token-Reduktion
```

#### Stufe 4: Context Compaction (50-70% Token-Reduktion)
```
Implementierung:
- Rolling Summary: Ältere Konversations-Teile zusammenfassen
- RAG für meingeschmack/: Nur relevante Regeln laden, nicht alle
- Selective Context: Nur benötigte Code-Teile senden, nicht ganzes Projekt

Erwartete Ersparnis: 50-70% bei langen Sessions
```

#### Stufe 5: Lokale Verarbeitung bevorzugen
```
Implementierung:
- Remotion statt Shotstack wo möglich (spart $0.20/Min pro Video)
- TripoSR statt Meshy für schnelle 3D-Previews
- VTracer/Potrace lokal statt Cloud-Vektorisierung
- Depth-Anything lokal statt fal.ai wo Performance reicht

Erwartete Ersparnis: 30-50% der Video- und 3D-Kosten
```

### Kombinierter Effekt

Bei konsequenter Anwendung aller 5 Stufen: **70-85% Kostenreduktion** gegenüber naivem API-Aufruf ohne Qualitätsverlust.

---

## Budget-Dashboard (geplantes Feature)

Die App zeigt dem User transparent:
- **Aktueller Verbrauch** pro Session, Tag, Monat
- **Kosten pro Modul** (Website, 2D, Video, etc.)
- **Abo-Status** aller Services (Credits verbraucht/übrig)
- **Sparquote** durch Caching, Routing & lokale Verarbeitung
- **Budget-Limits** (konfigurierbar, Warnung bei 80%)

---

## Kein separater Agent nötig

**Frage: Brauchen wir Hermes (Nous Research) oder ähnliche Agent-Frameworks?**

**Antwort: Nein, nicht für v1.**

Begründung:
1. Claude Code + Superpowers-Skill bietet bereits einen 7-Phasen-Workflow
2. Tauri's Rust-Backend übernimmt die Orchestrierung der API-Aufrufe
3. Der AI Router ist eine deterministische State Machine, kein autonomer Agent
4. OpenMontage dient als Architektur-Referenz, nicht als Dependency

**Wann ein Agent-Framework sinnvoll wird** (v2+):
- Wenn TERRYBLEMACHINE autonom komplexe Multi-Step-Aufgaben ausführen soll
- Wenn parallele Sub-Agents (Design-Validierung, Code-Review, Testing) nötig werden
- Dann: Claude Agent SDK oder LangGraph evaluieren

---

## API-Key Management

```
Alle Keys werden sicher in macOS Keychain gespeichert.
Kein Key jemals in Code, Config-Dateien oder Git.

Benötigte Keys:
- Anthropic (Claude Max — für programmatischen Zugriff falls nötig)
- Kling AI
- Runway
- Higgsfield
- Shotstack
- Ideogram
- Meshy
- fal.ai
- Replicate

Setup-Flow in der App:
1. User öffnet Settings
2. Gibt API-Keys ein
3. Keys werden im macOS Keychain verschlüsselt gespeichert
4. Rust-Backend liest Keys zur Laufzeit aus Keychain
5. Keys werden nie an Frontend/WebView übergeben
```
