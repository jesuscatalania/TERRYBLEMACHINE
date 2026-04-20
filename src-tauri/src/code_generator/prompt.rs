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
        // Rich structured block — gives Claude real signals (layout shape,
        // content excerpts, detected features) instead of the one-line
        // "title + colours" summary that used to produce generic output.
        let colors = analysis.colors.join(", ");
        let fonts = analysis.fonts.join(", ");
        let nav = if analysis.nav_items.is_empty() {
            "—".to_string()
        } else {
            analysis.nav_items.join(", ")
        };
        let sections = if analysis.section_headings.is_empty() {
            "—".to_string()
        } else {
            analysis.section_headings.join(", ")
        };
        let ctas = if analysis.cta_labels.is_empty() {
            "—".to_string()
        } else {
            analysis.cta_labels.join(", ")
        };
        let paragraphs = if analysis.paragraph_sample.is_empty() {
            "—".to_string()
        } else {
            analysis.paragraph_sample.join(" | ")
        };
        let typography = if analysis.typography.is_empty() {
            "—".to_string()
        } else {
            analysis
                .typography
                .iter()
                .map(|t| format!("{}/{} {}", t.size, t.weight, t.family))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let bg = analysis.color_roles.bg.as_deref().unwrap_or("—");
        let fg = analysis.color_roles.fg.as_deref().unwrap_or("—");
        let accent = analysis.color_roles.accent.as_deref().unwrap_or("—");
        let features = &analysis.detected_features;
        let hero = analysis.hero_text.as_deref().unwrap_or("—");
        let screenshot_line = match &analysis.screenshot_path {
            Some(p) => format!(
                "\n- Screenshot saved at: {} (not available as vision input in this call, \
                 so use the textual signals above to approximate the design)",
                p.display()
            ),
            None => String::new(),
        };

        clauses.push(format!(
            "REFERENCE SITE ANALYSIS — base the generated design on these extracted signals:\n\
             - URL: {url}\n\
             - Title: {title}\n\
             - Hero: {hero}\n\
             - Nav: {nav}\n\
             - Sections: {sections}\n\
             - CTAs: {ctas}\n\
             - Paragraph samples: {paragraphs}\n\
             - Detected features: canvas={has_canvas}, webgl={has_webgl}, three.js={has_three_js}, \
               video={has_video}, form={has_form}, iframe={has_iframe}\n\
             - Typography hierarchy: {typography}\n\
             - Color roles: bg={bg}, fg={fg}, accent={accent}\n\
             - Raw palette: [{colors}]\n\
             - Fonts: [{fonts}]\n\
             - Layout shape: {layout}{screenshot_line}",
            url = analysis.url,
            title = analysis.title,
            layout = analysis.layout,
            has_canvas = features.has_canvas,
            has_webgl = features.has_webgl,
            has_three_js = features.has_three_js,
            has_video = features.has_video,
            has_form = features.has_form,
            has_iframe = features.has_iframe,
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

HARD OUTPUT CONTRACT — non-negotiable:
- NEVER ask clarifying questions. NEVER reply with prose like \"Which page
  should I copy? Please share a URL or screenshot.\" — the user cannot
  respond; this is a one-shot non-interactive call.
- If information is missing, unclear, or contradictory, pick the most
  plausible interpretation and build something that matches the user's
  intent as closely as possible. If no reference content is attached and
  the brief says \"copy this site\", invent a plausible page that matches
  the brief's theme.
- Your response must parse as JSON with the shape above — no markdown
  fences, no preamble, no trailing prose. The FIRST character of your
  response must be `{`.

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
            hero_text: None,
            nav_items: Vec::new(),
            section_headings: Vec::new(),
            paragraph_sample: Vec::new(),
            cta_labels: Vec::new(),
            detected_features: Default::default(),
            typography: Vec::new(),
            image_urls: Vec::new(),
            color_roles: Default::default(),
        });
        let p = build_prompt(&i, None);
        assert!(p.contains("REFERENCE SITE ANALYSIS"));
        assert!(p.contains("URL: https://stripe.com"));
        assert!(p.contains("Inter"));
        assert!(p.contains("grid"));
    }

    #[test]
    fn reference_url_analysis_injects_rich_signals() {
        use crate::website_analyzer::{ColorRoles, DetectedFeatures, TypographyStyle};

        let mut i = input("site", Template::LandingPage);
        i.reference = Some(AnalysisResult {
            url: "https://ilithya.rocks".into(),
            status: 200,
            title: "ilithya.rocks".into(),
            description: None,
            colors: vec!["rgb(20, 20, 20)".into()],
            fonts: vec!["Times".into()],
            spacing: vec![],
            custom_properties: HashMap::new(),
            layout: "flex".into(),
            screenshot_path: Some("/tmp/tm-analyze-xyz/screenshot.png".into()),
            assets: Vec::new(),
            hero_text: Some("WE MAKE WEIRD WEBSITES".into()),
            nav_items: vec!["Home".into(), "Work".into()],
            section_headings: vec!["Features".into()],
            paragraph_sample: vec!["A paragraph.".into()],
            cta_labels: vec!["Get started".into()],
            detected_features: DetectedFeatures {
                has_canvas: true,
                has_webgl: true,
                has_three_js: true,
                ..Default::default()
            },
            typography: vec![TypographyStyle {
                size: "64px".into(),
                weight: "700".into(),
                family: "Times".into(),
            }],
            image_urls: vec!["https://ilithya.rocks/hero.jpg".into()],
            color_roles: ColorRoles {
                bg: Some("rgb(20, 20, 20)".into()),
                fg: Some("rgb(226, 226, 226)".into()),
                accent: Some("rgb(46, 111, 239)".into()),
            },
        });
        let p = build_prompt(&i, None);
        assert!(p.contains("REFERENCE SITE ANALYSIS"));
        assert!(p.contains("Hero: WE MAKE WEIRD WEBSITES"));
        assert!(p.contains("Nav: Home, Work"));
        assert!(p.contains("canvas=true"));
        assert!(p.contains("webgl=true"));
        assert!(p.contains("three.js=true"));
        assert!(p.contains("64px/700 Times"));
        assert!(p.contains("bg=rgb(20, 20, 20)"));
        assert!(p.contains("accent=rgb(46, 111, 239)"));
        assert!(p.contains("/tmp/tm-analyze-xyz/screenshot.png"));
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
