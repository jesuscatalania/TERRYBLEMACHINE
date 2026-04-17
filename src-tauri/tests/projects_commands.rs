//! Integration tests for the Tauri project commands.
//!
//! Tauri `#[tauri::command]` functions require a `tauri::State<'_, T>`
//! wrapper that cannot be constructed outside a live Tauri runtime. To keep
//! this crate-level test lightweight, we exercise the exact same logic the
//! commands delegate to: `ProjectStoreState::store()` returns a fresh
//! `FileProjectStore`, and every command is a one-liner that calls into it.
//!
//! Closes the test gap from Schritt 1.4: storage.rs was covered, but the
//! command wrapper and its path wiring had no integration coverage.

use std::fs;

use tempfile::TempDir;
use terryblemachine_lib::projects::{
    commands::ProjectStoreState, FileProjectStore, NewProject, Project,
};

fn state_with_root() -> (ProjectStoreState, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let state = ProjectStoreState::new(dir.path().to_path_buf());
    (state, dir)
}

#[test]
fn create_generates_id_and_path() {
    let (state, tmp) = state_with_root();

    let project: Project = state
        .store()
        .create(NewProject {
            name: "My Website".into(),
            module: "website".into(),
            description: Some("integration test".into()),
        })
        .expect("create_project");

    assert_eq!(project.id, "my-website");
    assert_eq!(project.name, "My Website");
    assert_eq!(project.module, "website");
    assert_eq!(project.description.as_deref(), Some("integration test"));
    assert!(!project.created_at.is_empty());

    let on_disk = tmp.path().join("my-website");
    assert!(on_disk.is_dir(), "project dir should exist");
    assert!(on_disk.join("project.json").is_file());
    assert!(on_disk.join("assets").is_dir());
    assert_eq!(project.path, on_disk.to_string_lossy());
}

#[test]
fn open_reads_the_created_project() {
    let (state, _tmp) = state_with_root();
    let created = state
        .store()
        .create(NewProject {
            name: "Opener".into(),
            module: "graphic2d".into(),
            description: None,
        })
        .unwrap();

    // open via a freshly obtained store — simulates a second IPC call.
    let loaded = state.store().open(&created.id).expect("open_project");
    assert_eq!(loaded, created);
}

#[test]
fn list_returns_newest_first() {
    let (state, _tmp) = state_with_root();

    let first = state
        .store()
        .create(NewProject {
            name: "Alpha".into(),
            module: "website".into(),
            description: None,
        })
        .unwrap();
    // Ensure distinct timestamps; rfc3339 sorts lexicographically.
    std::thread::sleep(std::time::Duration::from_millis(5));
    let second = state
        .store()
        .create(NewProject {
            name: "Beta".into(),
            module: "video".into(),
            description: None,
        })
        .unwrap();

    let list = state.store().list().expect("list_projects");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, second.id);
    assert_eq!(list[1].id, first.id);
}

#[test]
fn delete_removes_the_project_folder() {
    let (state, tmp) = state_with_root();
    let created = state
        .store()
        .create(NewProject {
            name: "Trash Me".into(),
            module: "typography".into(),
            description: None,
        })
        .unwrap();

    let project_dir = tmp.path().join(&created.id);
    assert!(project_dir.exists());

    state.store().delete(&created.id).expect("delete_project");
    assert!(!project_dir.exists());

    // second delete is a no-op (delete is idempotent: missing id returns Ok).
    state
        .store()
        .delete(&created.id)
        .expect("delete is idempotent");
}

#[test]
fn list_empty_root_returns_empty_vec() {
    let dir = TempDir::new().unwrap();
    // Point the store at a root that does not yet exist — mirrors a first-run
    // app whose documents dir has no TERRYBLEMACHINE/projects/ yet.
    let missing = dir.path().join("never-created");
    let state = ProjectStoreState::new(missing.clone());

    let list = state.store().list().expect("list_projects on missing root");
    assert!(list.is_empty());
    // list() must not materialize the directory.
    assert!(!missing.exists());
}

#[test]
fn state_store_yields_root_aware_file_store() {
    // The command-layer wrapper must hand out a store rooted at the exact
    // path it was constructed with; this guards against future refactors
    // that might accidentally reinterpret the root (e.g. via default_root).
    let dir = TempDir::new().unwrap();
    let state = ProjectStoreState::new(dir.path().to_path_buf());

    let store: FileProjectStore = state.store();
    assert_eq!(store.root(), dir.path());

    // Writes via the wrapper land in the expected place.
    state
        .store()
        .create(NewProject {
            name: "Roots".into(),
            module: "website".into(),
            description: None,
        })
        .unwrap();
    assert!(fs::metadata(dir.path().join("roots").join("project.json")).is_ok());
}

#[test]
fn state_exposes_projects_root_as_string() {
    // The `projects_root` #[tauri::command] body is just
    // `state.root.to_string_lossy().into_owned()`. We can't construct a
    // real `tauri::State` outside the Tauri runtime, but we can exercise
    // the same code path directly via `state.root` — this pins the
    // contract that projects_root returns the constructor-provided root
    // verbatim. Closes FU #96.
    let dir = TempDir::new().unwrap();
    let state = ProjectStoreState::new(dir.path().to_path_buf());

    // Mirror the command body exactly.
    let root_str = state.root.to_string_lossy().into_owned();
    assert_eq!(root_str, dir.path().to_string_lossy());
    // And the returned string points at a real directory.
    assert!(std::path::Path::new(&root_str).is_dir());
}
