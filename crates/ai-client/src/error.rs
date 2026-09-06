use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum AiError {
    #[error("API key is missing or not configured")]
    MissingApiKey,

    #[error("Authentication failed (401/403): please check your API key")]
    AuthenticationFailed,

    #[error("Rate limit or quota exceeded (429): please check your billing / rate limit")]
    RateLimited,

    #[error("Request timed out")]
    Timeout,

    #[error("Request was cancelled by user")]
    Cancelled,

    #[error("Network connection error: {0}")]
    Network(String),

    #[error("Server returned error ({status}): {message}")]
    ServerError { status: u16, message: String },

    #[error("Failed to parse API response: {0}")]
    InvalidResponse(String),

    #[error("Image too large or unsupported format")]
    ImageError(String),
}
