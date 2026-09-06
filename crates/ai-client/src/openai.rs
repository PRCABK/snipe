use crate::error::AiError;
use crate::provider::{OcrResult, TranslationResult, VisionProvider};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

pub struct OpenAiVisionClient {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout: Duration,
    client: reqwest::Client,
}

impl OpenAiVisionClient {
    pub fn new(base_url: String, api_key: String, model: String, timeout_secs: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs.max(5)))
            .build()
            .unwrap_or_default();

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
            timeout: Duration::from_secs(timeout_secs),
            client,
        }
    }

    async fn send_chat_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        image_bytes: &[u8],
        format: &str,
        cancel: CancellationToken,
    ) -> Result<String, AiError> {
        if self.api_key.trim().is_empty() {
            return Err(AiError::MissingApiKey);
        }

        let mime_type = match format.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            _ => "image/png",
        };

        let b64_img = BASE64.encode(image_bytes);
        let data_uri = format!("data:{};base64,{}", mime_type, b64_img);

        let endpoint = format!("{}/chat/completions", self.base_url);

        let payload = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": user_prompt
                        },
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": data_uri
                            }
                        }
                    ]
                }
            ],
            "max_tokens": 4096,
            "temperature": 0.1
        });

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let auth_val = format!("Bearer {}", self.api_key.trim());
        if let Ok(hv) = HeaderValue::from_str(&auth_val) {
            headers.insert(AUTHORIZATION, hv);
        }

        let request = self.client.post(&endpoint).headers(headers).json(&payload);

        let response = tokio::select! {
            _ = cancel.cancelled() => {
                return Err(AiError::Cancelled);
            }
            res = request.send() => {
                res.map_err(|e| {
                    if e.is_timeout() {
                        AiError::Timeout
                    } else {
                        AiError::Network(e.to_string())
                    }
                })?
            }
        };

        let status = response.status();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(AiError::AuthenticationFailed);
        }
        if status.as_u16() == 429 {
            return Err(AiError::RateLimited);
        }
        if !status.is_success() {
            let err_text = response.text().await.unwrap_or_default();
            warn!("AI API error HTTP {}: {}", status.as_u16(), err_text);
            return Err(AiError::ServerError {
                status: status.as_u16(),
                message: err_text,
            });
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| AiError::InvalidResponse(format!("Failed to parse JSON body: {}", e)))?;

        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                AiError::InvalidResponse("Missing choices[0].message.content".to_string())
            })?;

        info!("Successfully received AI response for model {}", self.model);
        Ok(content.trim().to_string())
    }
}

#[async_trait]
impl VisionProvider for OpenAiVisionClient {
    async fn recognize(
        &self,
        image_bytes: &[u8],
        format: &str,
        cancel: CancellationToken,
    ) -> Result<OcrResult, AiError> {
        let system_prompt = "You are an accurate OCR engine. Extract all readable text from the provided image verbatim. Output ONLY the extracted text with no conversational filler, markdown formatting blocks, or explanations. If no text is found, output nothing.";
        let user_prompt = "Please perform OCR text extraction on this image.";

        let text = self
            .send_chat_completion(system_prompt, user_prompt, image_bytes, format, cancel)
            .await?;

        Ok(OcrResult {
            text,
            model: self.model.clone(),
        })
    }

    async fn translate_image(
        &self,
        image_bytes: &[u8],
        target_lang: &str,
        cancel: CancellationToken,
    ) -> Result<TranslationResult, AiError> {
        let system_prompt = format!(
            "You are a professional image translator. Read all text in the image and translate it accurately into {}. Retain original paragraph structures, formatting, numbers, and proper nouns. Output ONLY the translated text without explanations.",
            target_lang
        );
        let user_prompt = format!(
            "Please translate all text in this image into {}.",
            target_lang
        );

        let text = self
            .send_chat_completion(&system_prompt, &user_prompt, image_bytes, "png", cancel)
            .await?;

        Ok(TranslationResult {
            translated_text: text,
            source_language: None,
            target_language: target_lang.to_string(),
            model: self.model.clone(),
        })
    }
}
