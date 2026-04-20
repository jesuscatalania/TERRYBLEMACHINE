//! Playwright-backed analyzer. Spawns `node scripts/url_analyzer.mjs <url>`
//! and parses its single-line JSON output.
//!
//! Callers construct a [`PlaywrightUrlAnalyzer`] with the absolute path of
//! the script (usually resolved from the app's working directory). The node
//! binary name defaults to `node` but can be overridden for sandboxed hosts.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::timeout;

use super::types::{AnalysisResult, AnalyzerError, UrlAnalyzer};

pub struct PlaywrightUrlAnalyzer {
    node_binary: String,
    script_path: PathBuf,
    timeout: Duration,
}

impl PlaywrightUrlAnalyzer {
    pub fn new(script_path: PathBuf) -> Self {
        Self {
            node_binary: "node".to_string(),
            script_path,
            timeout: Duration::from_secs(45),
        }
    }

    pub fn with_node(mut self, node_binary: impl Into<String>) -> Self {
        self.node_binary = node_binary.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn script_path(&self) -> &Path {
        &self.script_path
    }
}

fn validate_url(url: &str) -> Result<(), AnalyzerError> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AnalyzerError::InvalidUrl(format!(
            "must start with http:// or https://: {url}"
        )));
    }
    Ok(())
}

#[async_trait]
impl UrlAnalyzer for PlaywrightUrlAnalyzer {
    async fn analyze(
        &self,
        url: &str,
        screenshot_path: Option<&Path>,
        assets_dir: Option<&Path>,
    ) -> Result<AnalysisResult, AnalyzerError> {
        validate_url(url)?;

        let mut cmd = Command::new(&self.node_binary);
        cmd.arg(&self.script_path).arg(url);
        if let Some(path) = screenshot_path {
            cmd.arg(format!("--screenshot={}", path.display()));
        }
        if let Some(dir) = assets_dir {
            cmd.arg(format!("--assets-dir={}", dir.display()));
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| AnalyzerError::Spawn(e.to_string()))?;

        // Read stdout AND stderr concurrently — previously we only read
        // stdout, so any sidecar crash (missing Playwright browsers, node
        // syntax error, unhandled rejection) produced an opaque "exit 1"
        // with zero diagnostics in the toast. Read both, surface both.
        let mut stdout_pipe = child.stdout.take();
        let mut stderr_pipe = child.stderr.take();
        let run = async {
            let mut out = String::new();
            let mut err = String::new();
            if let Some(mut s) = stdout_pipe.take() {
                s.read_to_string(&mut out).await?;
            }
            if let Some(mut e) = stderr_pipe.take() {
                e.read_to_string(&mut err).await?;
            }
            let status = child.wait().await?;
            Ok::<_, std::io::Error>((status, out, err))
        };

        let (status, out, err) = match timeout(self.timeout, run).await {
            Ok(res) => res?,
            Err(_) => {
                let _ = child.start_kill();
                return Err(AnalyzerError::Sidecar("timed out".into()));
            }
        };

        if !status.success() {
            // Even on failure the script writes a JSON error object — keep
            // the message when present. Otherwise fall back to the first
            // meaningful line of stderr (Playwright throws loud stack
            // traces) and finally the bare exit code.
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(out.trim()) {
                if let Some(msg) = value.get("error").and_then(|v| v.as_str()) {
                    return Err(AnalyzerError::Sidecar(msg.to_string()));
                }
            }
            let stderr_trimmed = err.trim();
            if !stderr_trimmed.is_empty() {
                // Truncate to first 600 chars — stack traces can be long.
                let preview: String = stderr_trimmed.chars().take(600).collect();
                return Err(AnalyzerError::Sidecar(format!(
                    "exit {}: {preview}",
                    status.code().unwrap_or(-1)
                )));
            }
            return Err(AnalyzerError::Sidecar(format!(
                "exit {} (no stderr output)",
                status.code().unwrap_or(-1)
            )));
        }

        parse_sidecar_output(&out)
    }
}

pub(crate) fn parse_sidecar_output(raw: &str) -> Result<AnalysisResult, AnalyzerError> {
    // The script writes one JSON line — but it may have a trailing newline.
    let line = raw.trim();
    if line.is_empty() {
        return Err(AnalyzerError::ParseOutput("empty stdout".into()));
    }
    serde_json::from_str::<AnalysisResult>(line)
        .map_err(|e| AnalyzerError::ParseOutput(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_url_rejects_non_http() {
        assert!(matches!(
            validate_url("file:///etc/passwd"),
            Err(AnalyzerError::InvalidUrl(_))
        ));
        assert!(matches!(
            validate_url("javascript:alert(1)"),
            Err(AnalyzerError::InvalidUrl(_))
        ));
    }

    #[test]
    fn validate_url_accepts_http_and_https() {
        assert!(validate_url("http://example.com").is_ok());
        assert!(validate_url("https://example.com").is_ok());
    }

    #[test]
    fn parse_sidecar_output_accepts_happy_json() {
        let raw = r#"{"url":"https://x","status":200,"title":"X","colors":[],"fonts":[],"spacing":[],"customProperties":{},"layout":"flex"}"#;
        let r = parse_sidecar_output(raw).unwrap();
        assert_eq!(r.url, "https://x");
        assert_eq!(r.layout, "flex");
        // `assets` defaults to empty when the sidecar omitted it (backward compat).
        assert!(r.assets.is_empty());
    }

    #[test]
    fn parse_sidecar_output_reads_assets_array() {
        let raw = r#"{"url":"https://x","status":200,"title":"X","colors":[],"fonts":[],"spacing":[],"customProperties":{},"layout":"flex","assets":[{"url":"https://x/a.png","saved_as":"x_a.png"}]}"#;
        let r = parse_sidecar_output(raw).unwrap();
        assert_eq!(r.assets.len(), 1);
        assert_eq!(r.assets[0].url, "https://x/a.png");
        assert_eq!(r.assets[0].saved_as, "x_a.png");
    }

    #[test]
    fn parse_sidecar_output_errors_on_empty() {
        assert!(matches!(
            parse_sidecar_output(""),
            Err(AnalyzerError::ParseOutput(_))
        ));
    }

    #[test]
    fn parse_sidecar_output_errors_on_bad_json() {
        assert!(matches!(
            parse_sidecar_output("{not json"),
            Err(AnalyzerError::ParseOutput(_))
        ));
    }
}
