use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use crate::error::AiError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub translated_text: String,
    pub source_language: Option<String>,
    pub target_language: String,
    pub model: String,
}

#[async_trait]
pub trait VisionProvider: Send + Sync {
    async fn recognize(
        &self,
        image_bytes: &[u8],
        format: &str,
        cancel: CancellationToken,
    ) -> Result<OcrResult, AiError>;

    async fn translate_image(
        &self,
        image_bytes: &[u8],
        target_lang: &str,
        cancel: CancellationToken,
    ) -> Result<TranslationResult, AiError>;
}
