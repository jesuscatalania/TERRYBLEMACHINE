//! Prompt builder — turns a [`GenerationInput`] + optional taste profile
//! into the single string we send to the LLM.

use crate::taste_engine::{build_negative_prompt, enrich_prompt, EnrichOptions, TasteRules};

use super::types::GenerationInput;

/// Build the complete prompt. Steps:
/// 1. Start from the user's brief.
/// 2. Prepend the template brief when provided.
/// 3. Append URL analysis snippets (title, colours, fonts, layout) when present.
/// 4. Run the whole thing through the taste engine's enricher (style +
///    palette + context + negatives).
/// 5. Append explicit instructions about the expected output format so
///    Claude returns deterministic multi-file JSON.
pub fn build_prompt(input: &GenerationInput, rules: Option<&TasteRules>) -> String {
    let mut clauses: Vec<String> = Vec::new();

    let brief = input.template.brief();
    if !brief.is_empty() {
        clauses.push(format!("Template: {brief}"));
    }

    if !input.prompt.trim().is_empty() {
        clauses.push(format!("User brief: {}", input.prompt.trim()));
    }

    if let Some(analysis) = &input.reference {
        let colors = analysis.colors.join(", ");
        let fonts = analysis.fonts.join(", ");
        clauses.push(format!(
            "Reference URL: {} — title \"{}\", layout \"{}\". Palette hints: [{colors}]. Fonts: [{fonts}].",
            analysis.url, analysis.title, analysis.layout
        ));
    }

    if let Some(path) = &input.image_path {
        clauses.push(format!("Reference image: {}", path.display()));
    }

    let core = clauses.join("\n");
    let enriched = match rules {
        Some(r) => enrich_prompt(
            &core,
            r,
            &EnrichOptions {
                module: Some(input.module.clone()),
                tags: Vec::new(),
                with_negative: false,
            },
        ),
        None => core,
    };

    let mut negative = String::new();
    if let Some(r) = rules {
        negative = build_negative_prompt(r);
    }

    let format_instructions = "\
Output strictly valid JSON with this shape (no prose outside JSON):
{
  \"summary\": \"<short description of what you generated>\",
  \"files\": [
    { \"path\": \"<relative-path>\", \"content\": \"<file body>\" }
  ]
}
Use React + Tailwind CSS. Default to responsive layouts. Produce at minimum
an `index.html` entry point plus any React components you need.";

    let mut parts = vec![enriched, format_instructions.to_string()];
    if !negative.is_empty() {
        parts.push(format!("Avoid: {negative}"));
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_generator::templates::Template;
    use crate::taste_engine::TasteRules;
    use crate::website_analyzer::AnalysisResult;
    use std::collections::HashMap;

    fn input(prompt: &str, template: Template) -> GenerationInput {
        GenerationInput {
            prompt: prompt.into(),
            template,
            reference: None,
            image_path: None,
            module: "website".into(),
        }
    }

    #[test]
    fn includes_template_brief_when_not_custom() {
        let p = build_prompt(&input("make me a site", Template::LandingPage), None);
        assert!(p.contains("Template:"));
        assert!(p.contains("hero"));
        assert!(p.contains("User brief: make me a site"));
    }

    #[test]
    fn custom_template_omits_template_clause() {
        let p = build_prompt(&input("do X", Template::Custom), None);
        assert!(!p.contains("Template:"));
        assert!(p.contains("User brief: do X"));
    }

    #[test]
    fn reference_url_analysis_appends_hints() {
        let mut i = input("blog", Template::Blog);
        i.reference = Some(AnalysisResult {
            url: "https://stripe.com".into(),
            status: 200,
            title: "Stripe".into(),
            description: None,
            colors: vec!["rgb(0, 0, 0)".into(), "rgb(255, 255, 255)".into()],
            fonts: vec!["Inter".into()],
            spacing: vec!["16px".into()],
            custom_properties: HashMap::new(),
            layout: "grid".into(),
            screenshot_path: None,
            assets: Vec::new(),
        });
        let p = build_prompt(&i, None);
        assert!(p.contains("Reference URL: https://stripe.com"));
        assert!(p.contains("Inter"));
        assert!(p.contains("grid"));
    }

    #[test]
    fn taste_rules_feed_into_enrichment() {
        let rules = TasteRules {
            preferred: vec!["Warm tones".into()],
            forbidden: vec!["Neon".into()],
            ..Default::default()
        };
        let p = build_prompt(&input("hero", Template::LandingPage), Some(&rules));
        assert!(p.contains("Style: Warm tones"));
        assert!(p.contains("Avoid: Neon"));
    }

    #[test]
    fn output_format_instruction_is_always_present() {
        let p = build_prompt(&input("x", Template::Custom), None);
        assert!(p.contains("Output strictly valid JSON"));
        assert!(p.contains("\"files\""));
        assert!(p.contains("React + Tailwind"));
    }
}
