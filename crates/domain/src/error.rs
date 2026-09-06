use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomainError {
    #[error("Coordinate or rectangle is out of bounds: {0}")]
    OutOfBounds(String),

    #[error("Invalid dimensions: width={0}, height={1}")]
    InvalidDimensions(u32, u32),

    #[error("Coordinate conversion failed: {0}")]
    ConversionError(String),

    #[error("Pixel buffer length mismatch: expected {expected}, actual {actual}")]
    BufferMismatch { expected: usize, actual: usize },
}
