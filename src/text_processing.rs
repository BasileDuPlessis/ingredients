//! # Text Processing Module
//!
//! This module provides text processing utilities for the Ingredients Telegram bot,
//! including regex-based measurement detection and ingredient parsing.
//!
//! ## Features
//!
//! - Measurement unit detection using comprehensive regex patterns
//! - Support for English and French measurement units
//! - Ingredient name extraction alongside quantity and measurement
//! - Line-by-line text analysis for ingredient lists

use regex::Regex;
use std::collections::HashSet;

/// Represents a detected measurement in text
#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementMatch {
    /// The matched measurement text (e.g., "2 cups", "500g")
    pub text: String,
    /// The extracted ingredient name (e.g., "flour", "de tomates", "all-purpose flour")
    pub ingredient_name: String,
    /// The line number where the measurement was found (0-indexed)
    pub line_number: usize,
    /// The starting character position in the line
    pub start_pos: usize,
    /// The ending character position in the line
    pub end_pos: usize,
}

/// Measurement detector using regex patterns for English and French units
pub struct MeasurementDetector {
    /// Compiled regex pattern for detecting measurements
    pattern: Regex,
}

impl MeasurementDetector {
    /// Create a new measurement detector with the default comprehensive pattern
    ///
    /// The pattern matches common measurement units in both English and French,
    /// including volume, weight, count, and other ingredient measurements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingredients::text_processing::MeasurementDetector;
    ///
    /// let detector = MeasurementDetector::new();
    /// ```
    pub fn new() -> Result<Self, regex::Error> {
        // Comprehensive regex pattern for measurement units
        // This pattern matches numbers followed by units, or units alone
        let pattern_str = r#"(?i)\b\d*\.?\d+\s*(?:cup(?:s)?|teaspoon(?:s)?|tsp(?:\.?)|tablespoon(?:s)?|tbsp(?:\.?)|pint(?:s)?|quart(?:s)?|gallon(?:s)?|oz|ounce(?:s)?|lb(?:\.?)|pound(?:s)?|mg|g|gram(?:me)?s?|kg|kilogram(?:me)?s?|l|liter(?:s)?|litre(?:s)?|ml|millilitre(?:s)?|cc|cl|dl|cm3|mm3|cm²|mm²|slice(?:s)?|can(?:s)?|bottle(?:s)?|stick(?:s)?|packet(?:s)?|pkg|bag(?:s)?|dash(?:es)?|pinch(?:es)?|drop(?:s)?|cube(?:s)?|piece(?:s)?|handful(?:s)?|bar(?:s)?|sheet(?:s)?|serving(?:s)?|portion(?:s)?|tasse(?:s)?|cuillère(?:s)?(?:\s+à\s+(?:café|soupe))?|poignée(?:s)?|sachet(?:s)?|paquet(?:s)?|boîte(?:s)?|conserve(?:s)?|tranche(?:s)?|morceau(?:x)?|gousse(?:s)?|brin(?:s)?|feuille(?:s)?|bouquet(?:s)?|egg(?:s)?|œuf(?:s)?)\b"#;

        let pattern = Regex::new(pattern_str)?;
        Ok(Self { pattern })
    }

    /// Create a measurement detector with a custom regex pattern
    ///
    /// # Arguments
    ///
    /// * `pattern` - Custom regex pattern string
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingredients::text_processing::MeasurementDetector;
    ///
    /// let custom_pattern = r"\b\d+\s*(?:cups?|tablespoons?)\b";
    /// let detector = MeasurementDetector::with_pattern(custom_pattern)?;
    /// # Ok::<(), regex::Error>(())
    /// ```
    pub fn with_pattern(pattern: &str) -> Result<Self, regex::Error> {
        let pattern = Regex::new(pattern)?;
        Ok(Self { pattern })
    }

