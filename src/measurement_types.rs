//! # Measurement Types Module
//!
//! This module defines the core types used for measurement detection and processing.

/// Represents a detected measurement in text
#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementMatch {
    /// The matched measurement text (e.g., "2 cups", "500g")
    pub text: String,
    /// The extracted ingredient name (e.g., "flour", "de tomates", "all-purpose flour")
    pub ingredient_name: String,
    /// The line number where the measurement was found
    pub line_number: usize,
    /// The starting character position in the line
    pub start_pos: usize,
    /// The ending character position in the line
    pub end_pos: usize,
}

/// Configuration options for measurement detection
#[derive(Clone, Debug)]
pub struct MeasurementConfig {
    /// Custom regex pattern for measurements. If None, uses the default comprehensive pattern
    #[allow(dead_code)]
    pub custom_pattern: Option<String>,
    /// Whether to enable ingredient name postprocessing (cleaning, normalization)
    pub enable_ingredient_postprocessing: bool,
    /// Maximum length for ingredient names (truncated if longer)
    #[allow(dead_code)]
    pub max_ingredient_length: usize,
    /// Whether to include count-only measurements (e.g., "2 eggs" -> "2")
    #[allow(dead_code)]
    pub include_count_measurements: bool,
}

impl Default for MeasurementConfig {
    fn default() -> Self {
        Self {
            custom_pattern: None,
            enable_ingredient_postprocessing: true,
            max_ingredient_length: 100,
            include_count_measurements: true,
        }
    }
}
