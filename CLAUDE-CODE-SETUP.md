# TERRYBLEMACHINE — Claude Code Setup & Entwicklungsumgebung

## Übersicht

Dieses Dokument beschreibt die vollständige Einrichtung von Claude Code für die Entwicklung von TERRYBLEMACHINE. Es umfasst drei Skill-Pakete, MCP-Server, Projekt-Tooling und den Entwicklungs-Workflow.

### Die drei Skill-Pakete

| Paket | Quelle | Zweck | Umfang |
|-------|--------|-------|--------|
| **Superpowers** | github.com/obra/superpowers | Strukturierter 7-Phasen-Workflow, TDD, Code Review, parallele Agents | 14 Skills (Plugin) |
| **Anthropic Frontend-Design** | github.com/anthropics/skills | Hochwertige UI-Komponenten ohne "AI-Slop-Ästhetik" | 1 Skill + weitere nützliche |
| **Claude Skills Collection** | github.com/jeffallan/claude-skills | 66 spezialisierte Skills für Sprachen, Frameworks, DevOps, Security | 66 Skills (Plugin) |

---

## Voraussetzungen

### System

```
macOS 14+ (Sonoma oder neuer)
M3/M4 MacBook Pro
Homebrew installiert
```

### Prüfung

```bash
# Diese Befehle sollten alle eine Version ausgeben:
sw_vers                    # macOS Version
brew --version             # Homebrew
git --version              # Git
```

### Accounts & API-Keys (11 Services)

Abos:
- [x] Claude Max (anthropic.com) — $100/Mo
- [ ] Kling AI (klingai.com) — ~$98/Mo (Unit-Paket)
- [ ] Runway Pro (runwayml.com) — $28/Mo
- [ ] Higgsfield Plus (higgsfield.ai) — $39/Mo
- [ ] Shotstack Subscription (shotstack.io) — $39/Mo
- [ ] Ideogram Plus (ideogram.ai) — $15/Mo
- [ ] Meshy Pro (meshy.ai) — ~$10/Mo

Pay-per-Use:
- [x] fal.ai (fal.ai) — Pay-per-Image (~$16/Mo geschätzt)
- [ ] Replicate (replicate.com) — Pay-per-Compute (~$9/Mo geschätzt)

Kostenlos:
- [x] Remotion (remotion.dev) — Open Source, keine Anmeldung nötig
- [x] OpenMontage (github.com/calesthio/OpenMontage) — Open Source

Sonstiges:
- [ ] GitHub Personal Access Token (für MCP-Server)

---

## Schritt-für-Schritt-Installation

---

### SCHRITT 1: Grundlegende Dev-Tools installieren

```bash
# ── 1.1 Rust (für Tauri Backend) ──
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup target add aarch64-apple-darwin
```

**Prüfung:**
```bash
rustc --version
# Erwartete Ausgabe: rustc 1.x.x (...)
cargo --version
# Erwartete Ausgabe: cargo 1.x.x (...)
```

```bash
# ── 1.2 Node.js (für Frontend) ──
# Falls nvm noch nicht installiert:
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash
source ~/.zshrc

nvm install 20
nvm use 20
```

**Prüfung:**
```bash
node --version
# Erwartete Ausgabe: v20.x.x
npm --version
# Erwartete Ausgabe: 10.x.x
```

```bash
# ── 1.3 pnpm (schneller als npm) ──
npm install -g pnpm
```

**Prüfung:**
```bash
pnpm --version
# Erwartete Ausgabe: 9.x.x oder 10.x.x
```

```bash
# ── 1.4 Tauri CLI ──
cargo install tauri-cli
```

**Prüfung:**
```bash
cargo tauri --version
# Erwartete Ausgabe: tauri-cli 2.x.x
```

```bash
# ── 1.5 GitHub CLI ──
brew install gh
gh auth login
# Folge dem Login-Flow (Browser öffnet sich)
```

**Prüfung:**
```bash
gh auth status
# Erwartete Ausgabe: ✓ Logged in to github.com as <dein-username>
```

```bash
# ── 1.6 Playwright (für Website-Analyse) ──
npx playwright install chromium
```

