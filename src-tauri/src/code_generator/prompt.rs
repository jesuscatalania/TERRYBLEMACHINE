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

    // User brief ALWAYS comes first and outranks the template. Earlier
    // ordering (template first) caused Claude to treat the template's
    // "hero + 3-column features + testimonials + CTA" defaults as hard
    // requirements and invent ORBIT-style marketing content around a
    // minimal user ask like "landing page with a dark planet in the
    // middle" — the user's 3D Three.js request got buried.
    if !input.prompt.trim().is_empty() {
        clauses.push(format!(
            "USER BRIEF (highest priority — build exactly this, do NOT add sections \
             or content the user didn't ask for):\n{}",
            input.prompt.trim()
        ));
    }

    let brief = input.template.brief();
    if !brief.is_empty() {
        clauses.push(format!(
            "Template default (use ONLY as a fallback when the user brief leaves \
             structure unspecified — if the user's brief is specific about layout \
             or contents, ignore this template): {brief}"
        ));
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

Build rules:
- Honor the USER BRIEF literally. If the user asks for a minimal single-section
  page (e.g. just a hero with a 3D object), produce exactly that — do not add
  navigation, features, pricing, testimonials, or made-up branding the user
  didn't request.
- Default tech: React + Tailwind CSS, responsive.
- Prefer self-contained single-file index.html when the user's brief fits one
  page. Only split into multiple files when complexity genuinely requires it.
- Do NOT invent a product name, logo text, or hero copy unless the user asked.
  If a title is needed, derive it directly from the user brief or leave it
  descriptive (e.g. \"3D Planet Scene\").";

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
            model_override: None,
        }
    }

    #[test]
    fn includes_template_brief_when_not_custom() {
        let p = build_prompt(&input("make me a site", Template::LandingPage), None);
        assert!(p.contains("Template default"));
        assert!(p.contains("hero"));
        assert!(p.contains("USER BRIEF"));
        assert!(p.contains("make me a site"));
    }

    #[test]
    fn custom_template_omits_template_clause() {
        let p = build_prompt(&input("do X", Template::Custom), None);
        assert!(!p.contains("Template default"));
        assert!(p.contains("USER BRIEF"));
        assert!(p.contains("do X"));
    }

    #[test]
    fn user_brief_appears_before_template_in_output() {
        let p = build_prompt(&input("just a planet", Template::LandingPage), None);
        let brief_pos = p.find("USER BRIEF").expect("USER BRIEF present");
        let template_pos = p.find("Template default").expect("Template default present");
        assert!(
            brief_pos < template_pos,
            "user brief must come before template hints",
        );
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
