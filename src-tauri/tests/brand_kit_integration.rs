//! Integration tests for the brand_kit module.
//!
//! Covers the happy path (all 12 expected assets produced from a tiny
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

    // 8 sizes + 1 svg + 1 bw + 1 inverted + 1 style-guide.html = 12 assets
    assert_eq!(result.assets.len(), 12);
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
    assert!(filenames.contains(&"style-guide.html"));

    assert!(
        !result.style_guide_html.is_empty(),
        "style_guide_html should be non-empty (T5 placeholder or T6 real)"
    );
}

#[tokio::test]
async fn brand_kit_bw_and_inverted_preserve_alpha() {
    use image::{ImageBuffer, ImageFormat, Rgba};

    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src.png");

    // 2x2 PNG where one pixel is fully transparent
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(2, 2, |x, y| match (x, y) {
        (0, 0) => Rgba([255, 0, 0, 255]), // opaque red
        (1, 0) => Rgba([0, 255, 0, 128]), // semi-transparent green
        (0, 1) => Rgba([0, 0, 255, 0]),   // fully transparent
        _ => Rgba([255, 255, 255, 255]),
    });
    let mut buf = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut buf), ImageFormat::Png)
        .unwrap();
    std::fs::write(&src, &buf).unwrap();

    let kit = StandardBrandKit::new();
    let result = kit
        .build(BrandKitInput {
            logo_svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"/>"
                .into(),
            source_png_path: src,
            brand_name: "Acme".into(),
            primary_color: "#000".into(),
            accent_color: "#fff".into(),
            font: "Inter".into(),
        })
        .await
        .unwrap();

    // Pull the bw + inverted PNGs back out, decode them, and assert that
    // each variant's alpha channel still has at least one non-255 value
    // (i.e. transparency survived the transform).
    for fname in &["logo-bw.png", "logo-inverted.png"] {
        let asset = result.assets.iter().find(|a| a.filename == *fname).unwrap();
        let decoded = image::load_from_memory(&asset.bytes).unwrap().to_rgba8();
        let has_transparency = decoded.pixels().any(|p| p.0[3] != 255);
        assert!(
            has_transparency,
            "{fname} lost alpha channel — all pixels opaque after transform"
        );
    }
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

#[tokio::test]
async fn brand_kit_includes_style_guide_html() {
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

    let filenames: Vec<&str> = result.assets.iter().map(|a| a.filename.as_str()).collect();
    assert!(filenames.contains(&"style-guide.html"));
    let guide = result
        .assets
        .iter()
        .find(|a| a.filename == "style-guide.html")
        .unwrap();
    let html = String::from_utf8(guide.bytes.clone()).unwrap();
    assert!(html.contains("Acme"));
    assert!(html.contains("#e85d2d"));
    // Also verify the top-level BrandKitResult field still carries the same string
    assert_eq!(html, result.style_guide_html);
}

#[test]
fn zip_export_contains_all_assets() {
    use std::io::Read;
    use terryblemachine_lib::brand_kit::export::{slug_for, write_zip};
    use terryblemachine_lib::brand_kit::types::BrandKitAsset;

    let tmp = TempDir::new().unwrap();
    let assets = vec![
        BrandKitAsset {
            filename: "a.txt".into(),
            bytes: b"hello".to_vec(),
        },
        BrandKitAsset {
            filename: "b.svg".into(),
            bytes: b"<svg/>".to_vec(),
        },
    ];
    let path = write_zip(tmp.path(), &slug_for("Acme Brand"), &assets).unwrap();
    assert!(path.exists());
    assert!(path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .contains("acme-brand"));

    let bytes = std::fs::read(&path).unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();
    assert!(names.contains(&"a.txt".to_string()));
    assert!(names.contains(&"b.svg".to_string()));

    {
        let mut f = archive.by_name("a.txt").unwrap();
        let mut content = String::new();
        f.read_to_string(&mut content).unwrap();
        assert_eq!(content, "hello");
    }

    {
        let mut g = archive.by_name("b.svg").unwrap();
        let mut svg_content = String::new();
        g.read_to_string(&mut svg_content).unwrap();
        assert_eq!(svg_content, "<svg/>");
    }
}

#[tokio::test]
async fn build_plus_zip_roundtrip_contains_all_12_assets() {
    use std::io::Read;
    use terryblemachine_lib::brand_kit::export::{slug_for, write_zip};

    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src.png");
    std::fs::write(&src, tiny_png()).unwrap();

    let kit = StandardBrandKit::new();
    let input = BrandKitInput {
        logo_svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"/>".into(),
        source_png_path: src,
        brand_name: "Acme Brand".into(),
        primary_color: "#e85d2d".into(),
        accent_color: "#0E0E11".into(),
        font: "Inter".into(),
    };
    let brand_slug = slug_for(&input.brand_name);
    let result = kit.build(input).await.unwrap();

    let zip_dir = tmp.path().join("out");
    let zip_path = write_zip(&zip_dir, &brand_slug, &result.assets).unwrap();
    assert!(zip_path.exists());
    assert!(zip_path.ends_with("acme-brand-brand-kit.zip"));

    let bytes = std::fs::read(&zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    assert_eq!(archive.len(), 12);
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();
    for expected in &[
        "logo.svg",
        "favicon-16.png",
        "favicon-32.png",
        "favicon-64.png",
        "logo-128.png",
        "logo-256.png",
        "logo-512.png",
        "print-1024.png",
        "print-2048.png",
        "logo-bw.png",
        "logo-inverted.png",
        "style-guide.html",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "missing {expected} in zip"
        );
    }
    // Spot-check that style-guide.html survived the round-trip with content
    let mut f = archive.by_name("style-guide.html").unwrap();
    let mut html = String::new();
    f.read_to_string(&mut html).unwrap();
    assert!(html.contains("Acme Brand"));
}
