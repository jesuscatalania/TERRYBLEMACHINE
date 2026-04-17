//! ZIP-based exporter. Writes a [`GeneratedProject`] to disk as a single
//! `.zip` archive.
//!
//! Three output formats are supported:
//! - [`ExportFormat::Raw`] — files exactly as the generator produced them.
//! - [`ExportFormat::React`] — the generator output is placed under `src/`
//!   with a minimal `package.json` + Vite config at the root.
//! - [`ExportFormat::NextJs`] — generator output is placed under `app/`,
//!   with a minimal Next.js `package.json` + `next.config.mjs`.
//!
//! The more opinionated bundles (Vercel/Netlify deploy configs) will layer
//! on top in follow-up steps.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::code_generator::{GeneratedFile, GeneratedProject};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExportFormat {
    #[default]
    Raw,
    React,
    NextJs,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExportRequest {
    pub project: GeneratedProject,
    #[serde(default)]
    pub format: ExportFormat,
    /// Directory to write the `.zip` into. The filename is derived from the
    /// first index.html / first file path.
    pub destination: PathBuf,
}

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
}

/// Write a project's files to a ZIP archive at
/// `<destination>/<slug>.zip`. Returns the final path.
pub fn export_to_zip(req: &ExportRequest) -> Result<PathBuf, ExportError> {
    if req.project.files.is_empty() {
        return Err(ExportError::InvalidRequest("project has no files".into()));
    }
    std::fs::create_dir_all(&req.destination)?;

    let slug = derive_slug(&req.project);
    let path = req.destination.join(format!("{slug}.zip"));
    let file = File::create(&path)?;
    let mut zip = ZipWriter::new(file);

    let options: SimpleFileOptions = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for file in files_for_format(&req.project.files, req.format) {
        write_entry(&mut zip, &file.path, &file.content, options)?;
    }

    // Per-format scaffolding.
    match req.format {
        ExportFormat::Raw => {}
        ExportFormat::React => {
            for (path, content) in react_scaffold() {
                write_entry(&mut zip, Path::new(path), content, options)?;
            }
        }
        ExportFormat::NextJs => {
            for (path, content) in nextjs_scaffold() {
                write_entry(&mut zip, Path::new(path), content, options)?;
            }
        }
    }

    zip.finish()?;
    Ok(path)
}

fn files_for_format(files: &[GeneratedFile], format: ExportFormat) -> Vec<GeneratedFile> {
    match format {
        ExportFormat::Raw => files.to_vec(),
        ExportFormat::React => files
            .iter()
            .map(|f| GeneratedFile {
                path: Path::new("src").join(&f.path),
                content: f.content.clone(),
            })
            .collect(),
        ExportFormat::NextJs => files
            .iter()
            .map(|f| GeneratedFile {
                path: Path::new("app").join(&f.path),
                content: f.content.clone(),
            })
            .collect(),
    }
}

fn write_entry(
    zip: &mut ZipWriter<File>,
    path: &Path,
    content: &str,
    options: SimpleFileOptions,
) -> Result<(), ExportError> {
    let name = path.to_string_lossy().replace('\\', "/");
    zip.start_file(name, options)?;
    zip.write_all(content.as_bytes())?;
    Ok(())
}

fn derive_slug(project: &GeneratedProject) -> String {
    let base = project
        .files
        .iter()
        .find_map(|f| f.path.file_stem().and_then(|s| s.to_str()))
        .unwrap_or("project");
    slug(base)
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut prev_hyphen = false;
    for c in s.chars() {
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
        "project".into()
    } else {
        trimmed
    }
}

// ─── Scaffolds ───────────────────────────────────────────────────────────

fn react_scaffold() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "package.json",
            r#"{
  "name": "terryblemachine-export",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.3.0",
    "vite": "^6.0.0"
  }
}
"#,
        ),
        (
            "vite.config.js",
            "import react from '@vitejs/plugin-react';\nimport { defineConfig } from 'vite';\n\nexport default defineConfig({ plugins: [react()] });\n",
        ),
        (
            "README.md",
            "# Exported from TERRYBLEMACHINE\n\n```sh\npnpm install\npnpm dev\n```\n",
        ),
    ]
}

