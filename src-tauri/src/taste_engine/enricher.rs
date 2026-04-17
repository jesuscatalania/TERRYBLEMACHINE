//! Prompt enricher — glues the user's prompt, the parsed rules, and any
//! matching context snippets into a single string suitable for an LLM or
//! image API.

use super::negative::build_negative_prompt;
use super::types::TasteRules;

/// What generation context the caller is in — used to match
/// `## Kontext-Regeln` subheadings.
#[derive(Debug, Clone, Default)]
pub struct EnrichOptions {
    /// Module identifier (`"website"`, `"logo"`, `"video"`, …). Matched
    /// case-insensitively against context headings.
    pub module: Option<String>,
    /// Free-form additional context keywords (e.g. `"dark mode"`, `"hero"`).
    pub tags: Vec<String>,
    /// Whether to append a `Negative:` clause at the end.
    #[doc(hidden)]
    pub with_negative: bool,
}

impl EnrichOptions {
    pub fn for_module(module: impl Into<String>) -> Self {
        Self {
            module: Some(module.into()),
            tags: Vec::new(),
            with_negative: true,
        }
    }

    pub fn none() -> Self {
        Self {
            module: None,
            tags: Vec::new(),
            with_negative: false,
        }
    }
}

/// Enrich a prompt with the rules' preferred traits, matched context rules,
/// palette hints, and (optionally) a trailing negative-prompt clause.
pub fn enrich_prompt(original: &str, rules: &TasteRules, opts: &EnrichOptions) -> String {
    let original = original.trim();
    let mut clauses: Vec<String> = Vec::new();

    if !original.is_empty() {
        clauses.push(original.to_string());
    }

    if !rules.preferred.is_empty() {
        clauses.push(format!("Style: {}.", rules.preferred.join("; ")));
    }

    // Colour hint from hex codes + named palettes.
    let mut palette_hints: Vec<String> = Vec::new();
    if !rules.hex_colors.is_empty() {
        palette_hints.push(rules.hex_colors.join(", "));
    }
    for palette in &rules.palettes {
        if !palette.hex.is_empty() {
            palette_hints.push(format!("{}: {}", palette.name, palette.hex.join(", ")));
        }
    }
    if !palette_hints.is_empty() {
        clauses.push(format!("Palette: {}.", palette_hints.join(" / ")));
    }

    // Matched context rules.
    let module_lc = opts.module.as_deref().map(|m| m.to_ascii_lowercase());
    let tags_lc: Vec<String> = opts.tags.iter().map(|t| t.to_ascii_lowercase()).collect();
    let matched: Vec<&str> = rules
        .context_rules
        .iter()
        .filter(|ctx| {
            if let Some(ref m) = module_lc {
                if ctx.context.contains(m.as_str()) {
                    return true;
                }
            }
            tags_lc.iter().any(|t| ctx.context.contains(t.as_str()))
        })
        .flat_map(|ctx| ctx.rules.iter().map(|s| s.as_str()))
        .collect();
    if !matched.is_empty() {
        clauses.push(format!("Context: {}.", matched.join("; ")));
    }

    // Trailing negative.
    if opts.with_negative {
        let neg = build_negative_prompt(rules);
        if !neg.is_empty() {
            clauses.push(format!("Negative: {neg}."));
        }
    }

    clauses.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taste_engine::types::{ContextRule, Palette, TasteRules};

    fn sample_rules() -> TasteRules {
        TasteRules {
            preferred: vec!["Warm earthy tones".into(), "Generous whitespace".into()],
            forbidden: vec!["Neon colours".into(), "Clipart".into()],
            context_rules: vec![
                ContextRule {
                    context: "websites".into(),
                    rules: vec!["Dark mode first".into()],
                },
                ContextRule {
                    context: "logos".into(),
                    rules: vec!["Max 3 colours".into()],
                },
            ],
            palettes: vec![Palette {
                name: "Primär".into(),
                hex: vec!["#2D3436".into(), "#D4A373".into()],
            }],
            hex_colors: vec!["#2D3436".into(), "#D4A373".into()],
        }
    }

    #[test]
    fn empty_rules_return_original_prompt() {
        let out = enrich_prompt(
            "A coffee website hero",
            &TasteRules::default(),
            &EnrichOptions::none(),
        );
        assert_eq!(out, "A coffee website hero");
    }

    #[test]
    fn preferred_traits_are_appended_as_style_clause() {
        let rules = sample_rules();
        let out = enrich_prompt("A hero banner", &rules, &EnrichOptions::none());
        assert!(out.starts_with("A hero banner"));
        assert!(out.contains("Style: Warm earthy tones; Generous whitespace"));
    }

    #[test]
    fn palette_clause_lists_hex_codes() {
        let rules = sample_rules();
        let out = enrich_prompt("x", &rules, &EnrichOptions::none());
        assert!(out.contains("Palette:"));
        assert!(out.contains("#2D3436"));
        assert!(out.contains("Primär"));
    }

    #[test]
    fn matching_module_appends_context_rules() {
        let rules = sample_rules();
        let out = enrich_prompt("x", &rules, &EnrichOptions::for_module("websites"));
        assert!(out.contains("Context: Dark mode first."));
        assert!(!out.contains("Max 3 colours"));
    }

    #[test]
    fn non_matching_module_does_not_add_context() {
        let rules = sample_rules();
        let out = enrich_prompt("x", &rules, &EnrichOptions::for_module("video"));
        assert!(!out.contains("Context:"));
    }

    #[test]
    fn negative_clause_appears_only_when_requested() {
        let rules = sample_rules();
        let off = enrich_prompt("x", &rules, &EnrichOptions::none());
        assert!(!off.contains("Negative:"));
        let on = enrich_prompt("x", &rules, &EnrichOptions::for_module("websites"));
        assert!(on.contains("Negative: Neon colours, Clipart."));
    }

    #[test]
    fn tags_match_context_rules_even_without_module() {
        let rules = sample_rules();
        let opts = EnrichOptions {
            module: None,
            tags: vec!["logos".into()],
            with_negative: false,
        };
        let out = enrich_prompt("new badge", &rules, &opts);
        assert!(out.contains("Max 3 colours"));
    }
}