**Prüfung:**
```bash
npx playwright --version
# Erwartete Ausgabe: Version 1.x.x
```

---

### SCHRITT 2: Claude Code installieren und starten

```bash
# ── 2.1 Claude Code installieren (falls noch nicht vorhanden) ──
npm install -g @anthropic-ai/claude-code
```

**Prüfung:**
```bash
claude --version
# Erwartete Ausgabe: claude x.x.x
```

```bash
# ── 2.2 Claude Code erstmalig starten und authentifizieren ──
claude
# Beim ersten Start: Anthropic-Account verbinden (Browser öffnet sich)
# Danach mit /exit beenden
```

**Prüfung:**
```bash
# Claude Code nochmal starten — sollte ohne Login-Prompt öffnen:
claude --version
```

---

### SCHRITT 3: Superpowers Plugin installieren

Superpowers ist das Herzstück des Entwicklungsworkflows. Es erzwingt einen strukturierten 7-Phasen-Prozess und verhindert, dass Claude Code unstrukturiert drauflos implementiert.

```bash
# ── 3.1 Superpowers installieren ──
claude
```

Innerhalb von Claude Code:
```
/plugin install superpowers@superpowers-marketplace
```

Falls das nicht funktioniert, Alternative:
```
/plugin marketplace add obra/superpowers
```

**Prüfung — innerhalb von Claude Code:**
```
/help
```
Du solltest neue Befehle sehen. Zusätzlich prüfen:
```
Welche Superpowers-Skills hast du geladen? Liste sie auf.
```

Claude sollte die folgenden 14 Skills aufzählen:
1. `brainstorming` — Anforderungen klären durch Dialog
2. `using-git-worktrees` — Isolierte Workspaces erstellen
3. `writing-plans` — Arbeit in 2-5 Minuten Tasks aufteilen
4. `executing-plans` — Tasks mit Sub-Agents abarbeiten
5. `test-driven-development` — RED → GREEN → REFACTOR Zyklen
6. `subagent-driven-development` — Mehrere Agents koordinieren
7. `requesting-code-review` — Code-Review initiieren
8. `receiving-code-review` — Feedback einarbeiten
9. `systematic-debugging` — Methodische Fehleranalyse
10. `verification-before-completion` — Qualitätsprüfung vor Merge
11. `finishing-a-development-branch` — Branch abschließen
12. `dispatching-parallel-agents` — Parallele Agent-Ausführung
13. `using-superpowers` — Framework-Übersicht
14. `writing-skills` — Eigene Skills erstellen

**Wenn die Installation fehlschlägt:**
```bash
# Manuell klonen und lokal installieren:
cd ~/.claude
git clone https://github.com/obra/superpowers.git
# Dann in Claude Code:
/plugin install --local ~/.claude/superpowers
```

```
/exit
```

---

### SCHRITT 4: Anthropic Skills installieren (Frontend-Design + weitere)

Das offizielle Anthropic Skills Repository enthält 17 Skills. Für TERRYBLEMACHINE sind besonders relevant:
- `frontend-design` — Hochwertige, distinctive UI-Komponenten
- `canvas-design` — Canvas-basierte Designs (für Fabric.js/Three.js)
- `web-artifacts-builder` — Web-Artefakte bauen
- `theme-factory` — Theme-Generierung

```bash
# ── 4.1 Anthropic Skills Plugin installieren ──
claude
```

Innerhalb von Claude Code:
```
/plugin install example-skills@anthropic-agent-skills
```

Falls das nicht funktioniert, Alternative:
```
/plugin marketplace add anthropics/skills
```

**Prüfung — innerhalb von Claude Code:**
```
Welche Anthropic Skills hast du geladen? Kennst du den frontend-design Skill?
```

Claude sollte bestätigen, dass `frontend-design` verfügbar ist und beschreiben können:
- Vermeidet generische "AI-Slop-Ästhetik"
- Erzwingt distinctive Typografie (keine generischen Fonts wie Arial/Inter)
- Kohärente Farbpaletten mit CSS-Variablen
- Strategische Micro-Interactions und Animationen
- Asymmetrische, Grid-brechende Kompositionen

