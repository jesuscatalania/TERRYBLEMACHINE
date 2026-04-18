//! ZIP writer for brand-kit assets. Deflated compression, 0644 unix perms.
//! Consumer (T7 command / future CLI) passes a `destination` dir, a brand
//! slug, and the in-memory assets produced by the pipeline. Returns the
//! absolute path of the emitted ZIP.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::types::{BrandKitAsset, BrandKitError};

pub fn write_zip(
    destination: &Path,
    brand_slug: &str,
    assets: &[BrandKitAsset],
) -> Result<PathBuf, BrandKitError> {
    std::fs::create_dir_all(destination).map_err(|e| BrandKitError::Io(e.to_string()))?;
    let path = destination.join(format!("{brand_slug}-brand-kit.zip"));
    let file = File::create(&path).map_err(|e| BrandKitError::Io(e.to_string()))?;
    let mut zip = ZipWriter::new(file);
    let options: SimpleFileOptions = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    for asset in assets {
        zip.start_file(&asset.filename, options)
            .map_err(|e| BrandKitError::Io(e.to_string()))?;
        zip.write_all(&asset.bytes)
            .map_err(|e| BrandKitError::Io(e.to_string()))?;
    }
    zip.finish().map_err(|e| BrandKitError::Io(e.to_string()))?;
    Ok(path)
}

/// Normalize a brand name into a lowercase ASCII-alphanumeric slug with single
/// `-` separators. Empty/punctuation-only input falls back to `"brand"`.
pub fn slug_for(name: &str) -> String {
    let mut out = String::new();
    let mut prev_hyphen = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_hyphen = false;
        } else if !prev_hyphen {
            out.push('-');
            prev_hyphen = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "brand".into()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_normalizes_ascii_and_whitespace() {
        assert_eq!(slug_for("Hello World"), "hello-world");
        assert_eq!(slug_for("  TRIM  ME  "), "trim-me");
        assert_eq!(slug_for("multi___underscore"), "multi-underscore");
    }

    #[test]
    fn slug_falls_back_on_non_alphanumeric() {
        assert_eq!(slug_for("!!!"), "brand");
        assert_eq!(slug_for(""), "brand");
        assert_eq!(slug_for("---"), "brand");
    }

    #[test]
    fn slug_preserves_digits() {
        assert_eq!(slug_for("Brand 123"), "brand-123");
    }
}
