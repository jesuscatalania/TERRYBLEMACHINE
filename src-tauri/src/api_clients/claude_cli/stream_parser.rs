//! Pure parser for `claude -p --output-format stream-json` output.
//!
//! Each line is a JSON object describing an event in the streamed response.
//! We're interested in:
//! - `assistant` events with `content[].type == "text"` → accumulate text
//! - `result` events with `subtype == "success"` → final cost + usage
//! - `result` events with `subtype` containing `error` → return Err

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct StreamResult {
    pub text: String,
    pub cost_usd: Option<f64>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub model: Option<String>,
}

#[derive(Debug, Default)]
pub struct StreamAccumulator {
    text: String,
    cost_usd: Option<f64>,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    model: Option<String>,
    error: Option<String>,
}

impl StreamAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one stream-json line (newline-stripped). Malformed lines are
    /// silently skipped — claude CLI occasionally emits debug noise.
    pub fn feed_line(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => return,
        };
        let event_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match event_type {
            "system" => {
                if let Some(model) = v.get("model").and_then(|m| m.as_str()) {
                    self.model = Some(model.to_string());
                }
            }
            "assistant" => {
                let blocks = v
                    .pointer("/message/content")
                    .and_then(|c| c.as_array())
                    .cloned()
                    .unwrap_or_default();
                for block in blocks {
                    if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                        if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                            self.text.push_str(t);
                        }
                    }
                }
            }
            "result" => {
                let subtype = v.get("subtype").and_then(|s| s.as_str()).unwrap_or("");
                if subtype.contains("error") {
                    let msg = v
                        .pointer("/error/message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown CLI error")
                        .to_string();
                    self.error = Some(msg);
                }
                if let Some(c) = v.get("total_cost_usd").and_then(|c| c.as_f64()) {
                    self.cost_usd = Some(c);
                }
                if let Some(usage) = v.get("usage") {
                    self.input_tokens = usage
                        .get("input_tokens")
                        .and_then(|t| t.as_u64())
                        .map(|t| t as u32);
                    self.output_tokens = usage
                        .get("output_tokens")
                        .and_then(|t| t.as_u64())
                        .map(|t| t as u32);
                }
            }
            _ => {}
        }
    }

    /// Finish parsing — returns Ok with accumulated content or Err on error event.
    pub fn finish(self) -> Result<StreamResult, String> {
        if let Some(err) = self.error {
            return Err(err);
        }
        Ok(StreamResult {
            text: self.text,
            cost_usd: self.cost_usd,
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            model: self.model,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"type":"system","subtype":"init","model":"claude-opus-4-7","session_id":"abc"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hello "}]}}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"world"}]}}
{"type":"result","subtype":"success","total_cost_usd":0.0012,"usage":{"input_tokens":5,"output_tokens":3}}
"#;

    #[test]
    fn accumulates_text_across_assistant_chunks() {
        let mut acc = StreamAccumulator::new();
        for line in SAMPLE.lines() {
            acc.feed_line(line);
        }
        let result = acc.finish().unwrap();
        assert_eq!(result.text, "Hello world");
        assert_eq!(result.input_tokens, Some(5));
        assert_eq!(result.output_tokens, Some(3));
        assert!((result.cost_usd.unwrap() - 0.0012).abs() < 1e-9);
    }

    #[test]
    fn ignores_tool_use_blocks_for_text_extraction() {
        let mut acc = StreamAccumulator::new();
        acc.feed_line(r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"X "},{"type":"tool_use","name":"Bash","input":{}},{"type":"text","text":"Y"}]}}"#);
        acc.feed_line(r#"{"type":"result","subtype":"success"}"#);
        let result = acc.finish().unwrap();
        assert_eq!(result.text, "X Y");
    }

    #[test]
    fn surfaces_error_subtype() {
        let mut acc = StreamAccumulator::new();
        acc.feed_line(r#"{"type":"result","subtype":"error_during_execution","error":{"message":"auth: token expired"}}"#);
        let err = acc.finish().unwrap_err();
        assert!(err.contains("auth: token expired"));
    }

    #[test]
    fn malformed_lines_are_skipped() {
        let mut acc = StreamAccumulator::new();
        acc.feed_line("not json");
        acc.feed_line(r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"ok"}]}}"#);
        acc.feed_line(r#"{"type":"result","subtype":"success"}"#);
        let result = acc.finish().unwrap();
        assert_eq!(result.text, "ok");
    }

    #[test]
    fn empty_stream_returns_empty_text() {
        let mut acc = StreamAccumulator::new();
        acc.feed_line(r#"{"type":"result","subtype":"success"}"#);
        let result = acc.finish().unwrap();
        assert_eq!(result.text, "");
    }
}
