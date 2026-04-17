//! End-to-end Taste-Engine smoke (Phase 2 verification).
//!
//! Creates a real meingeschmack/ fixture on disk, parses it, enriches a
//! prompt, and asserts the full pipeline works end-to-end outside the
//! module-local unit tests.

use std::fs;

use tempfile::TempDir;
use terryblemachine_lib::taste_engine::{
    build_negative_prompt, enrich_prompt, parse_meingeschmack_dir, EnrichOptions,
};

#[test]
fn full_pipeline_parses_rules_and_enriches_prompt() {
    let tmp = TempDir::new().unwrap();
    let regeln = tmp.path().join("regeln");
    fs::create_dir_all(&regeln).unwrap();
    fs::write(
        regeln.join("farben.md"),
        "## Bevorzugt\n- Warm earthy tones\n- Deep navy accents #1A1A2E\n\n\
         ## Verboten\n- Neon colours\n- Comic Sans\n\n\
         ## Kontext-Regeln\n\n### Websites\n- Dark mode first\n",
    )
    .unwrap();

    let rules = parse_meingeschmack_dir(tmp.path()).unwrap();
    assert_eq!(rules.preferred.len(), 2);
    assert_eq!(rules.forbidden, vec!["Neon colours", "Comic Sans"]);
    assert!(rules.hex_colors.contains(&"#1A1A2E".to_string()));
    assert_eq!(rules.context_rules.len(), 1);
    assert_eq!(rules.context_rules[0].context, "websites");

    let enriched = enrich_prompt(
        "A hero banner for a coffee website",
        &rules,
        &EnrichOptions::for_module("websites"),
    );
    // Must contain every channel the engine produces:
    assert!(enriched.starts_with("A hero banner for a coffee website"));
    assert!(enriched.contains("Style: Warm earthy tones"));
    assert!(enriched.contains("#1A1A2E"));
    assert!(enriched.contains("Context: Dark mode first."));
    assert!(enriched.contains("Negative: Neon colours, Comic Sans."));

    let neg = build_negative_prompt(&rules);
    assert_eq!(neg, "Neon colours, Comic Sans");
}
