//! Markdown rule parser for `meingeschmack/regeln/*.md`.

use std::path::Path;

use regex::Regex;

use super::errors::TasteError;
use super::types::{ContextRule, Palette, TasteRules};

/// Parse a single Markdown file into [`TasteRules`].
///
/// Section recognition is based on `## ` headings matched case-insensitively
/// against German + English keywords:
///
/// | Heading | Target |
/// |---------|--------|
/// | Bevorzugt / Preferred | `preferred` |
/// | Verboten / Forbidden / No-Gos | `forbidden` |
/// | Kontext-Regeln / Context rules | `context_rules` |
/// | Paletten / Palettes | `palettes` |
///
/// Any section of sub-heading (`### Primär`) under `## Paletten` starts a
/// named palette. List items (`- ...`) become individual rules. HEX codes
/// (`#RRGGBB` or `#RGB`) are collected into `hex_colors` regardless of which
/// section they appear in.
pub fn parse_markdown_rules(content: &str) -> TasteRules {
    let hex_re = Regex::new(r"#(?:[0-9A-Fa-f]{3}){1,2}\b").expect("valid regex");

    let mut rules = TasteRules::default();
    let mut section = Section::None;
    let mut current_palette: Option<Palette> = None;
    let mut current_context: Option<ContextRule> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim_end();

        // Collect hex codes no matter the section.
        for cap in hex_re.find_iter(line) {
            let hex = normalize_hex(cap.as_str());
            if !rules
                .hex_colors
                .iter()
                .any(|h| h.eq_ignore_ascii_case(&hex))
            {
                rules.hex_colors.push(hex);
            }
        }

        if let Some(heading) = line.strip_prefix("## ") {
            // Close any open palette / context before switching section.
            flush_palette(&mut current_palette, &mut rules);
            flush_context(&mut current_context, &mut rules);
            section = classify_section(heading.trim());
            continue;
        }

        if let Some(sub) = line.strip_prefix("### ") {
            // Sub-heading semantics differ per section.
            match section {
                Section::Palettes => {
                    flush_palette(&mut current_palette, &mut rules);
                    current_palette = Some(Palette {
                        name: sub.trim().to_string(),
                        hex: Vec::new(),
                    });
                }
                Section::Context => {
                    flush_context(&mut current_context, &mut rules);
                    current_context = Some(ContextRule {
                        context: sub.trim().to_ascii_lowercase(),
                        rules: Vec::new(),
                    });
                }
                _ => {}
            }
            continue;
        }

        let Some(item) = strip_list_bullet(line) else {
            continue;
        };
        let item = item.trim().to_string();
        if item.is_empty() {
            continue;
        }

        match section {
            Section::Preferred => rules.preferred.push(item),
            Section::Forbidden => rules.forbidden.push(item),
            Section::Context => {
                if let Some(cr) = current_context.as_mut() {
                    cr.rules.push(item);
                } else {
                    // Inline "- Für Websites: Dark mode first"
                    if let Some((context, rest)) = item.split_once(':') {
                        rules.context_rules.push(ContextRule {
                            context: context.trim().to_ascii_lowercase(),
                            rules: vec![rest.trim().to_string()],
                        });
                    }
                }
            }
            Section::Palettes => {
                if let Some(p) = current_palette.as_mut() {
                    for cap in hex_re.find_iter(&item) {
                        p.hex.push(normalize_hex(cap.as_str()));
                    }
                } else if let Some((name, rest)) = item.split_once(':') {
                    // Inline "- Primär: #aaa, #bbb"
                    let mut pal = Palette {
                        name: name.trim().to_string(),
                        hex: Vec::new(),
                    };
                    for cap in hex_re.find_iter(rest) {
                        pal.hex.push(normalize_hex(cap.as_str()));
                    }
                    if !pal.hex.is_empty() {
                        rules.palettes.push(pal);
                    }
                }
            }
            Section::None => {}
        }
    }

    // Close the last open palette / context.
    flush_palette(&mut current_palette, &mut rules);
    flush_context(&mut current_context, &mut rules);

    rules
}