fn nextjs_scaffold() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "package.json",
            r#"{
  "name": "terryblemachine-export",
  "private": true,
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start"
  },
  "dependencies": {
    "next": "^15.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  }
}
"#,
        ),
        (
            "next.config.mjs",
            "/** @type {import('next').NextConfig} */\nconst nextConfig = {};\nexport default nextConfig;\n",
        ),
        (
            "README.md",
            "# Exported from TERRYBLEMACHINE (Next.js)\n\n```sh\npnpm install\npnpm dev\n```\n",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};
    use tempfile::TempDir;

    fn sample_project() -> GeneratedProject {
        GeneratedProject {
            summary: "demo".into(),
            prompt: "demo".into(),
            files: vec![
                GeneratedFile {
                    path: PathBuf::from("index.html"),
                    content: "<h1>Hi</h1>".into(),
                },
                GeneratedFile {
                    path: PathBuf::from("styles.css"),
                    content: "body { margin: 0; }".into(),
                },
            ],
        }
    }

    fn entries(path: &Path) -> Vec<String> {
        let bytes = std::fs::read(path).unwrap();
        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader).unwrap();
        (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect()
    }

    fn read_entry(path: &Path, entry: &str) -> String {
        let bytes = std::fs::read(path).unwrap();
        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader).unwrap();
        let mut file = archive.by_name(entry).unwrap();
        let mut out = String::new();
        file.read_to_string(&mut out).unwrap();
        out
    }

    #[test]
    fn raw_export_writes_every_file_as_is() {
        let tmp = TempDir::new().unwrap();
        let req = ExportRequest {
            project: sample_project(),
            format: ExportFormat::Raw,
            destination: tmp.path().to_path_buf(),
        };
        let path = export_to_zip(&req).unwrap();
        assert!(path.exists());
        let files = entries(&path);
        assert!(files.contains(&"index.html".to_string()));
        assert!(files.contains(&"styles.css".to_string()));
        assert_eq!(read_entry(&path, "index.html"), "<h1>Hi</h1>");
    }

    #[test]
    fn react_export_places_files_under_src_plus_scaffold() {
        let tmp = TempDir::new().unwrap();
        let req = ExportRequest {
            project: sample_project(),
            format: ExportFormat::React,
            destination: tmp.path().to_path_buf(),
        };
        let path = export_to_zip(&req).unwrap();
        let files = entries(&path);
        assert!(files.contains(&"src/index.html".to_string()));
        assert!(files.contains(&"package.json".to_string()));
        assert!(files.contains(&"vite.config.js".to_string()));
        let pkg = read_entry(&path, "package.json");
        assert!(pkg.contains("react"));
    }

    #[test]
    fn nextjs_export_places_files_under_app_plus_scaffold() {
        let tmp = TempDir::new().unwrap();
        let req = ExportRequest {
            project: sample_project(),
            format: ExportFormat::NextJs,
            destination: tmp.path().to_path_buf(),
        };
        let path = export_to_zip(&req).unwrap();
        let files = entries(&path);
        assert!(files.contains(&"app/index.html".to_string()));
        assert!(files.contains(&"next.config.mjs".to_string()));
    }

    #[test]
    fn empty_project_is_invalid() {
        let tmp = TempDir::new().unwrap();
        let req = ExportRequest {
            project: GeneratedProject {
                summary: "x".into(),
                prompt: "x".into(),
                files: vec![],
            },
            format: ExportFormat::Raw,
            destination: tmp.path().to_path_buf(),
        };
        let err = export_to_zip(&req).unwrap_err();
        assert!(matches!(err, ExportError::InvalidRequest(_)));
    }

    #[test]
    fn destination_is_created_if_missing() {
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("missing").join("nested");
        let req = ExportRequest {
            project: sample_project(),
            format: ExportFormat::Raw,
            destination: dest.clone(),
        };
        let path = export_to_zip(&req).unwrap();
        assert!(path.starts_with(&dest));
    }

    #[test]
    fn slug_normalises_special_chars() {
        assert_eq!(slug("Hello World"), "hello-world");
        assert_eq!(slug("!!!"), "project");
        assert_eq!(slug("alpha"), "alpha");
    }
}
