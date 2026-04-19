//! Locate the user-installed `claude` CLI binary. We do NOT bundle it —
//! the user runs `brew install anthropic/claude-code/claude` (or similar)
//! and `claude login` once. This module just finds whatever path it lives at.

use std::path::PathBuf;

/// Candidate paths checked in order. `which claude` first (respects user PATH),
/// then known install locations on macOS.
pub fn default_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    // 1. PATH lookup via `which` — macOS-only call, falls back gracefully
    if let Ok(out_bytes) = std::process::Command::new("which").arg("claude").output() {
        if out_bytes.status.success() {
            let path = String::from_utf8_lossy(&out_bytes.stdout)
                .trim()
                .to_string();
            if !path.is_empty() {
                out.push(PathBuf::from(path));
            }
        }
    }
    // 2. Known installs
    if let Some(home) = std::env::var_os("HOME") {
        let h = PathBuf::from(home);
        out.push(h.join(".claude/local/claude"));
        out.push(h.join(".local/bin/claude"));
    }
    out.push(PathBuf::from("/opt/homebrew/bin/claude"));
    out.push(PathBuf::from("/usr/local/bin/claude"));
    out
}

/// Return the first candidate path that exists on disk.
pub fn detect_first_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|p| p.exists()).cloned()
}

/// One-shot detection — returns Some(path) if any candidate exists, None otherwise.
pub fn detect_claude_binary() -> Option<PathBuf> {
    detect_first_existing(&default_candidates())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detect_returns_some_when_path_exists() {
        let exe = std::env::current_exe().unwrap();
        let candidates = vec![exe.clone()];
        assert_eq!(detect_first_existing(&candidates), Some(exe));
    }

    #[test]
    fn detect_returns_none_for_all_missing() {
        let candidates = vec![
            PathBuf::from("/definitely/missing/claude"),
            PathBuf::from("/also/missing/claude"),
        ];
        assert_eq!(detect_first_existing(&candidates), None);
    }
}
