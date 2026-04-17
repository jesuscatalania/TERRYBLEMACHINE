//! Provider-specific HTTP clients. Each file implements the
//! [`AiClient`](crate::ai_router::AiClient) trait for one external service.
//!
//! All clients share:
//! - `reqwest::Client` for HTTP (rustls TLS).
//! - [`common::RateLimiter`] for per-provider rate limits.
//! - [`common::map_http_error`] for status-code → [`ProviderError`]
//!   classification (retriable vs fallback vs permanent).
//! - [`KeyStore`](crate::keychain::KeyStore) for API-key resolution.
//!
//! For Schritt 2.2 each client implements one end-to-end endpoint with
//! wiremock-based tests. Richer endpoint coverage follows in the modules
//! that actually consume them (Phase 3+).

pub mod claude;
pub mod common;
pub mod fal;
pub mod higgsfield;
pub mod ideogram;
pub mod kling;
pub mod meshy;
pub mod registry;
pub mod replicate;
pub mod runway;
pub mod shotstack;