/// Walk `<root>/regeln/*.md` and merge every file into a single [`TasteRules`].
/// Missing directory → empty rules (not an error).
pub fn parse_meingeschmack_dir(root: &Path) -> Result<TasteRules, TasteError> {
    let regeln = root.join("regeln");
    if !regeln.exists() {
        return Ok(TasteRules::default());
    }
    let mut merged = TasteRules::default();
    for entry in std::fs::read_dir(&regeln)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let body = std::fs::read_to_string(&path)?;
        let parsed = parse_markdown_rules(&body);
        merged.preferred.extend(parsed.preferred);
        merged.forbidden.extend(parsed.forbidden);
        merged.context_rules.extend(parsed.context_rules);
        merged.palettes.extend(parsed.palettes);
        for hex in parsed.hex_colors {
            if !merged
                .hex_colors
                .iter()
                .any(|h| h.eq_ignore_ascii_case(&hex))
            {
                merged.hex_colors.push(hex);
            }
        }
    }
    Ok(merged)
}

// ─── Section classification ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Preferred,
    Forbidden,
    Context,
    Palettes,
}

fn classify_section(heading: &str) -> Section {
    let lower = heading.to_ascii_lowercase();
    if matches_any(&lower, &["bevorzugt", "preferred", "erlaubt", "allowed"]) {
        Section::Preferred
    } else if matches_any(
        &lower,
        &[
            "verboten",
            "forbidden",
            "no-gos",
            "no gos",
            "absolute no-gos",
            "disallowed",
            "dont",
            "don't",
        ],
    ) {
        Section::Forbidden
    } else if matches_any(&lower, &["kontext-regeln", "kontext", "context"]) {
        Section::Context
    } else if matches_any(&lower, &["paletten", "palettes", "palette"]) {
        Section::Palettes
    } else {
        Section::None
    }
}

fn matches_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn strip_list_bullet(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("- ") {
        return Some(rest);
    }
    if let Some(rest) = trimmed.strip_prefix("* ") {
        return Some(rest);
    }
    None
}

fn normalize_hex(s: &str) -> String {
    // Expand #abc → #aabbcc so downstream comparisons don't collide.
    if s.len() == 4 {
        let chars: Vec<char> = s.chars().collect();
        let expanded: String = chars[1..].iter().flat_map(|c| [*c, *c]).collect();
        format!("#{expanded}").to_ascii_uppercase()
    } else {
        s.to_ascii_uppercase()
    }
}

fn flush_palette(current: &mut Option<Palette>, rules: &mut TasteRules) {
    if let Some(p) = current.take() {
        if !p.hex.is_empty() {
            rules.palettes.push(p);
        }
    }
}

