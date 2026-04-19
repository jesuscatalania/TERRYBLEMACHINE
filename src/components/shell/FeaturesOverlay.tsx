import type { ReactNode } from "react";
import { Modal } from "@/components/ui/Modal";

export interface FeaturesOverlayProps {
  open: boolean;
  onClose: () => void;
}

/**
 * "Was kann das?" — vollständige Feature-Dokumentation in deutscher Sprache.
 *
 * Gedacht als Einmal-Lesen-Referenz für neue Nutzer: deckt alle fünf Module
 * plus Chat ab, listet sämtliche Modelle mit Stärken/Schwächen/Kosten,
 * erklärt die drei Ebenen der Modell-Auswahl, das Optimize-Flow, Claude-
 * Transport, Shortcuts, Keychain-Notizen und bekannte Einschränkungen.
 *
 * Rendering-Prinzipien (gespiegelt von ShortcutHelpOverlay):
 *   - Kopfzeilen in `font-mono uppercase tracking-label` + `text-2xs`
 *   - Listen-Inhalt in `text-2xs text-neutral-dark-200`
 *   - Slugs/Shortcuts/Modell-Namen als `<code>` mit `accent-500`
 *   - Tabellen als vertikale Definition-List-Karten (nicht `<table>`), damit
 *     schmale Viewports nicht horizontal scrollen müssen
 */
export function FeaturesOverlay({ open, onClose }: FeaturesOverlayProps) {
  return (
    <Modal open={open} onClose={onClose} title="Was kann das?" maxWidth={960}>
      <div className="max-h-[75vh] overflow-y-auto pr-2">
        <div className="flex flex-col gap-6 text-2xs leading-relaxed text-neutral-dark-200">
          <Intro />
          <ModulesSection />
          <ModelTiersSection />
          <ModelEcosystemSection />
          <OptimizeSection />
          <ClaudeTransportSection />
          <OverrideCheatsheetSection />
          <ShortcutsSection />
          <KeysAndCostsSection />
          <ProjectsSection />
          <LimitationsSection />
        </div>
      </div>
    </Modal>
  );
}

// ---- Shared building blocks ------------------------------------------------

function SectionHeading({ children }: { children: ReactNode }) {
  return (
    <h2 className="font-mono text-2xs uppercase tracking-label text-accent-500">{children}</h2>
  );
}

function SubHeading({ children }: { children: ReactNode }) {
  return (
    <h3 className="font-mono text-2xs uppercase tracking-label text-neutral-dark-300">
      {children}
    </h3>
  );
}

function Pill({ children }: { children: ReactNode }) {
  return (
    <code className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-1.5 py-0.5 font-mono text-2xs text-accent-500">
      {children}
    </code>
  );
}

function ModelCard({
  name,
  slugs,
  strengths,
  weaknesses,
  cost,
}: {
  name: string;
  slugs: string[];
  strengths: string;
  weaknesses: string;
  cost: string;
}) {
  return (
    <div className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-950/40 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-mono text-2xs uppercase tracking-label text-neutral-dark-100">
          {name}
        </span>
        {slugs.map((s) => (
          <Pill key={s}>{s}</Pill>
        ))}
      </div>
      <dl className="mt-2 grid grid-cols-1 gap-1 sm:grid-cols-[6rem_1fr]">
        <dt className="font-mono uppercase tracking-label text-neutral-dark-400">Stärken</dt>
        <dd className="text-neutral-dark-200">{strengths}</dd>
        <dt className="font-mono uppercase tracking-label text-neutral-dark-400">Schwächen</dt>
        <dd className="text-neutral-dark-200">{weaknesses}</dd>
        <dt className="font-mono uppercase tracking-label text-neutral-dark-400">Kosten</dt>
        <dd className="text-neutral-dark-200">{cost}</dd>
      </dl>
    </div>
  );
}

// ---- Sections --------------------------------------------------------------