**Zusätzlich prüfen ob diese Skills erkannt werden:**
```
Kennst du die Skills canvas-design, web-artifacts-builder und theme-factory?
```

```
/exit
```

---

### SCHRITT 5: Claude Skills Collection installieren (66 Skills)

Jeff Allans Sammlung gibt Claude Code Expertise in spezifischen Sprachen und Frameworks. Für TERRYBLEMACHINE relevant:
- `react-expert` — React-Komponenten auf Experten-Niveau
- `typescript-pro` — TypeScript Best Practices
- `rust-engineer` — Idiomatischer Rust-Code
- `test-master` — Umfassende Test-Strategien
- `playwright-expert` — E2E-Tests
- `api-designer` — API-Design-Patterns
- `code-reviewer` — Automatisierte Code-Reviews
- `architecture-designer` — Architektur-Entscheidungen

```bash
# ── 5.1 Claude Skills Collection installieren ──
claude
```

Innerhalb von Claude Code:
```
/plugin install fullstack-dev-skills@jeffallan
```

Falls das nicht funktioniert, Alternative:
```
/plugin marketplace add jeffallan/claude-skills
```

Dritte Alternative (CLI):
```bash
# Außerhalb von Claude Code:
npx skills add jeffallan/claude-skills
```

**Prüfung — innerhalb von Claude Code:**
```
Welche Skills aus der jeffallan/claude-skills Collection hast du geladen?
Kennst du den react-expert und rust-engineer Skill?
```

Claude sollte bestätigen und mindestens diese Kategorien aufzählen:
- 12 Sprach-Skills (Python, TypeScript, Rust, Go, etc.)
- 7 Backend-Framework-Skills (NestJS, Django, FastAPI, etc.)
- 7 Frontend-Skills (React, Next.js, Vue, Angular, etc.)
- 4 Quality/Testing-Skills
- 5 DevOps-Skills
- 2 Security-Skills

```
/exit
```

---

### SCHRITT 5.5: Remotion Skill installieren

Remotion ist unser lokales Video-Rendering-Framework (React → MP4). Der offizielle Claude Code Skill lehrt Claude wie Remotion funktioniert und generiert produktionsreifen Remotion-Code.

```bash
# ── 5.5.1 Remotion Skill installieren (außerhalb von Claude Code) ──
npx skills add remotion-dev/skills
```

**Prüfung:**
```bash
claude
```

Innerhalb von Claude Code:
```
Kennst du Remotion? Wie würdest du eine 30-Sekunden Kinetic Typography
Animation mit Remotion erstellen? Beschreibe nur den Ansatz.
```

**Erwartetes Verhalten:** Claude sollte Remotion-spezifische Konzepte nennen:
- `useCurrentFrame()` Hook für zeitbasierte Animationen
- `interpolate()` und `spring()` für Transitions
- `<Sequence>` und `<AbsoluteFill>` Komponenten
- `@remotion/three` für 3D-Integration
- `npx remotion render` für lokales Rendering

```
/exit
```

---

### SCHRITT 6: MCP-Server einrichten

MCP-Server erweitern Claude Code um externe Fähigkeiten.

```bash
# ── 6.1 MCP-Konfigurationsdatei erstellen/bearbeiten ──
```

Öffne die Datei `~/.claude/mcp_config.json` (erstellen falls nicht vorhanden) und setze folgenden Inhalt:

```json
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_TOKEN": "<DEIN-GITHUB-PERSONAL-ACCESS-TOKEN>"
      }
    },
    "filesystem": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-filesystem",
        "/Users/<DEIN-USERNAME>/TERRYBLEMACHINE"
      ]
    },
    "playwright": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-playwright"]
    }
  }
}
```

**Wichtig:** Ersetze `<DEIN-GITHUB-PERSONAL-ACCESS-TOKEN>` und `<DEIN-USERNAME>` mit deinen echten Werten.

GitHub Token erstellen: https://github.com/settings/tokens → "Generate new token (classic)" → Scopes: `repo`, `read:org`, `workflow`

**Prüfung:**
```bash
claude
```

Innerhalb von Claude Code:
```
Welche MCP-Server sind verbunden? Liste alle auf.
```

