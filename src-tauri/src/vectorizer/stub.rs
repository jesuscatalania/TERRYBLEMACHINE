//! Deterministic [`Vectorizer`] used by integration tests.
//!
//! Returns a canned 100×100 SVG filled with the TERRYBLE accent color so
//! tests can assert shape without pulling the real vtracer dependency
//! (which needs a valid PNG — fiddly to fabricate in-memory).

use async_trait::async_trait;

use super::types::{validate_input, VectorizeError, VectorizeInput, VectorizeResult, Vectorizer};

pub struct StubVectorizer;

impl StubVectorizer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StubVectorizer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Vectorizer for StubVectorizer {
    async fn vectorize(&self, input: VectorizeInput) -> Result<VectorizeResult, VectorizeError> {
        validate_input(&input)?;
        if !input.image_path.exists() {
            return Err(VectorizeError::InvalidInput("missing image".into()));
        }
        Ok(VectorizeResult {
            svg: r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="#e85d2d"/></svg>"##.into(),
            width: 100,
            height: 100,
        })
    }
}