fn flush_context(current: &mut Option<ContextRule>, rules: &mut TasteRules) {
    if let Some(c) = current.take() {
        if !c.rules.is_empty() {
            rules.context_rules.push(c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn empty_input_yields_empty_rules() {
        let r = parse_markdown_rules("");
        assert!(r.preferred.is_empty());
        assert!(r.forbidden.is_empty());
        assert!(r.hex_colors.is_empty());
    }

    #[test]
    fn preferred_section_collects_list_items() {
        let md = "
## Bevorzugt
- Warm, earthy tones
- Deep navy accents
- Minimal whitespace
";
        let r = parse_markdown_rules(md);
        assert_eq!(r.preferred.len(), 3);
        assert_eq!(r.preferred[0], "Warm, earthy tones");
    }

    #[test]
    fn forbidden_section_collects_list_items() {
        let md = "
## Verboten
- Neonfarben jeglicher Art
- Comic Sans
- Stock-Photo Looks
";
        let r = parse_markdown_rules(md);
        assert_eq!(r.forbidden.len(), 3);
        assert!(r.forbidden.iter().any(|f| f.contains("Neonfarben")));
    }

    #[test]
    fn english_heading_aliases_work() {
        let r = parse_markdown_rules("## Preferred\n- Calm palette\n## Forbidden\n- Bright neon\n");
        assert_eq!(r.preferred, vec!["Calm palette".to_string()]);
        assert_eq!(r.forbidden, vec!["Bright neon".to_string()]);
    }

    #[test]
    fn hex_codes_are_auto_detected_and_uppercased() {
        let md = "
## Bevorzugt
- Use #1a1a2e for deep background
- Ocker #D4A373 as accent
- Also #fff something
";
        let r = parse_markdown_rules(md);
        assert!(r.hex_colors.iter().any(|h| h == "#1A1A2E"));
        assert!(r.hex_colors.iter().any(|h| h == "#D4A373"));
        // Short form #fff → expanded #FFFFFF
        assert!(r.hex_colors.iter().any(|h| h == "#FFFFFF"));
    }

    #[test]
    fn palettes_section_collects_named_palettes() {
        let md = "
## Paletten

### Primär
- #2D3436
- #D4A373
- #FAEDCD

### Akzent
- #E9C46A
- #264653
";
        let r = parse_markdown_rules(md);
        assert_eq!(r.palettes.len(), 2);
        assert_eq!(r.palettes[0].name, "Primär");
        assert_eq!(r.palettes[0].hex.len(), 3);
        assert_eq!(r.palettes[1].name, "Akzent");
        assert_eq!(r.palettes[1].hex.len(), 2);
    }

    #[test]
    fn palettes_inline_form_is_supported() {
        let md = "
## Paletten
- Primär: #2D3436, #D4A373
- Akzent: #E9C46A, #264653
";
        let r = parse_markdown_rules(md);
        assert_eq!(r.palettes.len(), 2);
        assert_eq!(r.palettes[0].name, "Primär");
        assert_eq!(r.palettes[0].hex.len(), 2);
    }

    #[test]
    fn context_rules_capture_subheadings() {
        let md = "
## Kontext-Regeln

### Websites
- Dark mode first
- Dense layouts

### Logos
- Max 3 colors
";
        let r = parse_markdown_rules(md);
        assert_eq!(r.context_rules.len(), 2);
        assert_eq!(r.context_rules[0].context, "websites");
        assert_eq!(r.context_rules[0].rules.len(), 2);
        assert_eq!(r.context_rules[1].context, "logos");
    }

    #[test]
    fn context_rules_inline_form() {
        let md = "
## Kontext-Regeln
- Für Websites: Dark mode first
- Für Logos: Max 3 colors
";
        let r = parse_markdown_rules(md);
        assert_eq!(r.context_rules.len(), 2);
        assert_eq!(r.context_rules[0].context, "für websites");
        assert_eq!(r.context_rules[0].rules[0], "Dark mode first");
    }

    #[test]
    fn hex_codes_deduplicate_case_insensitive() {
        let r = parse_markdown_rules("- #d4a373\n- #D4A373\n");
        assert_eq!(r.hex_colors.len(), 1);
        assert_eq!(r.hex_colors[0], "#D4A373");
    }

    #[test]
    fn parse_meingeschmack_dir_merges_every_md_file() {
        let tmp = TempDir::new().unwrap();
        let regeln = tmp.path().join("regeln");
        std::fs::create_dir_all(&regeln).unwrap();
        std::fs::write(regeln.join("farben.md"), "## Bevorzugt\n- Warm tones\n").unwrap();
        std::fs::write(regeln.join("verboten.md"), "## Verboten\n- Neon colours\n").unwrap();
        let r = parse_meingeschmack_dir(tmp.path()).unwrap();
        assert_eq!(r.preferred, vec!["Warm tones"]);
        assert_eq!(r.forbidden, vec!["Neon colours"]);
    }

    #[test]
    fn parse_meingeschmack_dir_ignores_non_markdown_files() {
        let tmp = TempDir::new().unwrap();
        let regeln = tmp.path().join("regeln");
        std::fs::create_dir_all(&regeln).unwrap();
        std::fs::write(regeln.join("README.txt"), "## Bevorzugt\n- ignored\n").unwrap();
        let r = parse_meingeschmack_dir(tmp.path()).unwrap();
        assert!(r.preferred.is_empty());
    }

    #[test]
    fn parse_meingeschmack_dir_missing_folder_is_ok() {
        let tmp = TempDir::new().unwrap();
        let r = parse_meingeschmack_dir(tmp.path()).unwrap();
        assert_eq!(r, TasteRules::default());
    }

    #[test]
    fn parse_meingeschmack_dir_dedupes_hex_codes_across_files() {
        let tmp = TempDir::new().unwrap();
        let regeln = tmp.path().join("regeln");
        std::fs::create_dir_all(&regeln).unwrap();
        std::fs::write(regeln.join("a.md"), "## Bevorzugt\n- #d4a373\n").unwrap();
        std::fs::write(regeln.join("b.md"), "## Bevorzugt\n- #D4A373 variant\n").unwrap();
        let r = parse_meingeschmack_dir(tmp.path()).unwrap();
        assert_eq!(r.hex_colors.len(), 1);
    }
}
