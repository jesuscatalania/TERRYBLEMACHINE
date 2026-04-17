//! Negative-prompt builder.

use super::types::TasteRules;

/// Build a single "avoid these" string that can be passed verbatim to image
/// APIs that take a `negative_prompt` parameter. The result is a comma-
/// separated list of forbidden items + any hex colours whose mention should
/// be suppressed.
pub fn build_negative_prompt(rules: &TasteRules) -> String {
    let mut parts: Vec<String> = Vec::new();

    for f in &rules.forbidden {
        parts.push(f.trim().to_string());
    }

    if !parts.is_empty() {
        parts.join(", ")
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taste_engine::types::TasteRules;

    #[test]
    fn empty_rules_yield_empty_negative() {
        let rules = TasteRules::default();
        assert_eq!(build_negative_prompt(&rules), "");
    }

    #[test]
    fn forbidden_items_join_with_commas() {
        let rules = TasteRules {
            forbidden: vec![
                "Neon colours".into(),
                "Clipart aesthetic".into(),
                "Bevel & emboss".into(),
            ],
            ..Default::default()
        };
        let out = build_negative_prompt(&rules);
        assert_eq!(out, "Neon colours, Clipart aesthetic, Bevel & emboss");
    }

    #[test]
    fn whitespace_in_forbidden_items_is_trimmed() {
        let rules = TasteRules {
            forbidden: vec!["  Neon ".into(), " Clipart  ".into()],
            ..Default::default()
        };
        assert_eq!(build_negative_prompt(&rules), "Neon, Clipart");
    }
}
