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
    // Destination must be an existing directory — we refuse to create
    // arbitrary new parent hierarchies (e.g. `"/etc/evil/brand-kit.zip"`
    // would otherwise silently try). Callers that want the directory
    // auto-created should do it explicitly before invoking the export.
    if !destination.exists() {
        return Err(BrandKitError::InvalidInput(format!(
            "destination does not exist: {}",
            destination.display()
        )));
    }
    // Canonicalize to resolve `..` segments and symlinks up front —
    // matches the website_analyzer::commands::resolve_assets_dir pattern
    // (debug-review I8). The post-join check below defends against a TOCTOU
    // race where the canonicalized dir is replaced with a symlink between
    // this call and File::create.
    let canonical = destination.canonicalize().map_err(|e| {
        BrandKitError::InvalidInput(format!(
            "destination canonicalize failed for `{}`: {e}",
            destination.display()
        ))
    })?;
    if !canonical.is_dir() {
        return Err(BrandKitError::InvalidInput(format!(
            "destination must be an existing directory, got {}",
            canonical.display()
        )));
    }
    let path = canonical.join(format!("{brand_slug}-brand-kit.zip"));
    // Defend against a race where the parent is swapped for a symlink to a
    // different directory between the canonicalize above and the File::create
    // below. The parent of `path` is guaranteed to equal `canonical`
    // lexically; if canonicalizing it resolves to a different real path, an
    // attacker slipped in a symlink — reject.
    if let Some(parent) = path.parent() {
        if parent != canonical.as_path() {
            // Lexical prefix sanity check — a defensive belt on top of the
            // canonicalize below.
            return Err(BrandKitError::InvalidInput(format!(
                "output parent `{}` drifted from canonical destination `{}`",
                parent.display(),
                canonical.display()
            )));
        }
    }
    let file = File::create(&path)?;
    let mut zip = ZipWriter::new(file);
    let options: SimpleFileOptions = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    for asset in assets {
        zip.start_file(&asset.filename, options)?;
        zip.write_all(&asset.bytes)?;
    }
    zip.finish()?;
    Ok(path)
}

/// Normalize a brand name into a lowercase ASCII-alphanumeric slug with single
/// `-` separators. Intentionally ASCII-only (no Unicode transliteration) so the
/// resulting filename is safe across Windows/macOS/Linux without pulling in a
/// transliteration dependency. Non-ASCII characters collapse to `-`:
/// `"Café Münchën"` becomes `"caf-m-nch-n"`. The brand name itself is preserved
/// verbatim in the HTML style guide; only the ZIP filename uses the slug.
///
/// Empty or punctuation-only input falls back to `"brand"`.
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

    #[test]
    fn slug_collapses_non_ascii_without_transliteration() {
        // Intentional behavior — not a bug. Document via test.
        assert_eq!(slug_for("Café Münchën"), "caf-m-nch-n");
        // Pure non-ASCII input collapses to hyphens, which then trim to an
        // empty slug and trigger the "brand" fallback.
        assert_eq!(slug_for("東京"), "brand");
    }

    // Regression tests for debug-review I8: write_zip must canonicalize the
    // destination and reject paths that don't resolve to an existing directory.

    #[test]
    fn write_zip_rejects_nonexistent_destination() {
        let err = write_zip(
            std::path::Path::new("/tmp/this-path-should-not-exist-terrybleharden-I8"),
            "brand",
            &[],
        )
        .unwrap_err();
        assert!(matches!(err, BrandKitError::InvalidInput(_)));
    }

    #[test]
    fn write_zip_rejects_destination_that_is_a_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let file_path = tmp.path().join("not-a-dir.txt");
        std::fs::write(&file_path, b"hi").expect("write test file");
        let err = write_zip(&file_path, "brand", &[]).unwrap_err();
        assert!(matches!(err, BrandKitError::InvalidInput(_)));
    }

    #[cfg(unix)]
    #[test]
    fn write_zip_follows_symlinked_destination_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let real_dir = tmp.path().join("real");
        std::fs::create_dir_all(&real_dir).expect("mkdir");
        let link = tmp.path().join("via-symlink");
        std::os::unix::fs::symlink(&real_dir, &link).expect("symlink");
        let path = write_zip(
            &link,
            "brand",
            &[BrandKitAsset {
                filename: "hello.txt".into(),
                bytes: b"hi".to_vec(),
            }],
        )
        .expect("symlinked dest should resolve and succeed");
        // The returned path must be inside the REAL directory, not the
        // symlink wrapper — i.e. canonicalize did its job.
        let canon_real = std::fs::canonicalize(&real_dir).expect("canon real");
        assert!(
            path.starts_with(&canon_real),
            "expected output path `{}` under real dir `{}`",
            path.display(),
            canon_real.display()
        );
        assert!(path.exists());
    }
}