Claude sollte antworten:
- `github` — GitHub API Zugriff
- `filesystem` — Dateisystem-Zugriff auf TERRYBLEMACHINE
- `playwright` — Browser-Automatisierung

```
/exit
```

---

### SCHRITT 7: TERRYBLEMACHINE Projekt initialisieren

```bash
# ── 7.1 In den Projektordner wechseln ──
cd ~/TERRYBLEMACHINE   # oder wo auch immer dein Ordner liegt

# ── 7.2 Git initialisieren ──
git init
git add .
git commit -m "docs: Initial project documentation and planning"
```

**Prüfung:**
```bash
git log --oneline
# Erwartete Ausgabe: ein Commit mit der Dokumentation
```

```bash
# ── 7.3 GitHub Repository erstellen ──
gh repo create TERRYBLEMACHINE --private --source=. --push
```

**Prüfung:**
```bash
gh repo view
# Erwartete Ausgabe: Repository-Info mit deinem Username
git remote -v
# Erwartete Ausgabe: origin zeigt auf github.com/<user>/TERRYBLEMACHINE
```

```bash
# ── 7.4 Develop-Branch erstellen ──
git checkout -b develop
git push -u origin develop
```

**Prüfung:**
```bash
git branch -a
# Erwartete Ausgabe: * develop, main, remotes/origin/develop, remotes/origin/main
```

---

### SCHRITT 8: Gesamtprüfung — Alles zusammen testen

```bash
# ── 8.1 Claude Code im Projektordner starten ──
cd ~/TERRYBLEMACHINE
claude
```

Innerhalb von Claude Code — Folgende Prüfungen durchführen:

**8.2 Prüfe Superpowers:**
```
Starte einen Brainstorming-Prozess: Wie sollte die Sidebar-Navigation
für TERRYBLEMACHINE aussehen? (Nur Brainstorm, nicht implementieren!)
```

**Erwartetes Verhalten:** Claude startet den Superpowers `brainstorming`-Skill, stellt Rückfragen zu Anforderungen, und erstellt ein strukturiertes Design-Dokument. Claude sollte NICHT sofort Code schreiben.

**8.3 Prüfe Frontend-Design:**
```
Wie würdest du den frontend-design Skill nutzen um eine distinctive
Ästhetik für TERRYBLEMACHINE zu entwerfen? Beschreibe nur den Ansatz.
```

**Erwartetes Verhalten:** Claude referenziert den frontend-design Skill und spricht über distinctive Typografie, kohärente Farbpaletten, Grid-brechende Layouts, und vermeidet generische Patterns.

**8.4 Prüfe Claude Skills Collection:**
```
Ich möchte den AI Router in Rust implementieren. Welche Skills aus
deiner Sammlung würdest du dafür aktivieren?
```

**Erwartetes Verhalten:** Claude sollte `rust-engineer`, `api-designer`, `architecture-designer` und `test-master` nennen.

**8.5 Prüfe MCP-Server:**
```
Lies die Datei docs/PROJEKTUEBERSICHT.md und fasse sie in 3 Sätzen zusammen.
```

**Erwartetes Verhalten:** Claude liest die Datei über den Filesystem-MCP und gibt eine Zusammenfassung.

**8.6 Prüfe GitHub MCP:**
```
Zeige mir die offenen Issues in meinem TERRYBLEMACHINE Repository.
```

**Erwartetes Verhalten:** Claude greift via GitHub-MCP auf das Repo zu (vermutlich 0 Issues, das ist ok).

```
/exit
```

---

## Installierte Skills — Gesamtübersicht

### Superpowers (14 Skills — Workflow & Methodik)

