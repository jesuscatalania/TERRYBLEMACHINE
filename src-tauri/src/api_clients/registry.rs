//! Registers all 9 provider clients with a shared [`KeyStore`]. Used by
//! `lib::run` to populate the router's client map.

use std::collections::HashMap;
use std::sync::Arc;

use crate::ai_router::{AiClient, Provider};
use crate::keychain::KeyStore;

use super::{
    claude::ClaudeClient, fal::FalClient, higgsfield::HiggsfieldClient, ideogram::IdeogramClient,
    kling::KlingClient, meshy::MeshyClient, replicate::ReplicateClient, runway::RunwayClient,
    shotstack::ShotstackClient,
};

/// Build the full set of provider clients. Clients do not hit the network
/// until dispatched; missing API keys surface as `ProviderError::Auth`
/// during `execute`, which the router treats as "try another model".
pub fn build_default_clients(keystore: Arc<dyn KeyStore>) -> HashMap<Provider, Arc<dyn AiClient>> {
    let mut m: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
    m.insert(
        Provider::Claude,
        Arc::new(ClaudeClient::new(keystore.clone())),
    );
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
