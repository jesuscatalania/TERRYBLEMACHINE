//! Thin wrapper around the `notify` crate that turns filesystem events into
//! a coarse `WatchEvent` stream. Consumers poll `next_event` and call
//! [`crate::taste_engine::TasteEngine::refresh`] whenever something relevant
//! changes.

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::event::{EventKind, ModifyKind};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use super::errors::TasteError;

/// Coarse-grained signal emitted on any interesting change inside
/// `meingeschmack/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// Files created / modified / renamed — refresh the profile.
    Changed(Vec<PathBuf>),
    /// Files removed — refresh the profile.
    Removed(Vec<PathBuf>),
}

/// Live filesystem watcher. Dropping it stops observation.
pub struct TasteWatcher {
    root: PathBuf,
    rx: mpsc::UnboundedReceiver<WatchEvent>,
    _watcher: RecommendedWatcher,
}

impl TasteWatcher {
    pub fn new(root: PathBuf) -> Result<Self, TasteError> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let Ok(event) = res else { return };
            let interesting = match event.kind {
                EventKind::Create(_) => WatchEvent::Changed(event.paths.clone()),
                EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Name(_)) => {
                    WatchEvent::Changed(event.paths.clone())
                }
                EventKind::Remove(_) => WatchEvent::Removed(event.paths.clone()),
                _ => return,
            };
            let _ = tx.send(interesting);
        })
        .map_err(|e| TasteError::Watcher(e.to_string()))?;

        if root.exists() {
            watcher
                .watch(&root, RecursiveMode::Recursive)
                .map_err(|e| TasteError::Watcher(e.to_string()))?;
        }

        Ok(Self {
            root,
            rx,
            _watcher: watcher,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Await the next coalesced event. Returns `None` if the sender side
    /// has been dropped (shouldn't happen while `self` is alive).
    pub async fn next_event(&mut self) -> Option<WatchEvent> {
        self.rx.recv().await
    }

    /// Drain any buffered events non-blockingly. Useful in tests and for
    /// coalescing multiple FS events into a single engine refresh.
    pub fn try_drain(&mut self) -> Vec<WatchEvent> {
        let mut out = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            out.push(event);
        }
        out
    }
}

/// Default poll interval for the coalescing loop (not used internally but
/// exposed for consumers that want a sensible default).
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(500);

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn watcher_builds_on_existing_dir() {
        let tmp = TempDir::new().unwrap();
        let watcher = TasteWatcher::new(tmp.path().to_path_buf()).unwrap();
        assert_eq!(watcher.root(), tmp.path());
    }

    #[tokio::test]
    async fn watcher_builds_when_dir_does_not_exist_yet() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("missing");
        let watcher = TasteWatcher::new(missing.clone()).unwrap();
        assert_eq!(watcher.root(), missing);
    }

    #[tokio::test]
    async fn try_drain_is_empty_on_idle_watcher() {
        let tmp = TempDir::new().unwrap();
        let mut watcher = TasteWatcher::new(tmp.path().to_path_buf()).unwrap();
        let drained = watcher.try_drain();
        assert!(drained.is_empty());
    }
}
