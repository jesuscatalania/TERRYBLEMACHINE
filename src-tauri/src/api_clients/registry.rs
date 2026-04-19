//! Registers all 9 provider clients with a shared [`KeyStore`]. Used by
//! `lib::run` to populate the router's client map.

use std::collections::HashMap;
use std::sync::Arc;

use crate::ai_router::{AiClient, Provider};
use crate::keychain::KeyStore;

use super::{
    claude::ClaudeClient, claude_cli::discovery::detect_claude_binary,
    claude_cli::ClaudeCliClient, claude_cli_commands::TRANSPORT_META_KEY, fal::FalClient,
    higgsfield::HiggsfieldClient, ideogram::IdeogramClient, kling::KlingClient, meshy::MeshyClient,
    replicate::ReplicateClient, runway::RunwayClient, shotstack::ShotstackClient,
};

/// Build the full set of provider clients. Clients do not hit the network
/// until dispatched; missing API keys surface as `ProviderError::Auth`
/// during `execute`, which the router treats as "try another model".
///
/// Note on Kling: `KlingClient` (direct Kling API) stays registered so
/// users who configure a `kling` keychain entry can use it as an
/// emergency fallback, but the default video chain
/// (`router.rs::DefaultRoutingStrategy`) routes text-to-video /
/// image-to-video through the fal.ai aggregator
/// (`Model::FalKlingV2Master` / `Model::FalKlingV15`). Under the default
/// routing strategy, `Model::Kling20` is never selected.
pub fn build_default_clients(keystore: Arc<dyn KeyStore>) -> HashMap<Provider, Arc<dyn AiClient>> {
    let mut m: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();

    // Claude transport selection — the user can pin it via Settings
    // (`auto` | `api` | `cli`). Defaults to `auto` when nothing stored.
    // Under `auto`, we prefer the local CLI (subscription billing) when
    // it's installed, else fall back to the HTTP client.
    let transport = keystore
        .get(TRANSPORT_META_KEY)
        .ok()
        .unwrap_or_else(|| "auto".to_string());
    let claude_client: Arc<dyn AiClient> = match transport.as_str() {
        "cli" => match detect_claude_binary() {
            Some(bin) => Arc::new(ClaudeCliClient::new(bin)),
            None => {
                eprintln!(
                    "[registry] Claude transport pinned to 'cli' but no claude binary found — falling back to HTTP API"
                );
                Arc::new(ClaudeClient::new(keystore.clone()))
            }
        },
        "api" => Arc::new(ClaudeClient::new(keystore.clone())),
        _ => match detect_claude_binary() {
            Some(bin) => Arc::new(ClaudeCliClient::new(bin)),
            None => Arc::new(ClaudeClient::new(keystore.clone())),
        },
    };
    m.insert(Provider::Claude, claude_client);
    m.insert(
        Provider::Kling,
        Arc::new(KlingClient::new(keystore.clone())),
    );
    m.insert(
        Provider::Runway,
        Arc::new(RunwayClient::new(keystore.clone())),
    );
    m.insert(
        Provider::Higgsfield,
        Arc::new(HiggsfieldClient::new(keystore.clone())),
    );
    m.insert(
        Provider::Shotstack,
        Arc::new(ShotstackClient::new(keystore.clone())),
    );
    m.insert(
        Provider::Ideogram,
        Arc::new(IdeogramClient::new(keystore.clone())),
    );
    m.insert(
        Provider::Meshy,
        Arc::new(MeshyClient::new(keystore.clone())),
    );
    m.insert(Provider::Fal, Arc::new(FalClient::new(keystore.clone())));
    m.insert(
        Provider::Replicate,
        Arc::new(ReplicateClient::new(keystore)),
    );
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keychain::InMemoryStore;

    #[tokio::test]
    async fn registry_contains_all_nine_providers() {
        let ks: Arc<dyn KeyStore> = Arc::new(InMemoryStore::new());
        let m = build_default_clients(ks);
        for p in [
            Provider::Claude,
            Provider::Kling,
            Provider::Runway,
            Provider::Higgsfield,
            Provider::Shotstack,
            Provider::Ideogram,
            Provider::Meshy,
            Provider::Fal,
            Provider::Replicate,
        ] {
            assert!(m.contains_key(&p), "missing provider {p:?}");
        }
        assert_eq!(m.len(), 9);
    }
}