function Intro() {
  return (
    <section className="flex flex-col gap-2">
      <p>
        Lokales KI-Designtool für macOS. Nutzt{" "}
        <strong className="text-neutral-dark-50">Claude</strong> über deine Subscription für
        Text/Code-Aufgaben (null zusätzliche Kosten) und Pay-per-Use-APIs für Bilder, Videos und 3D.
        Jeder generative Output läuft durch die persönliche{" "}
        <strong className="text-neutral-dark-50">Taste-Engine</strong> (
        <code className="font-mono text-accent-500">meingeschmack/</code>) und wird an deinen Stil
        angepasst.
      </p>
    </section>
  );
}

function ModulesSection() {
  return (
    <section className="flex flex-col gap-3">
      <SectionHeading>Die 5 Module + Chat</SectionHeading>

      <div className="flex flex-col gap-3">
        <ModuleBlock
          emoji="🖼️"
          title="Graphic 2D — Bilder, Illustrationen, Fotos"
          bullets={[
            "Prompt eintippen → Generate → 4 Varianten zur Auswahl",
            "Canvas-Editor mit Layers, Text, Filtern (Blur/Sharpen/Brightness/Contrast/Saturation)",
            "Masken-Werkzeug für Selective-Inpaint",
            "Marquee- und Lasso-Auswahl",
            "Export: PNG, JPG, WebP, SVG (bei Vektor-Content)",
          ]}
        />
        <ModuleBlock
          emoji="🎲"
          title="Graphic 3D — Meshes + Depth-Maps"
          bullets={[
            "Text-zu-3D: Beschreibung → 3D-Mesh (glTF)",
            "Bild-zu-3D: Referenzbild → 3D-Mesh",
            "Depth-Map: Bild → Tiefenkarte (PNG, brighter = closer)",
            "Glb/Gltf Viewer mit Orbit-Control",
            "Export: glTF, OBJ",
          ]}
        />
        <ModuleBlock
          emoji="🎬"
          title="Video — Storyboard → Segmente → Montage"
          bullets={[
            <>
              Drei-Stufen-Flow:
              <ol className="mt-1 ml-4 list-decimal space-y-1">
                <li>
                  <strong className="text-neutral-dark-100">Storyboard:</strong> Prompt + Template
                  (Commercial, Music Video, Narrative, etc.) → Claude generiert Shot-Liste (typisch
                  5 Shots mit Description + Dauer)
                </li>
                <li>
                  <strong className="text-neutral-dark-100">Segmente rendern:</strong>{" "}
                  <code className="font-mono text-accent-500">Generate All</code> oder per-Shot →
                  jeder Shot wird zum ~5-10s Video
                </li>
                <li>
                  <strong className="text-neutral-dark-100">Assemble:</strong> Cloud (Shotstack, mit
                  Audio + Overlays) oder Lokal (Remotion, programmierbar)
                </li>
              </ol>
            </>,
            "Export: MP4",
          ]}
        />
        <ModuleBlock
          emoji="🏗️"
          title="Website Builder — Code mit Live-Preview"
          bullets={[
            "Prompt → komplettes Projekt (HTML/CSS/JS oder React je nach Template)",
            "URL-Analyzer: Einer bestehenden Seite Stil + Struktur abschauen (via Playwright)",
            "Monaco-Editor mit Multi-File-Tabs, Live-Preview in Desktop/Tablet/Mobile-Viewports",
            "Assist: Code markieren + Anweisung tippen → nur die Selection wird rewrittet",
            "Templates: Landing, Portfolio, Blog, Dashboard, E-Commerce, Custom",
            "Export: ZIP, in Projekt-Ordner",
          ]}
        />
        <ModuleBlock
          emoji="🎨"
          title="Typography — Logos + Brand Kits"
          bullets={[
            "Prompt + Style (Modern/Brutalist/Vintage/Handwritten/etc.) → Logo-Varianten",
            "PNG → SVG via VTracer (lokal, kostenlos)",
            "Brand Kit: Logo + Farbpalette + Typo-System als Paket exportieren",
          ]}
        />
        <ModuleBlock
          emoji="💬"
          title="Chat (Sidebar, oben über Settings, oder Cmd+L)"
          bullets={[
            "Konversation mit Claude über deine Subscription",
            <>
              <strong className="text-neutral-dark-100">Wofür gut:</strong>
              <ul className="mt-1 ml-4 list-disc space-y-1">
                <li>
                  Prompt-Coaching vor einem Generate („wie beschreibe ich einen Sonnenuntergang?")
                </li>
                <li>Modell-Empfehlungen („welches Video-Modell für Architektur?")</li>
                <li>Allgemeine Claude-Fragen ohne Modul-Kontext</li>
                <li>Text zusammenfassen, übersetzen, brainstormen</li>
              </ul>
            </>,
            <>
              <strong className="text-neutral-dark-100">Wofür NICHT:</strong>
              <ul className="mt-1 ml-4 list-disc space-y-1">
                <li>
                  Bilder/Videos direkt aus dem Chat generieren (Tool-Use ist für Phase 10 geplant)
                </li>
                <li>
                  Langzeit-Gedächtnis über Sessions (nur localStorage,{" "}
                  <code className="font-mono text-accent-500">New chat</code> löscht alles)
                </li>
              </ul>
            </>,
          ]}
        />
      </div>
    </section>
  );
}

function ModuleBlock({
  emoji,
  title,
  bullets,
}: {
  emoji: string;
  title: string;
  bullets: ReactNode[];
}) {
  return (
    <div className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-950/40 p-3">
      <SubHeading>
        <span className="mr-1" aria-hidden="true">
          {emoji}
        </span>
        {title}
      </SubHeading>
      <ul className="mt-2 ml-4 list-disc space-y-1 text-neutral-dark-200">
        {bullets.map((b, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: static doc content
          <li key={i}>{b}</li>
        ))}
      </ul>
    </div>
  );
}

function ModelTiersSection() {
  return (
    <section className="flex flex-col gap-3">
      <SectionHeading>Drei Ebenen der Modell-Auswahl</SectionHeading>

      <div className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-950/40 p-3">
        <SubHeading>1. Default (der Router entscheidet)</SubHeading>
        <p className="mt-2 text-neutral-dark-200">
          Jede <code className="font-mono text-accent-500">TaskKind</code> hat eine Default-Chain
          aus Primary + Fallbacks. Beispiel{" "}
          <code className="font-mono text-accent-500">ImageGeneration</code> mit{" "}
          <code className="font-mono text-accent-500">Complexity=Medium</code>: Primary ={" "}
          <strong className="text-neutral-dark-100">Flux Pro</strong> (fal.ai) → Fallback 1 ={" "}
          <strong className="text-neutral-dark-100">Flux Dev</strong> (Replicate) → Fallback 2 ={" "}
          <strong className="text-neutral-dark-100">SDXL</strong> (fal.ai). Bei Auth- oder
          Transient-Errors wird automatisch weitergereicht.
        </p>
      </div>

      <div className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-950/40 p-3">
        <SubHeading>2. ToolDropdown (manuelles Override pro Prompt)</SubHeading>
        <p className="mt-2 text-neutral-dark-200">
          Neben jedem Generate-Button ist ein Dropdown{" "}
          <code className="font-mono text-accent-500">Auto (Router entscheidet)</code>. Klicken →
          Liste aller kompatiblen Modelle, gruppiert nach Tier (Primary / Fallbacks / Alternatives).
          Wahl überschreibt den Default für NUR diesen Generate.
        </p>
      </div>

      <div className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-950/40 p-3">
        <SubHeading>3. /tool Override (im Prompt-Text)</SubHeading>
        <p className="mt-2 text-neutral-dark-200">
          Start ODER Ende des Prompts darf ein Slug tragen:
        </p>
        <ul className="mt-2 ml-4 list-disc space-y-1 text-neutral-dark-200">
          <li>
            <Pill>/flux ein sonnenuntergang</Pill> — zwingt Flux Pro
          </li>
          <li>
            <Pill>ein sonnenuntergang /flux</Pill> — gleich
          </li>
          <li>
            <Pill>ein /flux sonnenuntergang</Pill> — wird ignoriert (mid-string)
          </li>
        </ul>
        <p className="mt-2 text-neutral-dark-200">
          Override gewinnt IMMER gegen Dropdown. Unbekannte Slugs (z.B. <Pill>/xyz</Pill>) werden
          nicht konsumiert — bleiben als Text im Prompt stehen. Vollständige Slug-Liste siehe
          „Override-Cheatsheet" unten oder <code className="font-mono text-accent-500">Cmd+/</code>.
        </p>
      </div>
    </section>
  );
}

function ModelEcosystemSection() {
  return (
    <section className="flex flex-col gap-3">
      <SectionHeading>Das Modell-Ökosystem — Was wofür</SectionHeading>

      <SubHeading>Bilder</SubHeading>
      <div className="flex flex-col gap-2">
        <ModelCard
          name="Flux Pro (default Medium/Complex)"
          slugs={["/flux", "/flux-pro"]}
          strengths="Fotorealismus, komplexe Kompositionen, natürliche Lichtstimmung"
          weaknesses="Teurer, langsamer"
          cost="~$0.05/Bild"
        />
        <ModelCard
          name="SDXL (default Simple)"
          slugs={["/sdxl"]}
          strengths="Schnell (~3s), günstig, gut für Stilisiertes"
          weaknesses="Schwächer bei Fotorealismus + Text-in-Bild"
          cost="~$0.01/Bild"
        />
        <ModelCard
          name="Flux Dev (via Replicate)"
          slugs={["/flux-dev"]}
          strengths="Open-Source-Variante, experimentell, mehr Creative Control"
          weaknesses="Weniger konsistent als Flux Pro"
          cost="~$0.03/Bild"
        />
        <ModelCard
          name="Real-ESRGAN"
          slugs={["/upscale"]}
          strengths="Upscaling 2x/4x, sehr scharf, Foto-optimiert"
          weaknesses="Nur Upscaling"
          cost="~$0.01/Bild"
        />
        <ModelCard
          name="Flux Fill"
          slugs={["/flux-fill"]}
          strengths="Präzises Inpainting, versteht Masken gut"
          weaknesses="Braucht public URL, keine data-URLs"
          cost="~$0.04/Bild"
        />
      </div>

      <SubHeading>Video</SubHeading>
      <div className="flex flex-col gap-2">
        <ModelCard
          name="Kling V2 Master (default, via fal.ai)"
          slugs={["/kling", "/kling-v2"]}
          strengths="Höchste Qualität, Kinematisch, Menschen/Tiere realistisch"
          weaknesses="Nur 5s oder 10s, 60-120s Rendering"
          cost="~$0.30/5s"
        />
        <ModelCard
          name="Kling V1.5 (fallback, via fal.ai)"
          slugs={["/kling-v15"]}
          strengths="Schneller, günstiger"
          weaknesses="Weniger detailliert"
          cost="~$0.15/5s"
        />
        <ModelCard
          name="Runway Gen-3"
          slugs={["/runway"]}
          strengths="Alternative, kreativer bei Motion, Abo-Modell"
          weaknesses="Abo statt Pay-per-Use"
          cost="Abo"
        />
        <ModelCard
          name="Higgsfield Multi"
          slugs={["/higgsfield"]}
          strengths="Motion-Control, Kamera-Pfade"
          weaknesses="Abo, Nische"
          cost="Abo"
        />
        <ModelCard
          name="Shotstack (Montage, Cloud)"
          slugs={["/shotstack"]}
          strengths="Audio-Mixing, Overlays, Text, Transitions"
          weaknesses="Nur Montage, kein Render"
          cost="~$0.05/s"
        />
        <ModelCard
          name="Remotion (Montage, lokal)"
          slugs={["/remotion"]}
          strengths="Kostenlos, programmierbar, reproduzierbar"
          weaknesses="Nur Template-basiert, kein AI"
          cost="$0"
        />
      </div>

      <SubHeading>3D</SubHeading>
      <div className="flex flex-col gap-2">
        <ModelCard
          name="Meshy Text-3D (default Text)"
          slugs={["/meshy", "/meshy-text"]}
          strengths="Hochwertige Textur, Abo-Modell, gut für Charaktere"
          weaknesses="Abo nötig"
          cost="Abo"
        />
        <ModelCard
          name="Meshy Image-3D (default Image Complex)"
          slugs={["/meshy-image"]}
          strengths="Image-basiert, scharfe Geometrie"
          weaknesses="Abo, langsamer"
          cost="Abo"
        />
        <ModelCard
          name="TripoSR (default Image Simple, via Replicate)"
          slugs={["/triposr"]}
          strengths="Pay-per-Use, schnell (~10s), gut für Previews"
          weaknesses="Niedrigere Qualität als Meshy"
          cost="~$0.05/Mesh"
        />
        <ModelCard
          name="Depth-Anything v2 (via Replicate)"
          slugs={["/depth"]}
          strengths="Tiefenkarten, robust"
          weaknesses="Nur Depth"
          cost="~$0.01"
        />
      </div>

      <SubHeading>
        Text + Code (läuft via Claude-CLI über deine Subscription, wenn auf{" "}
        <code className="font-mono text-accent-500">cli</code> gesetzt)
      </SubHeading>
      <div className="flex flex-col gap-2">
        <ModelCard
          name="Claude Opus"
          slugs={["/claude-opus"]}
          strengths="Beste Reasoning, komplexe Code-Projekte"
          weaknesses="Langsamer, teurer bei API-Transport"
          cost="Complex"
        />
        <ModelCard
          name="Claude Sonnet (default TextGeneration)"
          slugs={["/claude", "/claude-sonnet"]}
          strengths="Balance aus Qualität + Kosten, auch Vision-Analyse"
          weaknesses="Medium"
          cost="Medium"
        />
        <ModelCard
          name="Claude Haiku"
          slugs={["/claude-haiku"]}
          strengths="Schnell, günstig für einfache Tasks"
          weaknesses="Weniger tiefes Reasoning"
          cost="Simple"
        />
      </div>

      <SubHeading>Logos + Typografie</SubHeading>
      <div className="flex flex-col gap-2">
        <ModelCard
          name="Ideogram V3 (default Logo)"
          slugs={["/ideogram"]}
          strengths="Text-in-Bild Champion (Logos lesbar!)"
          weaknesses="Eigener Key nötig"
          cost="~$0.04/Logo"
        />
      </div>
    </section>
  );
}

function OptimizeSection() {
  return (
    <section className="flex flex-col gap-2">
      <SectionHeading>Optimize-Toggle</SectionHeading>
      <p>
        Neben jedem Prompt-Feld sitzt ein kleiner Switch{" "}
        <code className="font-mono text-accent-500">Optimize</code>. Wenn aktiviert + Generate
        geklickt:
      </p>
      <ol className="ml-4 list-decimal space-y-1">
        <li>
          Dein Roh-Prompt geht erst zu Claude mit einem Meta-Template (visuell / code / logo /
          generic, je nach TaskKind)
        </li>
        <li>Claude schreibt ihn spezifischer, visuell reicher, unmissverständlicher um</li>
        <li>
          Der neue Prompt <strong className="text-neutral-dark-50">ersetzt</strong> den Text im
          Eingabefeld
        </li>
        <li>
          Ein <code className="font-mono text-accent-500">Undo</code>-Button erscheint — klicken
          stellt den Original-Prompt wieder her
        </li>
        <li>Erst DANN geht der optimierte Prompt an das generierende Modell (Flux, Kling etc.)</li>
      </ol>
      <p>
        <strong className="text-neutral-dark-50">Wichtig:</strong> ein{" "}
        <code className="font-mono text-accent-500">/tool</code>-Override wird vor dem Optimize-Call
        aus dem Prompt genommen und danach wieder drangehängt — so sieht Claude nie den Slug und
        rewrittet ihn nicht weg.
      </p>
    </section>
  );
}

function ClaudeTransportSection() {
  return (
    <section className="flex flex-col gap-2">
      <SectionHeading>Transport-Wahl für Claude</SectionHeading>
      <p>
        In <code className="font-mono text-accent-500">Settings → Claude-Zeile</code> → Dropdown
        links neben dem Key-Feld:
      </p>
      <ul className="ml-4 list-disc space-y-1">
        <li>
          <strong className="text-neutral-dark-50">auto</strong> (default) — nimmt CLI-Subscription
          wenn erkannt, sonst API-Key als Fallback
        </li>
        <li>
          <strong className="text-neutral-dark-50">api</strong> — immer über API-Key (kostet
          Credits)
        </li>
        <li>
          <strong className="text-neutral-dark-50">cli</strong> — erzwingt CLI-Subscription
          (kostenlos im Rahmen deines Plans); fehlt die CLI → klarer Fehler
        </li>
      </ul>
      <p>
        Installation:{" "}
        <code className="font-mono text-accent-500">brew install anthropic/claude-code/claude</code>{" "}
        + <code className="font-mono text-accent-500">claude login</code>.
      </p>
    </section>
  );
}

function OverrideCheatsheetSection() {
  const groups: Array<{ label: string; slugs: string[] }> = [
    {
      label: "Text-Modelle",
      slugs: ["/claude", "/claude-opus", "/claude-sonnet", "/claude-haiku"],
    },
    {
      label: "Bilder",
      slugs: ["/sdxl", "/flux", "/flux-pro", "/flux-dev", "/flux-fill", "/upscale", "/ideogram"],
    },
    {
      label: "Video",
      slugs: [
        "/kling",
        "/kling-v15",
        "/kling-v2",
        "/runway",
        "/higgsfield",
        "/shotstack",
        "/remotion",
      ],
    },
    {
      label: "3D",
      slugs: ["/meshy", "/meshy-text", "/meshy-image", "/triposr", "/depth"],
    },
  ];
  return (
    <section className="flex flex-col gap-2">
      <SectionHeading>Override-Cheatsheet (vollständig)</SectionHeading>
      <div className="flex flex-col gap-2">
        {groups.map((g) => (
          <div
            key={g.label}
            className="flex flex-col gap-2 rounded-xs border border-neutral-dark-700 bg-neutral-dark-950/40 p-3 sm:flex-row sm:items-center"
          >
            <span className="min-w-[7rem] font-mono text-2xs uppercase tracking-label text-neutral-dark-400">
              {g.label}
            </span>
            <div className="flex flex-wrap gap-1.5">
              {g.slugs.map((s) => (
                <Pill key={s}>{s}</Pill>
              ))}
            </div>
          </div>
        ))}
      </div>
      <p className="text-neutral-dark-300">
        (Gleiche Liste siehe <code className="font-mono text-accent-500">Cmd+/</code> —
        Shortcut-Overlay.)
      </p>
    </section>
  );
}

function ShortcutsSection() {
  const entries: Array<{ combo: string; label: string }> = [
    { combo: "Cmd+L", label: "Chat öffnen" },
    { combo: "Cmd+,", label: "Settings" },
    { combo: "Cmd+/ oder ?", label: "Shortcut-Overlay (inkl. Override-Liste)" },
    { combo: "Cmd+Enter", label: "Im Chat: Senden" },
    {
      combo: "Cmd+S",
      label: "Projekt speichern (in Modulen die das unterstützen)",
    },
  ];
  return (
    <section className="flex flex-col gap-2">
      <SectionHeading>Tastaturkürzel</SectionHeading>
      <ul className="flex flex-col gap-1">
        {entries.map((e) => (
          <li
            key={e.combo}
            className="flex items-center justify-between border-b border-neutral-dark-800 py-1 last:border-b-0"
          >
            <span>{e.label}</span>
            <kbd className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-1.5 py-0.5 font-mono text-2xs text-neutral-dark-300">
              {e.combo}
            </kbd>
          </li>
        ))}
      </ul>
    </section>
  );
}

function KeysAndCostsSection() {
  return (
    <section className="flex flex-col gap-2">
      <SectionHeading>API-Keys + Kosten</SectionHeading>
      <p>
        Bereit für Live-Test reicht <strong className="text-neutral-dark-50">ein fal.ai-Key</strong>
        , denn fal deckt ab:
      </p>
      <ul className="ml-4 list-disc space-y-1">
        <li>Alle Bilder (Flux Pro, SDXL, Real-ESRGAN, Flux Fill)</li>
        <li>Alle Videos (Kling V1.5 + V2 Master)</li>
      </ul>
      <p>
        Plus die <strong className="text-neutral-dark-50">Claude-CLI-Installation</strong> für alle
        Text-Pfade (kostenlos über Subscription).
      </p>
      <p>Alle weiteren Keys sind optional je nach Modell:</p>
      <ul className="ml-4 list-disc space-y-1">
        <li>
          <strong className="text-neutral-dark-50">Ideogram</strong> — nur wenn du Logos mit Text
          machen willst
        </li>
        <li>
          <strong className="text-neutral-dark-50">Replicate</strong> — für TripoSR, Flux Dev,
          Depth-Anything v2
        </li>
        <li>
          <strong className="text-neutral-dark-50">Runway Pro</strong> — nur falls du Runway als
          Video-Fallback oder Override willst
        </li>
        <li>
          <strong className="text-neutral-dark-50">Higgsfield</strong> — Nische
        </li>
        <li>
          <strong className="text-neutral-dark-50">Shotstack</strong> — falls du Cloud-Video-Montage
          willst (alternative: Remotion lokal)
        </li>
        <li>
          <strong className="text-neutral-dark-50">Meshy</strong> — wenn du hochwertige 3D-Meshes
          willst (alternative: TripoSR Pay-per-Use)
        </li>
      </ul>
      <p>
        Alle Keys liegen im <strong className="text-neutral-dark-50">macOS-Keychain</strong>, nicht
        in Config-Files. Werden dort gespeichert wenn du sie in Settings einträgst. macOS fragt beim
        ersten Read: bitte <code className="font-mono text-accent-500">Immer erlauben</code> wählen,
        sonst fragt es bei jedem Generate wieder.
      </p>
    </section>
  );
}

function ProjectsSection() {
  return (
    <section className="flex flex-col gap-2">
      <SectionHeading>Projekte + Speicherung</SectionHeading>
      <ul className="ml-4 list-disc space-y-1">
        <li>
          Projekte liegen unter{" "}
          <code className="font-mono text-accent-500">~/Documents/TERRYBLEMACHINE Projects/</code>{" "}
          (default, in Settings änderbar)
        </li>
        <li>
          Jedes Projekt hat seinen eigenen Ordner mit Manifest (JSON), Canvas-State, und Asset-Cache
        </li>
        <li>
          <code className="font-mono text-accent-500">Recents</code>-Menü oben im Header zeigt die
          letzten Projekte
        </li>
        <li>
          <code className="font-mono text-accent-500">meingeschmack/</code>-Ordner enthält deine
          Taste-Engine-Config (Referenz-Bilder, Farb-Paletten, Fonts) — wird bei jedem Generate
          konsultiert
        </li>
      </ul>
    </section>
  );
}

function LimitationsSection() {
  return (
    <section className="flex flex-col gap-2">
      <SectionHeading>Bekannte Einschränkungen (Transparenz)</SectionHeading>
      <ul className="ml-4 list-disc space-y-1">
        <li>
          <strong className="text-neutral-dark-50">Chat ist rein konversational</strong> — kein
          autonomes Tool-Use, keine Modul-Ausführung aus dem Chat heraus (Phase 10)
        </li>
        <li>
          <strong className="text-neutral-dark-50">Flux Fill</strong> braucht public URLs für
          Source/Mask — data-URLs vom Canvas funktionieren noch nicht (Phase 5 Upload-Shim)
        </li>
        <li>
          <strong className="text-neutral-dark-50">Kling V2 Master:</strong> nur 5s oder 10s
          Segmente. App schnappt auto auf den näheren Wert (≤7s → 5, ≥8s → 10)
        </li>
        <li>
          <strong className="text-neutral-dark-50">Video-Rendering:</strong> 60-120s pro Segment via
          Queue+Poll — sei geduldig
        </li>
        <li>
          <strong className="text-neutral-dark-50">Chat-Streaming:</strong> momentan Single-Shot
          (buffert komplette Response dann eine Chunk-Emit) — echtes Token-by-Token folgt
        </li>
        <li>
          <strong className="text-neutral-dark-50">Windows/Linux:</strong> nicht supportet,
          macOS-only
        </li>
      </ul>
    </section>
  );
}