| Skill | Zweck | Wann aktiv |
|-------|-------|------------|
| brainstorming | Anforderungen klären, Design erarbeiten | VOR jeder Implementation |
| using-git-worktrees | Isolierte Feature-Branches | Bei Feature-Start |
| writing-plans | Tasks in 2-5 Min Schritte aufteilen | Nach Brainstorming |
| executing-plans | Tasks strukturiert abarbeiten | Während Implementation |
| test-driven-development | RED → GREEN → REFACTOR | Bei Code-Änderungen |
| subagent-driven-development | Sub-Agents für parallele Arbeit | Bei komplexen Features |
| requesting-code-review | Review-Prozess starten | Nach Implementation |
| receiving-code-review | Feedback einarbeiten | Nach Review |
| systematic-debugging | Methodische Bug-Analyse | Bei Fehlern |
| verification-before-completion | Qualitätssicherung | Vor Merge |
| finishing-a-development-branch | Branch abschließen | Am Ende eines Features |
| dispatching-parallel-agents | Parallele Agent-Ausführung | Bei unabhängigen Tasks |
| using-superpowers | Framework-Übersicht | Immer (Meta-Skill) |
| writing-skills | Eigene Skills erstellen | Bei Bedarf |

### Anthropic Skills (4 relevante — Design & Frontend)

| Skill | Zweck | Wann aktiv |
|-------|-------|------------|
| frontend-design | Distinctive UI ohne AI-Slop | Bei UI-Komponenten |
| canvas-design | Canvas-basierte Designs | Bei Fabric.js/Three.js Arbeit |
| web-artifacts-builder | Web-Artefakte bauen | Bei Website-Modul |
| theme-factory | Theme-Generierung | Bei Design-System |

### Claude Skills Collection (8 primär relevante von 66)

| Skill | Zweck | Wann aktiv |
|-------|-------|------------|
| react-expert | React Best Practices | Bei Frontend-Code |
| typescript-pro | TypeScript Patterns | Immer (Frontend) |
| rust-engineer | Idiomatischer Rust | Immer (Backend) |
| test-master | Test-Strategien | Bei Tests |
| playwright-expert | E2E-Tests | Bei Integration-Tests |
| api-designer | API-Design | Bei AI Router, API Clients |
| code-reviewer | Code-Review | Bei Reviews |
| architecture-designer | Architektur-Entscheidungen | Bei Design-Phase |

---

## Der Superpowers-Workflow für TERRYBLEMACHINE

So sieht ein typischer Feature-Zyklus aus:

```
┌─────────────────────────────────────────────────────────────────┐
│                     SUPERPOWERS WORKFLOW                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Phase 1: BRAINSTORMING                                          │
│  ────────────────────                                            │
│  Claude fragt: Was genau soll gebaut werden?                     │
│  Dialog bis Design-Dokument steht.                               │
│  → Kein Code bis Phase 1 abgeschlossen!                         │
│                                                                  │
│  Phase 2: GIT WORKTREE                                           │
│  ──────────────────                                              │
│  Claude erstellt isolierten Workspace:                           │
│  git worktree add ../TERRYBLEMACHINE-feature feature/xxx         │
│                                                                  │
│  Phase 3: PLAN SCHREIBEN                                         │
│  ──────────────────────                                          │
│  Claude bricht Arbeit in Tasks á 2-5 Minuten:                   │
│  Task 1: Setup types/interfaces                                  │
│  Task 2: Write failing test for X                                │
│  Task 3: Implement X                                             │
│  Task 4: Write failing test for Y                                │
│  ...                                                             │
│                                                                  │
│  Phase 4: EXECUTE (mit TDD)                                      │
│  ──────────────────────────                                      │
│  Für jeden Task:                                                 │
│  1. Test schreiben (RED)                                         │
│  2. Minimal implementieren (GREEN)                               │
│  3. Refactoren (REFACTOR)                                        │
│  Skills aktiv: react-expert, rust-engineer, frontend-design      │
│                                                                  │
│  Phase 5: CODE REVIEW                                            │
│  ─────────────────────                                           │
│  Claude reviewed eigenen Code, du reviewst danach.               │
│  Skills aktiv: code-reviewer                                     │
│                                                                  │
│  Phase 6: VERIFICATION                                           │
│  ─────────────────────                                           │
│  Alle Tests grün? Keine Regressions?                             │
│  Skills aktiv: test-master, playwright-expert                    │
│                                                                  │
│  Phase 7: FINISH BRANCH                                          │
│  ─────────────────────                                           │
│  PR erstellen, Worktree aufräumen, Merge.                        │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Git-Workflow

### Branch-Strategie

```
main                    → Stabile Releases
├── develop             → Integration Branch
│   ├── feature/core-ui        → UI-Shell, Navigation, Theme
│   ├── feature/ai-router      → AI-Router, Caching, Budget
│   ├── feature/website-module → Website-Builder
│   ├── feature/2d-module      → 2D-Grafik & Bild-Erstellung
│   ├── feature/3d-module      → Pseudo-3D
│   ├── feature/video-module   → Video-Produktion
│   ├── feature/typo-module    → Typografie & Logos
│   └── feature/taste-engine   → meingeschmack/ System
└── hotfix/*            → Kritische Fixes
```

### Commit-Konventionen

```
feat(module): Kurzbeschreibung       # Neues Feature
fix(module): Kurzbeschreibung        # Bugfix
refactor(module): Kurzbeschreibung   # Refactoring
docs: Kurzbeschreibung               # Dokumentation
test(module): Kurzbeschreibung       # Tests
chore: Kurzbeschreibung              # Build, Dependencies
```

### Parallele Entwicklung mit Git Worktrees (Superpowers)

```bash
# Worktree für paralleles Feature erstellen
git worktree add ../TERRYBLEMACHINE-ui feature/core-ui
git worktree add ../TERRYBLEMACHINE-router feature/ai-router

# In separaten Terminals arbeiten
cd ../TERRYBLEMACHINE-ui && claude
cd ../TERRYBLEMACHINE-router && claude
```

---

## Tauri Projekt-Setup (nach Skills-Installation)

### Frontend-Dependencies

```bash
cd ~/TERRYBLEMACHINE

# Tauri + React + TypeScript Projekt erstellen
pnpm create tauri-app . --template react-ts

# Abhängigkeiten installieren
pnpm install

# Canvas-Libraries
pnpm add fabric three @types/three pixijs

# State Management
pnpm add zustand

# Utility
pnpm add @tanstack/react-query axios zod

# Dev-Tools
pnpm add -D @biomejs/biome vitest @testing-library/react
```

### Tauri Rust-Abhängigkeiten

```toml
# In src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2", features = ["shell-open", "fs", "dialog"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json", "multipart"] }
tokio = { version = "1", features = ["full"] }
security-framework = "2"  # macOS Keychain
image = "0.25"
base64 = "0.22"
sha2 = "0.10"  # Für Cache-Keys
thiserror = "1"
```

### Prüfung

```bash
pnpm tauri dev
# Erwartetes Ergebnis: Vite Dev Server startet, Tauri-Fenster öffnet sich
# Das Fenster zeigt die Default-React-App
```

---

## Lokale Entwicklung

```bash
# Development Server starten
pnpm tauri dev

# Das öffnet:
# - Vite Dev Server (Frontend) auf http://localhost:1420
# - Tauri Window mit Hot-Reload
# - Rust Backend kompiliert bei Änderungen

# Tests
pnpm test              # Frontend-Tests (Vitest)
cargo test             # Rust-Tests
pnpm playwright test   # E2E-Tests
```

---

## Troubleshooting

### Plugin lässt sich nicht installieren

```bash
# Claude Code aktualisieren:
npm update -g @anthropic-ai/claude-code

# Cache leeren:
rm -rf ~/.claude/plugins/cache

# Claude Code neu starten:
claude
```

### MCP-Server verbindet nicht

```bash
# Prüfe ob die Config-Datei korrekt ist:
cat ~/.claude/mcp_config.json | python3 -m json.tool

# Prüfe ob npx funktioniert:
npx -y @modelcontextprotocol/server-github --help

# Prüfe GitHub Token:
gh auth status
```

### Superpowers-Skills werden nicht erkannt

```bash
# Prüfe Plugin-Installation:
ls ~/.claude/plugins/

# Falls leer — manuell installieren:
cd ~/.claude
git clone https://github.com/obra/superpowers.git plugins/superpowers
```

### Tauri Dev startet nicht

```bash
# Prüfe Rust-Installation:
rustc --version
cargo --version

# Prüfe Tauri-Voraussetzungen:
cargo tauri info

# macOS-spezifisch — Xcode Command Line Tools:
xcode-select --install
```
