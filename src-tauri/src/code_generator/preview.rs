//! `open_project_in_browser` — writes the current [`GeneratedProject`] to a
//! fresh temporary directory and opens its `index.html` in the system
//! default browser via `tauri-plugin-opener`.
//!
//! Unlike the in-app `DevicePreview` (iframe) or `SandpackPreview` (in-app
//! bundler), this command lets the user inspect the generated project in
//! a real browser with full DevTools. The temp directory is left on disk
//! — the browser still holds an open handle to the file — and is GC'd by
//! the OS the next time it sweeps `$TMPDIR`.

use std::path::PathBuf;

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::code_generator::types::GeneratedProject;

#[tauri::command]
pub async fn open_project_in_browser(
    project: GeneratedProject,
    app: AppHandle,
) -> Result<String, String> {
    let dir = write_project_to_tempdir(&project).map_err(|e| e.to_string())?;

    let entry = dir.join("index.html");
    if !entry.exists() {
        return Err(format!(
            "no index.html in project ({} files)",
            project.files.len()
        ));
    }

    let url = format!("file://{}", entry.display());
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| format!("open browser: {e}"))?;
    Ok(url)
}

/// Pure helper — writes a project out to a unique tempdir and returns the
/// path. Split out so tests can exercise the filesystem side of the command
/// without needing a `tauri::AppHandle`.
pub(crate) fn write_project_to_tempdir(
    project: &GeneratedProject,
) -> std::io::Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!(
        "tm-preview-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&dir)?;

    for file in &project.files {
        // Strip any leading `/` so the join stays inside `dir`.
        let rel = file
            .path
            .to_string_lossy()
            .trim_start_matches('/')
            .to_string();
        let path: PathBuf = dir.join(&rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &file.content)?;
    }

    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_generator::types::GeneratedFile;
    use std::path::PathBuf;

    fn project(files: Vec<GeneratedFile>) -> GeneratedProject {
        GeneratedProject {
            summary: "test".into(),
            files,
            prompt: "test prompt".into(),
        }
    }

    #[test]
    fn writes_all_files_to_tempdir() {
        let p = project(vec![
            GeneratedFile {
                path: PathBuf::from("index.html"),
                content: "<h1>Hi</h1>".into(),
            },
            GeneratedFile {
                path: PathBuf::from("src/app.js"),
                content: "console.log('ok')".into(),
            },
        ]);

        let dir = write_project_to_tempdir(&p).expect("write succeeds");

        assert!(dir.exists(), "tempdir created");
        let html = std::fs::read_to_string(dir.join("index.html")).unwrap();
        assert_eq!(html, "<h1>Hi</h1>");
        let js = std::fs::read_to_string(dir.join("src/app.js")).unwrap();
        assert!(js.contains("console.log"));

        // Clean up behind ourselves; tempdir names include a UUID so if
        // this fails it won't affect other test runs.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn strips_leading_slash_from_paths() {
        let p = project(vec![GeneratedFile {
            path: PathBuf::from("/index.html"),
            content: "ok".into(),
        }]);
        let dir = write_project_to_tempdir(&p).expect("write succeeds");
        assert!(dir.join("index.html").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn creates_nested_parent_directories() {
        let p = project(vec![GeneratedFile {
            path: PathBuf::from("a/b/c/deep.txt"),
            content: "deep".into(),
        }]);
        let dir = write_project_to_tempdir(&p).expect("write succeeds");
        assert!(dir.join("a/b/c/deep.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn each_call_gets_a_unique_dir() {
        let p = project(vec![GeneratedFile {
            path: PathBuf::from("index.html"),
            content: "x".into(),
        }]);
        let a = write_project_to_tempdir(&p).unwrap();
        let b = write_project_to_tempdir(&p).unwrap();
        assert_ne!(a, b, "tempdirs must differ between calls");
        let _ = std::fs::remove_dir_all(&a);
        let _ = std::fs::remove_dir_all(&b);
    }
}
