//! # Ingredients Telegram Bot
//!
//! A Telegram bot that extracts text from images using OCR and stores
//! ingredient measurements in a database with full-text search capabilities.

pub mod bot;
pub mod circuit_breaker;
pub mod db;
pub mod instance_manager;
pub mod localization;
pub mod measurement_patterns;
pub mod measurement_types;
pub mod ocr;
pub mod ocr_config;
pub mod ocr_errors;
pub mod text_processing;

// Re-export types for easier access
pub use measurement_types::{MeasurementConfig, MeasurementMatch};
pub use text_processing::MeasurementDetector;
