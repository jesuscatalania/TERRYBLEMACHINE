use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::errors::ProjectError;

/// Persisted project descriptor. Serialized as `project.json` inside each
/// project folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub module: String,
    pub path: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Input shape for creating a new project.
#[derive(Debug, Clone, Deserialize)]
pub struct NewProject {
    pub name: String,
    pub module: String,
    #[serde(default)]
    pub description: Option<String>,
}

const METADATA_FILE: &str = "project.json";

/// Filesystem-backed project store rooted at an arbitrary directory.
pub struct FileProjectStore {
    root: PathBuf,
}

impl FileProjectStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Create a new project folder + metadata. Returns the persisted [`Project`].
    ///
    /// The slug derived from the name is used as the folder name. If a folder
    /// already exists with that slug, numeric suffixes are appended until the
    /// name is unique.
    pub fn create(&self, input: NewProject) -> Result<Project, ProjectError> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(ProjectError::InvalidName("empty".into()));
        }
        if input.module.trim().is_empty() {
            return Err(ProjectError::InvalidName(format!(
                "empty module for project `{name}`"
            )));
        }

        fs::create_dir_all(&self.root)?;

        let base_slug = slugify(name);
        if base_slug.is_empty() {
            return Err(ProjectError::InvalidName(format!(
                "cannot derive a slug from `{name}`"
            )));
        }
        let id = unique_slug(&self.root, &base_slug);

        let project_dir = self.root.join(&id);
        fs::create_dir_all(&project_dir)?;
        fs::create_dir_all(project_dir.join("assets"))?;

        let project = Project {
            id: id.clone(),
            name: name.to_owned(),
            module: input.module,
            path: project_dir.to_string_lossy().into_owned(),
            created_at: Utc::now().to_rfc3339(),
            description: input.description.filter(|s| !s.trim().is_empty()),
        };

        let metadata = project_dir.join(METADATA_FILE);
        fs::write(&metadata, serde_json::to_vec_pretty(&project)?)?;

        Ok(project)
    }

    /// Read and return a project by its id (slug).
    pub fn open(&self, id: &str) -> Result<Project, ProjectError> {
        let metadata = self.root.join(id).join(METADATA_FILE);
        if !metadata.exists() {
            return Err(ProjectError::NotFound(id.to_owned()));
        }
        let bytes = fs::read(&metadata)?;
        let project: Project = serde_json::from_slice(&bytes)?;
        Ok(project)
    }

    /// Delete a project folder. Missing projects are a no-op.
    pub fn delete(&self, id: &str) -> Result<(), ProjectError> {
        let project_dir = self.root.join(id);
        if !project_dir.exists() {
            return Ok(());
        }
        fs::remove_dir_all(&project_dir)?;
        Ok(())
    }

    /// List every valid project under the root. Sorted newest-first by `created_at`.
    pub fn list(&self) -> Result<Vec<Project>, ProjectError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut out = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let metadata = entry.path().join(METADATA_FILE);
            if !metadata.exists() {
                continue;
            }
            match fs::read(&metadata).and_then(|b| {
                serde_json::from_slice::<Project>(&b).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })
            }) {
                Ok(project) => out.push(project),
                Err(_) => continue, // skip malformed entries silently
            }
        }

        // Sort newest-first: strings in rfc3339 format are lexicographically sortable.
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(out)
    }
}

