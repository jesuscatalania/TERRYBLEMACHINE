//! Integration tests for the brand_kit module.
//!
//! Covers the happy path (all 11 expected assets produced from a tiny
//! synthetic PNG) and the two validation rejections (empty SVG, missing
//! source raster). The tiny PNG is built in-memory via the `image` crate
//! rather than checked in so the test suite has no binary fixtures.

use tempfile::TempDir;
use terryblemachine_lib::brand_kit::{
    BrandKitBuilder, BrandKitError, BrandKitInput, StandardBrandKit,
};

fn tiny_png() -> Vec<u8> {
    // Build a valid 2x2 RGBA PNG via the image crate rather than hand-coding bytes
    use image::{ImageBuffer, ImageFormat, Rgba};
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(2, 2, |x, y| match (x, y) {
        (0, 0) => Rgba([255, 0, 0, 255]),
        (1, 0) => Rgba([0, 255, 0, 255]),
        (0, 1) => Rgba([0, 0, 255, 255]),
        _ => Rgba([255, 255, 255, 255]),
    });
    let mut buf = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut buf), ImageFormat::Png)
        .unwrap();
    buf
}

#[tokio::test]
async fn brand_kit_produces_all_sizes_plus_variants() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src.png");
    std::fs::write(&src, tiny_png()).unwrap();

    let kit = StandardBrandKit::new();
    let result = kit
        .build(BrandKitInput {
            logo_svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"/>"
                .into(),
            source_png_path: src,
            brand_name: "Acme".into(),
            primary_color: "#e85d2d".into(),
            accent_color: "#0E0E11".into(),
            font: "Inter".into(),
        })
        .await
        .unwrap();

    // 8 sizes + 1 svg + 1 bw + 1 inverted = 11 assets
    assert_eq!(result.assets.len(), 11);
    let filenames: Vec<&str> = result.assets.iter().map(|a| a.filename.as_str()).collect();
    assert!(filenames.contains(&"logo.svg"));
    assert!(filenames.contains(&"favicon-16.png"));
    assert!(filenames.contains(&"favicon-32.png"));
    assert!(filenames.contains(&"favicon-64.png"));
    assert!(filenames.contains(&"logo-128.png"));
    assert!(filenames.contains(&"logo-256.png"));
    assert!(filenames.contains(&"logo-512.png"));
    assert!(filenames.contains(&"print-1024.png"));
    assert!(filenames.contains(&"print-2048.png"));
    assert!(filenames.contains(&"logo-bw.png"));
    assert!(filenames.contains(&"logo-inverted.png"));
}

#[tokio::test]
async fn brand_kit_rejects_empty_svg() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src.png");
    std::fs::write(&src, tiny_png()).unwrap();

    let kit = StandardBrandKit::new();
    let err = kit
        .build(BrandKitInput {
            logo_svg: "   ".into(),
            source_png_path: src,
            brand_name: "X".into(),
            primary_color: "#000".into(),
            accent_color: "#fff".into(),
            font: "Inter".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, BrandKitError::InvalidInput(_)));
}

#[tokio::test]
async fn brand_kit_rejects_missing_source_png() {
    let kit = StandardBrandKit::new();
    let err = kit
        .build(BrandKitInput {
            logo_svg: "<svg/>".into(),
            source_png_path: std::path::PathBuf::from("/tmp/does-not-exist-terryblemachine.png"),
            brand_name: "X".into(),
            primary_color: "#000".into(),
            accent_color: "#fff".into(),
            font: "Inter".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, BrandKitError::InvalidInput(_)));
}
