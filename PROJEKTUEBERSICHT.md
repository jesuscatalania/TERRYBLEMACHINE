# TERRYBLEMACHINE — Projektübersicht

## Vision

TERRYBLEMACHINE ist ein lokal laufendes, KI-gestütztes Designtool für macOS, das kreative Produktion radikal vereinfacht. Es vereint Website-Erstellung, 2D-Grafik, Pseudo-3D-Grafik, Video-Produktion und Typografie/Logo-Design in einer einzigen, elegant designten Oberfläche — angetrieben von den besten verfügbaren KI-Modellen, orchestriert durch ein intelligentes Routing-System.

Das Tool lernt deinen persönlichen Geschmack durch den `meingeschmack/`-Ordner und wendet diesen als Regelwerk auf alle Outputs an.

---

## Kernfähigkeiten

### 1. Website-Erstellung
- **Input**: Freitext-Beschreibungen, Bild-Uploads (Mockups, Screenshots), URLs zur Analyse
- **Output**: Vollständiger, deploymentfähiger Code (HTML/CSS/JS, React, Next.js, Tailwind)
- **Besonderheit**: URL-Analyse extrahiert Design-Tokens (Farben, Fonts, Spacing, Schatten, Layout-Struktur) und kann Websites stilistisch 1:1 reproduzieren
- **Engine**: Claude Opus/Sonnet für Code-Generierung, Playwright-basierter Scraper für Website-Analyse

### 2. 2D-Grafik & Bild-Erstellung
- **Input**: Freitext, Bild-Uploads (Referenzen, Sketches), Style-Vorgaben aus `meingeschmack/`
- **Output**: PNG, JPEG, SVG, WebP, PDF, GIF in beliebiger Auflösung
- **Modelle**: Flux 2 Pro (Preis-Leistung), SDXL (Budget), Ideogram v3 (wenn Text enthalten)
- **Features**: Image-to-Image, Inpainting, Style Transfer, Upscaling

### 3. Pseudo-3D-Grafik & -Bilder
- **Input**: Freitext, Bild-Referenzen, Stil-Beschreibung (isometrisch, Depth, Parallax)
- **Output**: PNG, JPEG, SVG, WebP, PDF, GIF — statische Bilder mit 3D-Optik, keine funktionalen 3D-Modelle
- **Ansatz**: Kombination aus Image-Generation mit Depth-Estimation + Three.js für Preview-Rendering
- **Modelle**: Flux 2 Pro mit spezialisierten LoRAs, Meshy (Text-to-3D / Image-to-3D mit PBR-Texturen), TripoSR für schnelle Mesh-Previews (lokal), Depth-Anything für Tiefenkarten

### 4. Video-Produktion
- **Input**: Freitext, Bild-to-Video, Referenzvideos, Storyboard-Beschreibungen
- **Output**: MP4/WebM in verschiedenen Auflösungen und Stilen
- **Stile**: Echtfilm, Animation, Motion Graphics, Kinetic Typography, 2D, Pseudo-3D
- **KI-Generierung**: Kling (Qualitätsführer), Runway Pro (Motion Control, 4K), Higgsfield (Multi-Modell)
- **Lokales Rendering**: Remotion (React → MP4, für Kinetic Typography, Motion Graphics, 3D-Animation)
- **Post-Production**: Shotstack (JSON-API für Clip-Montage, Transitions, Audio)
- **Pipeline**: Storyboard → KI-Generation ODER Remotion-Rendering → Shotstack Assembly → Export
- **Architektur-Referenz**: OpenMontage (Open-Source Agent-Orchestrierung für Video-Pipelines)

### 5. Typografie & Logo-Design
- **Input**: Freitext (Firmenname, Stil, Branche), Bild-Referenzen
- **Output**: SVG-Logos, Typografie-Kompositionen, Brand-Assets
- **Modelle**: Ideogram v3 (einziges Modell mit zuverlässiger Text-Darstellung in Bildern)
- **Pipeline**: KI-Generierung → SVG-Vektorisierung → manuelle Feinabstimmung im Editor

---

## Der `meingeschmack/`-Ordner

Ein zentrales Differenzierungsmerkmal von TERRYBLEMACHINE. Dieser Ordner enthält:

- **Bilder**: Referenzbilder die den gewünschten Stil definieren
- **Videos**: Referenzvideos für Motion-Styles
- **`.md`-Dateien**: Regelwerke in natürlicher Sprache (z.B. "Verwende niemals Neonfarben", "Bevorzuge minimalistische Layouts mit viel Whitespace")

Das System analysiert diese Referenzen und extrahiert daraus:
- Farbpaletten (erlaubt/verboten)
- Typografie-Präferenzen
- Layout-Muster
- Effekt-Regeln (Schatten, Verläufe, Texturen)
- Stil-Kategorien (minimalistisch, brutalistisch, verspielt, etc.)

Diese Regeln werden als System-Prompt-Erweiterung an jedes generative Modell übergeben.

---

## Zielgruppe

Kreative Professionals und Entwickler, die:
- Schnell hochwertige visuelle Assets produzieren wollen
- Einen konsistenten visuellen Stil über alle Medien hinweg pflegen
- Lokale Kontrolle über ihre Tools und Daten bevorzugen
- Die Lücke zwischen Idee und fertigem Output minimieren wollen

---

## Technische Eckdaten

- **Plattform**: macOS (M3/M4 MacBook Pro optimiert)
- **Framework**: Tauri v2 (Rust-Backend, Web-Frontend)
- **Frontend**: React + TypeScript + Tailwind CSS
- **Rendering**: Fabric.js (2D-Editor) + Three.js (Pseudo-3D-Preview) + PixiJS (Performance)
- **AI-Services (Abos)**: Claude Max, Kling, Runway Pro, Higgsfield, Shotstack, Ideogram, Meshy Pro
- **AI-Services (Pay-per-Use)**: fal.ai, Replicate
- **Lokale Tools**: Remotion (Video-Rendering), TripoSR (3D-Preview), VTracer (SVG)
- **Architektur-Referenz**: OpenMontage (Video-Pipeline-Pattern)
- **Sprache der UI**: Deutsch (mit Option auf Englisch)

---

## Abgrenzung — Was TERRYBLEMACHINE NICHT ist

- Kein Figma-Klon (kein kollaboratives Design-Tool)
- Kein echtes 3D-Tool (keine funktionalen 3D-Modelle, nur visuelle 3D-Optik)
- Kein Video-Editor (kein Timeline-basiertes Editing, sondern KI-gestützte Generation)
- Kein Hosting-Service (generiert Code, deployed nicht)
