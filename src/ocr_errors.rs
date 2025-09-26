//! # OCR Error Types Module
//!
//! This module defines custom error types used throughout the OCR processing system.
//! It provides structured error handling for various OCR operations and failure modes.

/// Custom error types for OCR operations
#[derive(Debug, Clone)]
pub enum OcrError {
    /// File validation errors
    Validation(String),
    /// OCR engine initialization errors
    Initialization(String),
    /// Image loading errors
    ImageLoad(String),
    /// Text extraction errors
    Extraction(String),
    /// Instance corruption errors
    _InstanceCorruption(String),
    /// Timeout errors
    Timeout(String),
    /// Resource exhaustion errors
    _ResourceExhaustion(String),
}

impl std::fmt::Display for OcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcrError::Validation(msg) => write!(f, "Validation error: {msg}"),
            OcrError::Initialization(msg) => write!(f, "Initialization error: {msg}"),
            OcrError::ImageLoad(msg) => write!(f, "Image load error: {msg}"),
            OcrError::Extraction(msg) => write!(f, "Extraction error: {msg}"),
            OcrError::_InstanceCorruption(msg) => write!(f, "Instance corruption error: {msg}"),
            OcrError::Timeout(msg) => write!(f, "Timeout error: {msg}"),
            OcrError::_ResourceExhaustion(msg) => write!(f, "Resource exhaustion error: {msg}"),
        }
    }
}

impl std::error::Error for OcrError {}

impl From<anyhow::Error> for OcrError {
    fn from(err: anyhow::Error) -> Self {
        OcrError::Extraction(err.to_string())
    }
}
