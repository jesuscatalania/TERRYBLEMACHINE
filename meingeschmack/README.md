# meingeschmack/ — Dein Stil-Verzeichnis

Dieser Ordner definiert deinen persönlichen visuellen Stil. TERRYBLEMACHINE liest alles hier und wendet es als Regelwerk auf jeden generierten Output an.

## Schnellstart

1. Lege 3-5 Referenzbilder in `referenzen/bilder/`
2. Erstelle `regeln/verboten.md` mit deinen wichtigsten No-Gos
3. Erstelle `regeln/farben.md` mit deinen bevorzugten Farbpaletten
4. Starte die App — der Rest passiert automatisch

## Struktur

```
meingeschmack/
├── regeln/          # Textbasierte Stilregeln (.md Dateien)
├── referenzen/      # Visuelle Referenzen
│   ├── bilder/      # Referenzbilder
│   ├── videos/      # Video-Referenzen
│   ├── websites/    # Website-Screenshots
│   └── logos/       # Logo-Referenzen
└── paletten/        # Farbpaletten als .json
```

## Wie es funktioniert

Die App analysiert automatisch alles in diesem Ordner und baut daraus ein Style Profile. Dieses Profil wird bei jeder Generierung als zusätzlicher Kontext mitgegeben.

Änderungen werden automatisch erkannt — einfach Dateien hinzufügen, ändern oder löschen.

Details: siehe `docs/MEINGESCHMACK-SYSTEM.md`