    /// Find all measurement matches in the given text
    ///
    /// Scans the entire text and returns all detected measurements with their
    /// positions, line numbers, and extracted ingredient names.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to scan for measurements
    ///
    /// # Returns
    ///
    /// Returns a vector of `MeasurementMatch` containing all detected measurements
    /// with their associated ingredient names
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingredients::text_processing::MeasurementDetector;
    ///
    /// let detector = MeasurementDetector::new()?;
    /// let text = "2 cups flour\n1 tablespoon sugar\nsome salt";
    /// let matches = detector.find_measurements(text);
    ///
    /// assert_eq!(matches.len(), 2);
    /// assert_eq!(matches[0].text, "2 cups");
    /// assert_eq!(matches[0].ingredient_name, "flour");
    /// assert_eq!(matches[1].text, "1 tablespoon");
    /// assert_eq!(matches[1].ingredient_name, "sugar");
    /// # Ok::<(), regex::Error>(())
    /// ```
    pub fn find_measurements(&self, text: &str) -> Vec<MeasurementMatch> {
        let mut matches = Vec::new();
        let mut current_pos = 0;

        for (line_number, line) in text.lines().enumerate() {
            for capture in self.pattern.find_iter(line) {
                // Extract the ingredient name from the text after the measurement
                let measurement_end = capture.end();
                let ingredient_name = line[measurement_end..].trim().to_string();

                matches.push(MeasurementMatch {
                    text: capture.as_str().to_string(),
                    ingredient_name,
                    line_number,
                    start_pos: current_pos + capture.start(),
                    end_pos: current_pos + capture.end(),
                });
            }
            current_pos += line.len() + 1; // +1 for newline character
        }

        matches
    }

    /// Extract lines containing measurements from the text
    ///
    /// Returns all lines that contain at least one measurement unit.
    ///
    /// # Arguments
    ///
    /// * `text` - The multi-line text to analyze
    ///
    /// # Returns
    ///
    /// Returns a vector of tuples containing (line_number, line_content) for
    /// lines that contain measurements
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingredients::text_processing::MeasurementDetector;
    ///
    /// let detector = MeasurementDetector::new()?;
    /// let text = "2 cups flour\n1 tablespoon sugar\nsome salt\n3 eggs";
    /// let measurement_lines = detector.extract_measurement_lines(text);
    ///
    /// assert_eq!(measurement_lines.len(), 3); // eggs might be detected as measurements
    /// # Ok::<(), regex::Error>(())
    /// ```
    pub fn extract_measurement_lines(&self, text: &str) -> Vec<(usize, String)> {
        text.lines()
            .enumerate()
            .filter(|(_, line)| self.pattern.is_match(line))
            .map(|(i, line)| (i, line.to_string()))
            .collect()
    }

    /// Check if a given text contains any measurements
    ///
    /// # Arguments
    ///
    /// * `text` - The text to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the text contains at least one measurement unit
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingredients::text_processing::MeasurementDetector;
    ///
    /// let detector = MeasurementDetector::new()?;
    /// assert!(detector.has_measurements("2 cups flour"));
    /// assert!(!detector.has_measurements("some flour"));
    /// # Ok::<(), regex::Error>(())
    /// ```
    pub fn has_measurements(&self, text: &str) -> bool {
        self.pattern.is_match(text)
    }

    /// Get all unique measurement units found in the text
    ///
    /// # Arguments
    ///
    /// * `text` - The text to analyze
    ///
    /// # Returns
    ///
    /// Returns a HashSet of unique measurement unit strings found
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingredients::text_processing::MeasurementDetector;
    /// use std::collections::HashSet;
    ///
    /// let detector = MeasurementDetector::new()?;
    /// let text = "2 cups flour\n1 cup sugar\n500g butter";
    /// let units = detector.get_unique_units(text);
    ///
    /// assert!(units.contains("cups"));
    /// assert!(units.contains("cup"));
    /// assert!(units.contains("g"));
    /// # Ok::<(), regex::Error>(())
    /// ```
    pub fn get_unique_units(&self, text: &str) -> HashSet<String> {
        self.pattern
            .find_iter(text)
            .map(|m| m.as_str().to_lowercase())
            .collect()
    }
}

