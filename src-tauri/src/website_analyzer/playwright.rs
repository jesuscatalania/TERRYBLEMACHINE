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
    ) -> Result<AnalysisResult, AnalyzerError> {
        validate_url(url)?;

        let mut cmd = Command::new(&self.node_binary);
        cmd.arg(&self.script_path).arg(url);
        if let Some(path) = screenshot_path {
            cmd.arg(format!("--screenshot={}", path.display()));
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| AnalyzerError::Spawn(e.to_string()))?;

        let run = async {
            let mut out = String::new();
            if let Some(mut stdout) = child.stdout.take() {
                stdout.read_to_string(&mut out).await?;
            }
            let status = child.wait().await?;
            Ok::<_, std::io::Error>((status, out))
        };

        let (status, out) = match timeout(self.timeout, run).await {
            Ok(res) => res?,
            Err(_) => {
                let _ = child.start_kill();
                return Err(AnalyzerError::Sidecar("timed out".into()));
            }
        };

        if !status.success() {
            // Even on failure the script writes a JSON error object — keep
            // the message when present, otherwise surface the exit code.
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(out.trim()) {
                if let Some(msg) = value.get("error").and_then(|v| v.as_str()) {
                    return Err(AnalyzerError::Sidecar(msg.to_string()));
                }
            }
            return Err(AnalyzerError::Sidecar(format!(
                "exit {}",
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
