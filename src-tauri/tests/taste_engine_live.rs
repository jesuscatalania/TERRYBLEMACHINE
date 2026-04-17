//! End-to-end test that [`TasteEngine::refresh`] picks up new rule files.
//!
//! This mirrors the runtime setup (T9): the live watcher loop in `lib.rs`
//! calls `engine.refresh()` whenever `meingeschmack/` changes. Here we
//! simulate that by writing a rules file to disk and invoking `refresh()`
//! directly — asserting the profile observes the new rule.

use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use terryblemachine_lib::taste_engine::{StubVisionAnalyzer, TasteEngine};
use tokio::time::sleep;

#[tokio::test]
async fn refresh_picks_up_new_rules_file() {
    let tmp = TempDir::new().unwrap();
    let engine = Arc::new(TasteEngine::new(
        tmp.path().to_path_buf(),
        Arc::new(StubVisionAnalyzer::new()),
    ));

    // Initially no rules.
    engine.refresh().await.expect("refresh empty");
    let before = engine.profile().await;

    // Drop a rules file — parser walks <root>/regeln/*.md.
    let regeln = tmp.path().join("regeln");
    std::fs::create_dir_all(&regeln).unwrap();
    std::fs::write(
        regeln.join("rules.md"),
        "## Bevorzugt\n- Prefer warm palettes\n",
    )
    .unwrap();
    sleep(Duration::from_millis(50)).await;

    engine.refresh().await.expect("refresh after");
    let after = engine.profile().await;

    assert!(
        after.rules.preferred.len() > before.rules.preferred.len(),
        "expected new rule to appear (before: {:?}, after: {:?})",
        before.rules.preferred,
        after.rules.preferred
    );
    assert!(
        after
            .rules
            .preferred
            .iter()
            .any(|r| r.contains("warm palettes")),
        "expected the warm-palettes rule in {:?}",
        after.rules.preferred
    );
}