impl Default for MeasurementDetector {
    fn default() -> Self {
        Self::new().expect("Default measurement pattern should be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_detector() -> MeasurementDetector {
        MeasurementDetector::new().unwrap()
    }

    #[test]
    fn test_measurement_detector_creation() {
        let detector = create_detector();
        assert!(detector.pattern.as_str().len() > 0);
    }

    #[test]
    fn test_basic_measurement_detection() {
        let detector = create_detector();

        // Test basic measurements
        assert!(detector.has_measurements("2 cups flour"));
        assert!(detector.has_measurements("1 tablespoon sugar"));
        assert!(detector.has_measurements("500g butter"));
        assert!(detector.has_measurements("1 kg tomatoes"));
        assert!(detector.has_measurements("250 ml milk"));
    }

    #[test]
    fn test_no_measurement_detection() {
        let detector = create_detector();

        assert!(!detector.has_measurements("some flour"));
        assert!(!detector.has_measurements("add salt"));
        assert!(!detector.has_measurements(""));
    }

    #[test]
    fn test_extract_measurement_lines() {
        let detector = create_detector();
        let text = "2 cups flour\n1 tablespoon sugar\nsome salt\nto taste";

        let lines = detector.extract_measurement_lines(text);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], (0, "2 cups flour".to_string()));
        assert_eq!(lines[1], (1, "1 tablespoon sugar".to_string()));
    }

    #[test]
    fn test_find_measurements_with_positions() {
        let detector = create_detector();
        let text = "Mix 2 cups flour with 1 tbsp sugar";

        let matches = detector.find_measurements(text);

        assert_eq!(matches.len(), 2);

        // First match: "2 cups"
        assert_eq!(matches[0].text, "2 cups");
        assert_eq!(matches[0].line_number, 0);
        assert_eq!(matches[0].start_pos, 4);
        assert_eq!(matches[0].end_pos, 10);

        // Second match: "1 tbsp"
        assert_eq!(matches[1].text, "1 tbsp");
        assert_eq!(matches[1].line_number, 0);
    }

    #[test]
    fn test_french_measurements() {
        let detector = create_detector();

        // Test French measurements
        assert!(detector.has_measurements("2 tasses de farine"));
        assert!(detector.has_measurements("1 cuillère à soupe de sucre"));
        assert!(detector.has_measurements("500 g de beurre"));
        assert!(detector.has_measurements("1 kg de tomates"));
    }

    #[test]
    fn test_comprehensive_french_measurements() {
        let detector = create_detector();

        // Test volume measurements
        assert!(detector.has_measurements("2 tasses de lait"));
        assert!(detector.has_measurements("1 cuillère à café de sel"));
        assert!(detector.has_measurements("3 cuillères à soupe d'huile"));
        assert!(detector.has_measurements("250 ml d'eau"));
        assert!(detector.has_measurements("1 litre de jus"));

        // Test weight measurements
        assert!(detector.has_measurements("500 grammes de sucre"));
        assert!(detector.has_measurements("1 kilogramme de pommes"));
        assert!(detector.has_measurements("200 g de chocolat"));

        // Test count measurements
        assert!(detector.has_measurements("3 œufs"));
        assert!(detector.has_measurements("2 tranches de pain"));
        assert!(detector.has_measurements("1 boîte de conserve"));
        assert!(detector.has_measurements("4 morceaux de poulet"));
        assert!(detector.has_measurements("1 sachet de levure"));
        assert!(detector.has_measurements("2 paquets de pâtes"));
        assert!(detector.has_measurements("1 poignée d'amandes"));
        assert!(detector.has_measurements("3 gousses d'ail"));
        assert!(detector.has_measurements("1 brin de persil"));
        assert!(detector.has_measurements("2 feuilles de laurier"));
        assert!(detector.has_measurements("1 bouquet de thym"));
    }

    #[test]
    fn test_abbreviations() {
        let detector = create_detector();

        // Test abbreviations
        assert!(detector.has_measurements("1 tsp salt"));
        assert!(detector.has_measurements("2 tbsp oil"));
        assert!(detector.has_measurements("1 lb beef"));
        assert!(detector.has_measurements("8 oz water"));
    }

    #[test]
    fn test_plural_forms() {
        let detector = create_detector();

        // Test plural forms
        assert!(detector.has_measurements("2 cups"));
        assert!(detector.has_measurements("1 tablespoon"));
        assert!(detector.has_measurements("3 teaspoons"));
        assert!(detector.has_measurements("4 ounces"));
    }

    #[test]
    fn test_decimal_numbers() {
        let detector = create_detector();

        // Test decimal numbers
        assert!(detector.has_measurements("2.5 cups flour"));
        assert!(detector.has_measurements("0.5 kg sugar"));
        assert!(detector.has_measurements("1.25 liters milk"));
    }

    #[test]
    fn test_count_measurements() {
        let detector = create_detector();

        // Test count-based measurements
        assert!(detector.has_measurements("3 eggs"));
        assert!(detector.has_measurements("2 slices bread"));
        assert!(detector.has_measurements("1 can tomatoes"));
        assert!(detector.has_measurements("4 pieces chicken"));
    }

    #[test]
    fn test_unique_units_extraction() {
        let detector = create_detector();
        let text = "2 cups flour\n1 cup sugar\n500g butter\n200g flour";

        let units = detector.get_unique_units(text);

        // Should contain the measurement parts
        assert!(units.iter().any(|u| u.contains("cups")));
        assert!(units.iter().any(|u| u.contains("cup")));
        assert!(units.iter().any(|u| u.contains("g")));
    }

    #[test]
    fn test_multi_line_text() {
        let detector = create_detector();
        let text = "Ingredients:\n2 cups flour\n1 tablespoon sugar\n1 teaspoon salt\n\nInstructions:\nMix well";

        let matches = detector.find_measurements(text);

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].line_number, 1); // "2 cups flour"
        assert_eq!(matches[1].line_number, 2); // "1 tablespoon sugar"
        assert_eq!(matches[2].line_number, 3); // "1 teaspoon salt"
    }

    #[test]
    fn test_custom_pattern() {
        let pattern = r"\b\d+\s*(?:cups?|tablespoons?)\b";
        let detector = MeasurementDetector::with_pattern(pattern).unwrap();

        assert!(detector.has_measurements("2 cups flour"));
        assert!(detector.has_measurements("1 tablespoon sugar"));
        assert!(!detector.has_measurements("500g butter")); // g not in custom pattern
    }

    #[test]
    fn test_case_insensitive_matching() {
        let detector = create_detector();

        assert!(detector.has_measurements("2 CUPS flour"));
        assert!(detector.has_measurements("1 Tablespoon sugar"));
        assert!(detector.has_measurements("500G butter"));
    }

    #[test]
    fn test_ingredient_name_extraction() {
        let detector = create_detector();

        // Test basic ingredient name extraction
        let matches = detector.find_measurements("2 cups flour\n1 tablespoon sugar\n500g butter");

        assert_eq!(matches.len(), 3);

        assert_eq!(matches[0].text, "2 cups");
        assert_eq!(matches[0].ingredient_name, "flour");

        assert_eq!(matches[1].text, "1 tablespoon");
        assert_eq!(matches[1].ingredient_name, "sugar");

        assert_eq!(matches[2].text, "500g");
        assert_eq!(matches[2].ingredient_name, "butter");
    }

    #[test]
    fn test_french_ingredient_name_extraction() {
        let detector = create_detector();

        // Test French ingredient name extraction
        let matches = detector.find_measurements("250 g de farine\n1 litre de lait\n3 œufs");

        assert_eq!(matches.len(), 3);

        assert_eq!(matches[0].text, "250 g");
        assert_eq!(matches[0].ingredient_name, "de farine");

        assert_eq!(matches[1].text, "1 litre");
        assert_eq!(matches[1].ingredient_name, "de lait");

        assert_eq!(matches[2].text, "3 œufs");
        assert_eq!(matches[2].ingredient_name, ""); // "œufs" is both measurement and ingredient
    }

    #[test]
    fn test_multi_word_ingredient_names() {
        let detector = create_detector();

        // Test multi-word ingredient names
        let matches = detector.find_measurements("2 cups all-purpose flour\n1 teaspoon baking powder\n500g unsalted butter");

        assert_eq!(matches.len(), 3);

        assert_eq!(matches[0].text, "2 cups");
        assert_eq!(matches[0].ingredient_name, "all-purpose flour");

        assert_eq!(matches[1].text, "1 teaspoon");
        assert_eq!(matches[1].ingredient_name, "baking powder");

        assert_eq!(matches[2].text, "500g");
        assert_eq!(matches[2].ingredient_name, "unsalted butter");
    }

    #[test]
    fn test_measurement_at_end_of_line() {
        let detector = create_detector();

        // Test when measurement is at the end of the line (no ingredient name)
        let matches = detector.find_measurements("Add 2 cups\nMix 1 tablespoon\nBake at 350");

        assert_eq!(matches.len(), 2);

        assert_eq!(matches[0].text, "2 cups");
        assert_eq!(matches[0].ingredient_name, "");

        assert_eq!(matches[1].text, "1 tablespoon");
        assert_eq!(matches[1].ingredient_name, "");
    }
}