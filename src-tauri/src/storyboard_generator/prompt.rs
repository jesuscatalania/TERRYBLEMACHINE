//! Prompt builder: StoryboardInput + taste rules → one Claude prompt.

use crate::taste_engine::{enrich_prompt, EnrichOptions, TasteRules};

use super::types::StoryboardInput;

pub fn build_prompt(input: &StoryboardInput, rules: Option<&TasteRules>) -> String {
    let mut clauses = Vec::new();
    let brief = input.template.brief();
    if !brief.is_empty() {
        clauses.push(format!("Template: {brief}"));
    }
    if !input.prompt.trim().is_empty() {
        clauses.push(format!("User brief: {}", input.prompt.trim()));
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

    let format_instructions = r#"
Return a STRICT JSON object with no prose. Shape:
{
  "summary": "short description of the spot",
  "template": "<template-name>",
  "shots": [
    {
      "index": 1,
      "description": "what happens in this shot (concise)",
      "style": "visual language: palette, mood, texture",
      "duration_s": 5,
      "camera": "camera movement/framing (e.g. 'dolly in', 'aerial', 'static wide')",
      "transition": "how this shot ends (e.g. 'cut', 'fade', 'dissolve', 'whip-pan')"
    }
  ]
}

HARD CONSTRAINTS (non-negotiable):
1. `duration_s` MUST be exactly 5 or 10 — no other values. The rendering pipeline (Kling V2 Master / V1.5 via fal.ai) rejects everything else with HTTP 422. If a shot needs 3s or 7s, round to 5; if it needs 8s or 12s, round to 10.
2. If the user's brief specifies an exact number of shots or cuts (e.g. "2 shots", "1 cut so 2 segments", "give me 3 shots of 10 seconds each"), produce EXACTLY that count and those durations. User constraints override any template default.
3. If the user's brief does NOT specify a shot count, default to 4-8 shots whose durations sum roughly to the template's target length.
4. Transitions: "cut" for a hard cut between two adjacent shots, "fade", "dissolve", "whip-pan", or "match-cut".
"#;
    format!("{enriched}\n\n{format_instructions}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storyboard_generator::types::StoryboardTemplate;

    #[test]
    fn includes_template_brief() {
        let p = build_prompt(
            &StoryboardInput {
                prompt: "Tell a story about a coffee shop".into(),
                template: StoryboardTemplate::Commercial,
                module: "video".into(),
                model_override: None,
            },
            None,
        );
        assert!(p.contains("Template:"));
        assert!(p.contains("commercial"));
    }

    #[test]
    fn user_brief_is_embedded() {
        let p = build_prompt(
            &StoryboardInput {
                prompt: "Moody rainy street".into(),
                template: StoryboardTemplate::Custom,
                module: "video".into(),
                model_override: None,
            },
            None,
        );
        assert!(p.contains("User brief: Moody rainy street"));
    }

    #[test]
    fn output_format_instruction_is_always_present() {
        let p = build_prompt(
            &StoryboardInput {
                prompt: "x".into(),
                template: StoryboardTemplate::Custom,
                module: "video".into(),
                model_override: None,
            },
            None,
        );
        assert!(p.contains("STRICT JSON"));
        assert!(p.contains("\"shots\""));
    }
}
