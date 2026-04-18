//! Integration tests for the vectorizer module.
//!
//! The happy path is covered with [`StubVectorizer`] so the test suite
//! doesn't depend on the full vtracer pipeline (which needs a real raster
//! on disk). A real-vtracer round-trip is included but marked `#[ignore]`:
//! run it with `cargo test -- --ignored` to validate after changes to the
//! pipeline glue.

use tempfile::TempDir;

use terryblemachine_lib::vectorizer::{
    ColorMode, StubVectorizer, VectorizeError, VectorizeInput, Vectorizer,
};

#[tokio::test]
async fn stub_vectorizer_returns_svg_for_existing_file() {
    let tmp = TempDir::new().unwrap();
    let img = tmp.path().join("x.png");
    std::fs::write(&img, b"fake-png").unwrap();

    let v = StubVectorizer::new();
    let result = v
        .vectorize(VectorizeInput {
            image_path: img,
            color_mode: ColorMode::Color,
            filter_speckle: 4,
            corner_threshold: 60,
        })
        .await
        .unwrap();
    assert!(result.svg.contains("<svg"));
    assert_eq!(result.width, 100);
    assert_eq!(result.height, 100);
}

#[tokio::test]
async fn stub_vectorizer_rejects_missing_file() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("nope.png");
    let v = StubVectorizer::new();
    let err = v
        .vectorize(VectorizeInput {
            image_path: missing,
            color_mode: ColorMode::Color,
            filter_speckle: 4,
            corner_threshold: 60,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, VectorizeError::InvalidInput(_)));
}

#[tokio::test]
async fn stub_vectorizer_rejects_corner_threshold_out_of_range() {
    let tmp = TempDir::new().unwrap();
    let img = tmp.path().join("x.png");
    std::fs::write(&img, b"fake-png").unwrap();

    let v = StubVectorizer::new();
    let err = v
        .vectorize(VectorizeInput {
            image_path: img,
            color_mode: ColorMode::Color,
            filter_speckle: 4,
            corner_threshold: 200,
        })
        .await
        .unwrap_err();
    match err {
        VectorizeError::InvalidInput(msg) => assert!(msg.contains("corner_threshold")),
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

#[tokio::test]
async fn stub_vectorizer_rejects_filter_speckle_out_of_range() {
    let tmp = TempDir::new().unwrap();
    let img = tmp.path().join("x.png");
    std::fs::write(&img, b"fake-png").unwrap();

    let v = StubVectorizer::new();
    let err = v
        .vectorize(VectorizeInput {
            image_path: img,
            color_mode: ColorMode::Bw,
            filter_speckle: 200,
            corner_threshold: 60,
        })
        .await
        .unwrap_err();
    match err {
        VectorizeError::InvalidInput(msg) => assert!(msg.contains("filter_speckle")),
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

/// Real vtracer round-trip. `#[ignore]` because it pulls the full
/// vtracer/visioncortex stack at test time — slow and unnecessary in the
/// default suite when stub coverage is already in place. Run with
/// `cargo test -- --ignored` to validate end-to-end.
#[tokio::test]
#[ignore]
async fn vtracer_pipeline_converts_real_png() {
    let tmp = TempDir::new().unwrap();
    let png = tmp.path().join("red.png");
    let img = image::RgbImage::from_pixel(16, 16, image::Rgb([232, 93, 45]));
    img.save(&png).expect("save test png");

    let v = terryblemachine_lib::vectorizer::VtracerPipeline::new();
    let result = v
        .vectorize(VectorizeInput {
            image_path: png,
            color_mode: ColorMode::Color,
            filter_speckle: 4,
            corner_threshold: 60,
        })
        .await
        .expect("vtracer converts");
    assert!(result.svg.contains("<svg"));
    assert!(result.width > 0);
    assert!(result.height > 0);
}
