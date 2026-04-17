//! Anthropic Claude client (Messages API, Vision-ready).
//!
//! Reference implementation — all other clients in this module follow the
//! same pattern:
//! 1. `new(key_store, base_url, rate_per_sec)` — keychain-backed constructor.
//! 2. Impl of [`AiClient`](crate::ai_router::AiClient) dispatches
//!    model-specific payloads through a single `send_request` HTTP call.
//! 3. Wiremock-based unit tests cover happy path + key error modes.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::common::{get_api_key, map_http_error, map_reqwest_error, RateLimiter};
use crate::ai_router::{
    AiClient, AiRequest, AiResponse, Model, Provider, ProviderError, ProviderUsage,
};
use crate::keychain::KeyStore;

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";
/// Keychain service id under which the Anthropic key lives.
pub const KEYCHAIN_SERVICE: &str = "claude";
/// Default rate: Claude Max caps roughly at 50 rps across endpoints.
const DEFAULT_RATE_PER_SEC: usize = 10;

pub struct ClaudeClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
}

impl ClaudeClient {
    pub fn new(key_store: Arc<dyn KeyStore>) -> Self {
        Self::with_base_url(key_store, DEFAULT_BASE_URL.to_owned(), DEFAULT_RATE_PER_SEC)
    }

    pub fn with_base_url(
        key_store: Arc<dyn KeyStore>,
        base_url: String,
        rate_per_sec: usize,
    ) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("reqwest client builds");
        Self {
            http,
            base_url,
            key_store,
            rate: RateLimiter::new(rate_per_sec),
        }
    }

    /// Shared test-friendly variant that doesn't schedule a refill task.
    #[cfg(test)]
    pub fn for_test(key_store: Arc<dyn KeyStore>, base_url: String) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("reqwest client builds");
        Self {
            http,
            base_url,
            key_store,
            rate: RateLimiter::unlimited(),
        }
    }

    fn model_slug(model: Model) -> Option<&'static str> {
        match model {
            Model::ClaudeOpus => Some("claude-opus-4-7"),
            Model::ClaudeSonnet => Some("claude-sonnet-4-6"),
            Model::ClaudeHaiku => Some("claude-haiku-4-5"),
            _ => None,
        }
    }

    async fn send_messages(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
        let slug = Self::model_slug(model)
            .ok_or_else(|| ProviderError::Permanent(format!("unsupported model {model:?}")))?;

        // Vision path: if `payload.images[]` is set, build a multi-block
        // content array with one text block followed by one `image` block
        // per entry. Each image is expected as { media_type, data } where
        // data is base64-encoded and media_type is an RFC 2046 MIME type.
        let content = if let Some(imgs) = request.payload.get("images").and_then(|v| v.as_array()) {
            let mut blocks = vec![json!({ "type": "text", "text": request.prompt.clone() })];
            for img in imgs {
                let media_type =
                    img.get("media_type")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ProviderError::Permanent(
                                "claude vision: images[].media_type required".into(),
                            )
                        })?;
                let data = img.get("data").and_then(|v| v.as_str()).ok_or_else(|| {
                    ProviderError::Permanent("claude vision: images[].data required".into())
                })?;
                blocks.push(json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": media_type,
                        "data": data
                    }
                }));
            }
            serde_json::Value::Array(blocks)
        } else {
            serde_json::Value::String(request.prompt.clone())
        };

        let body = json!({
            "model": slug,
            "max_tokens": 1024,
            "messages": [
                { "role": "user", "content": content }
            ]
        });

        let url = format!("{}/v1/messages", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("x-api-key", key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(map_reqwest_error)?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_http_error(status, &text));
        }

        let parsed: MessagesResponse = resp.json().await.map_err(map_reqwest_error)?;
        let output = parsed
            .content
            .iter()
            .filter_map(|c| c.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({ "text": output, "stop_reason": parsed.stop_reason }),
            cost_cents: None,
            cached: false,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessagesResponse {
    #[serde(default)]
    content: Vec<ContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

#[async_trait]
impl AiClient for ClaudeClient {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn supports(&self, model: Model) -> bool {
        Self::model_slug(model).is_some()
    }

    async fn execute(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        self.send_messages(model, request).await
    }

    async fn health_check(&self) -> bool {
        // Cheap health check: the /v1/messages endpoint with an empty body
        // returns 400 when reachable with a valid key. For now we just
        // confirm the key exists in the keychain.
        self.key_store.get(KEYCHAIN_SERVICE).is_ok()
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        // Anthropic doesn't expose a usage endpoint — callers track via
        // response-level `usage` fields (tokens in/out).
        Ok(ProviderUsage {
            notes: Some("tracked per-request via response.usage".into()),
            ..ProviderUsage::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_router::{Complexity, Priority, TaskKind};
    use crate::keychain::InMemoryStore;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn key_store_with_key() -> Arc<dyn KeyStore> {
        let s = InMemoryStore::new();
        s.store(KEYCHAIN_SERVICE, "sk-test").unwrap();
        Arc::new(s)
    }

    fn request(prompt: &str) -> AiRequest {
        AiRequest {
            id: "r1".into(),
            task: TaskKind::TextGeneration,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: prompt.into(),
            payload: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn happy_path_returns_text_output() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "sk-test"))
            .and(header("anthropic-version", ANTHROPIC_VERSION))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "content": [{ "type": "text", "text": "hello world" }],
                "stop_reason": "end_turn"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = ClaudeClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::ClaudeSonnet, &request("say hi"))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::ClaudeSonnet);
        assert_eq!(
            resp.output.get("text").unwrap().as_str().unwrap(),
            "hello world"
        );
    }

    #[tokio::test]
    async fn missing_key_yields_auth_error() {
        let server = MockServer::start().await;
        let empty = Arc::new(InMemoryStore::new()) as Arc<dyn KeyStore>;
        let client = ClaudeClient::for_test(empty, server.uri());
        let err = client
            .execute(Model::ClaudeSonnet, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn server_500_is_transient() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream"))
            .mount(&server)
            .await;
        let client = ClaudeClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ClaudeSonnet, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Transient(_)));
    }

    #[tokio::test]
    async fn status_401_is_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid key"))
            .mount(&server)
            .await;
        let client = ClaudeClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ClaudeSonnet, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn unsupported_model_is_permanent() {
        let server = MockServer::start().await;
        let client = ClaudeClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::Kling20, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn supports_reports_only_claude_models() {
        let client = ClaudeClient::for_test(key_store_with_key(), "http://localhost".to_string());
        assert!(client.supports(Model::ClaudeOpus));
        assert!(client.supports(Model::ClaudeSonnet));
        assert!(client.supports(Model::ClaudeHaiku));
        assert!(!client.supports(Model::Kling20));
        assert!(!client.supports(Model::FalFluxPro));
    }

    #[tokio::test]
    async fn get_usage_returns_default_for_claude() {
        let client = ClaudeClient::for_test(key_store_with_key(), "http://localhost".to_string());
        let usage = client.get_usage().await.unwrap();
        assert!(usage.notes.is_some());
    }

    #[tokio::test]
    async fn vision_payload_uses_image_block() {
        // When `payload.images[]` is present the request body's
        // messages[0].content must be an array with a text block followed
        // by one image block per entry, each with a base64 source.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(body_partial_json(json!({
                "messages": [{
                    "role": "user",
                    "content": [
                        { "type": "text", "text": "describe this image" },
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": "AAAA"
                            }
                        }
                    ]
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "content": [{ "type": "text", "text": "ok" }],
                "stop_reason": "end_turn"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = ClaudeClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "v1".into(),
            task: TaskKind::ImageAnalysis,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "describe this image".into(),
            payload: json!({
                "images": [{ "media_type": "image/png", "data": "AAAA" }]
            }),
        };
        client.execute(Model::ClaudeSonnet, &req).await.unwrap();
    }

    #[tokio::test]
    async fn vision_payload_rejects_missing_media_type() {
        let server = MockServer::start().await;
        let client = ClaudeClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "v2".into(),
            task: TaskKind::ImageAnalysis,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "x".into(),
            payload: json!({ "images": [{ "data": "AAAA" }] }),
        };
        let err = client.execute(Model::ClaudeSonnet, &req).await.unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }
}