fn slugify(name: &str) -> String {
    let lower: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    lower
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn unique_slug(root: &Path, base: &str) -> String {
    if !root.join(base).exists() {
        return base.to_owned();
    }
    for n in 2..10_000 {
        let candidate = format!("{base}-{n}");
        if !root.join(&candidate).exists() {
            return candidate;
        }
    }
    format!("{base}-{}", Utc::now().timestamp_millis(),)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store() -> (FileProjectStore, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let store = FileProjectStore::new(dir.path().to_path_buf());
        (store, dir)
    }

    #[test]
    fn slugify_normalizes_input() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("  mix of SPACES  "), "mix-of-spaces");
        assert_eq!(slugify("Awesome!! Project 01"), "awesome-project-01");
        assert_eq!(slugify("_Leading_Trailing_"), "leading-trailing");
    }

    #[test]
    fn slugify_rejects_only_symbols() {
        assert_eq!(slugify("!!!"), "");
    }

    #[test]
    fn create_writes_folder_and_metadata() {
        let (store, _tmp) = store();
        let project = store
            .create(NewProject {
                name: "My Video".into(),
                module: "video".into(),
                description: Some("A test project".into()),
            })
            .unwrap();

        assert_eq!(project.id, "my-video");
        assert_eq!(project.name, "My Video");
        assert_eq!(project.module, "video");
        assert_eq!(project.description.as_deref(), Some("A test project"));
        assert!(!project.created_at.is_empty());

        let project_dir = store.root().join("my-video");
        assert!(project_dir.is_dir());
        assert!(project_dir.join("project.json").is_file());
        assert!(project_dir.join("assets").is_dir());

        let on_disk: Project =
            serde_json::from_slice(&fs::read(project_dir.join("project.json")).unwrap()).unwrap();
        assert_eq!(on_disk, project);
    }

    #[test]
    fn create_rejects_empty_name() {
        let (store, _tmp) = store();
        let err = store
            .create(NewProject {
                name: "  ".into(),
                module: "website".into(),
                description: None,
            })
            .unwrap_err();
        assert!(matches!(err, ProjectError::InvalidName(_)));
    }

    #[test]
    fn create_rejects_non_sluggable_name() {
        let (store, _tmp) = store();
        let err = store
            .create(NewProject {
                name: "!!!".into(),
                module: "website".into(),
                description: None,
            })
            .unwrap_err();
        assert!(matches!(err, ProjectError::InvalidName(_)));
    }

    #[test]
    fn create_generates_unique_slugs_on_collision() {
        let (store, _tmp) = store();
        let first = store
            .create(NewProject {
                name: "Demo".into(),
                module: "website".into(),
                description: None,
            })
            .unwrap();
        let second = store
            .create(NewProject {
                name: "Demo".into(),
                module: "website".into(),
                description: None,
            })
            .unwrap();
        assert_eq!(first.id, "demo");
        assert_eq!(second.id, "demo-2");
    }

    #[test]
    fn open_reads_metadata() {
        let (store, _tmp) = store();
        let created = store
            .create(NewProject {
                name: "Open Me".into(),
                module: "graphic2d".into(),
                description: None,
            })
            .unwrap();
        let loaded = store.open(&created.id).unwrap();
        assert_eq!(loaded, created);
    }

    #[test]
    fn open_missing_is_not_found() {
        let (store, _tmp) = store();
        let err = store.open("ghost").unwrap_err();
        assert!(matches!(err, ProjectError::NotFound(id) if id == "ghost"));
    }

    #[test]
    fn delete_removes_folder_and_is_idempotent() {
        let (store, _tmp) = store();
        let p = store
            .create(NewProject {
                name: "Trash".into(),
                module: "typography".into(),
                description: None,
            })
            .unwrap();
        assert!(store.root().join(&p.id).exists());
        store.delete(&p.id).unwrap();
        assert!(!store.root().join(&p.id).exists());
        // second delete is a no-op
        store.delete(&p.id).unwrap();
    }

    #[test]
    fn list_returns_newest_first() {
        let (store, _tmp) = store();
        let a = store
            .create(NewProject {
                name: "Alpha".into(),
                module: "website".into(),
                description: None,
            })
            .unwrap();
        // Ensure distinct timestamps
        std::thread::sleep(std::time::Duration::from_millis(5));
        let b = store
            .create(NewProject {
                name: "Beta".into(),
                module: "website".into(),
                description: None,
            })
            .unwrap();
        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, b.id);
        assert_eq!(list[1].id, a.id);
    }

    #[test]
    fn list_empty_when_root_missing() {
        let dir = TempDir::new().unwrap();
        let store = FileProjectStore::new(dir.path().join("does-not-exist"));
        assert_eq!(store.list().unwrap(), Vec::new());
    }

    #[test]
    fn list_skips_non_dir_and_malformed() {
        let (store, tmp) = store();
        // valid project
        store
            .create(NewProject {
                name: "Valid".into(),
                module: "website".into(),
                description: None,
            })
            .unwrap();
        // stray file in root
        fs::write(tmp.path().join("README.txt"), b"ignore me").unwrap();
        // folder without project.json
        fs::create_dir(tmp.path().join("empty")).unwrap();
        // folder with malformed project.json
        let broken = tmp.path().join("broken");
        fs::create_dir(&broken).unwrap();
        fs::write(broken.join("project.json"), b"{not json").unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "valid");
    }
}
