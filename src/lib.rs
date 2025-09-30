//! # Ingredients Telegram Bot
//!
//! A Telegram bot that extracts text from images using OCR and stores
//! ingredient measurements in a database with full-text search capabilities.

pub mod bot;
pub mod circuit_breaker;
pub mod db;
pub mod dialogue;
pub mod instance_manager;
pub mod localization;
pub mod measurement_patterns;
pub mod ocr;
pub mod ocr_config;
pub mod ocr_errors;
pub mod text_processing;

// Re-export types for easier access
pub use text_processing::{MeasurementConfig, MeasurementDetector, MeasurementMatch};
