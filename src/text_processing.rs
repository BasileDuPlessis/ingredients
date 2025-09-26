//! # Text Processing Module
//!
//! This module provides text processing utilities for the Ingredients Telegram bot,
//! including regex-based measurement detection and ingredient parsing.
//!
//! ## Features
//!
//! - Measurement unit detection using comprehensive regex patterns
//! - Support for English and French measurement units
//! - **Quantity-only ingredient support**: Recognizes ingredients with quantities but no units (e.g., "6 oeufs", "4 pommes")
//! - Ingredient name extraction alongside quantity and measurement
//! - Line-by-line text analysis for ingredient lists

use lazy_static::lazy_static;
use log::{debug, info, trace, warn};
use regex::Regex;
use std::collections::HashSet;

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
#[derive(Debug, Clone)]
pub struct MeasurementConfig {
    /// Custom regex pattern for measurements. If None, uses the default comprehensive pattern
    pub custom_pattern: Option<String>,
    /// Whether to enable ingredient name post-processing
    pub enable_ingredient_postprocessing: bool,
    /// Maximum length for ingredient names (to prevent overly long extractions)
    pub max_ingredient_length: usize,
    /// Whether to include count-based measurements (eggs, slices, etc.)
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

/// Measurement detector using regex patterns for English and French units
pub struct MeasurementDetector {
    /// Compiled regex pattern for detecting measurements
    pattern: Regex,
    /// Configuration options
    config: MeasurementConfig,
}

// Default comprehensive regex pattern for measurement units (now supports quantity-only ingredients)
const DEFAULT_PATTERN: &str = r#"(?i)\b\d*\.?\d+(?:\s*(?:cup(?:s)?|teaspoon(?:s)?|tsp(?:\.?)|tablespoon(?:s)?|tbsp(?:\.?)|pint(?:s)?|quart(?:s)?|gallon(?:s)?|oz|ounce(?:s)?|lb(?:\.?)|pound(?:s)?|mg|g|gram(?:me)?s?|kg|kilogram(?:me)?s?|l|liter(?:s)?|litre(?:s)?|ml|millilitre(?:s)?|cc|cl|dl|cm3|mm3|cm²|mm²|slice(?:s)?|can(?:s)?|bottle(?:s)?|stick(?:s)?|packet(?:s)?|pkg|bag(?:s)?|dash(?:es)?|pinch(?:es)?|drop(?:s)?|cube(?:s)?|piece(?:s)?|handful(?:s)?|bar(?:s)?|sheet(?:s)?|serving(?:s)?|portion(?:s)?|tasse(?:s)?|cuillère(?:s)?(?:\s+à\s+(?:café|soupe))?|poignée(?:s)?|sachet(?:s)?|paquet(?:s)?|boîte(?:s)?|conserve(?:s)?|tranche(?:s)?|morceau(?:x)?|gousse(?:s)?|brin(?:s)?|feuille(?:s)?|bouquet(?:s)?)|\s+\w+)\b"#;

// Lazy static regex for default pattern to avoid recompilation
lazy_static! {
    static ref DEFAULT_REGEX: Regex =
        Regex::new(DEFAULT_PATTERN).expect("Default measurement pattern should be valid");
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
        info!("Creating new MeasurementDetector with default configuration");
        Ok(Self {
            pattern: DEFAULT_REGEX.clone(),
            config: MeasurementConfig::default(),
        })
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
        Ok(Self {
            pattern,
            config: MeasurementConfig::default(),
        })
    }

    /// Create a measurement detector with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration options for the detector
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingredients::text_processing::{MeasurementDetector, MeasurementConfig};
    ///
    /// let config = MeasurementConfig {
    ///     enable_ingredient_postprocessing: true,
    ///     max_ingredient_length: 50,
    ///     ..Default::default()
    /// };
    /// let detector = MeasurementDetector::with_config(config)?;
    /// # Ok::<(), regex::Error>(())
    /// ```
    pub fn with_config(config: MeasurementConfig) -> Result<Self, regex::Error> {
        let pattern = if let Some(ref custom_pattern) = config.custom_pattern {
            debug!("Using custom regex pattern: {}", custom_pattern);
            Regex::new(custom_pattern)?
        } else {
            debug!("Using default regex pattern");
            DEFAULT_REGEX.clone()
        };

        info!("Creating MeasurementDetector with custom config: postprocessing={}, max_length={}, count_measurements={}",
              config.enable_ingredient_postprocessing, config.max_ingredient_length, config.include_count_measurements);

        Ok(Self { pattern, config })
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

        debug!(
            "Finding measurements in text with {} lines",
            text.lines().count()
        );

        for (line_number, line) in text.lines().enumerate() {
            trace!("Processing line {}: '{}'", line_number, line);
            for capture in self.pattern.find_iter(line) {
                let measurement_text = capture.as_str();
                debug!(
                    "Found measurement '{}' at line {}",
                    measurement_text, line_number
                );

                // Check if this is a quantity-only ingredient (no measurement unit)
                let is_quantity_only = self.is_quantity_only_match(measurement_text);

                let (final_measurement_text, raw_ingredient_name) = if is_quantity_only {
                    // For quantity-only ingredients, split the match into quantity and ingredient
                    if let Some(space_pos) = measurement_text.find(' ') {
                        let quantity = &measurement_text[..space_pos];
                        let ingredient = &measurement_text[space_pos + 1..];
                        debug!(
                            "Split quantity-only ingredient: '{}' -> quantity='{}', ingredient='{}'",
                            measurement_text, quantity, ingredient
                        );
                        (quantity.to_string(), ingredient.to_string())
                    } else {
                        // Fallback: shouldn't happen with current regex
                        (measurement_text.to_string(), String::new())
                    }
                } else {
                    // Traditional measurement: extract ingredient name from text after the measurement
                    let measurement_end = capture.end();
                    let ingredient_name = line[measurement_end..].trim().to_string();
                    (measurement_text.to_string(), ingredient_name)
                };

                let ingredient_name = self.post_process_ingredient_name(&raw_ingredient_name);

                trace!(
                    "Extracted ingredient name: '{}' -> '{}'",
                    raw_ingredient_name,
                    ingredient_name
                );

                matches.push(MeasurementMatch {
                    text: final_measurement_text,
                    ingredient_name,
                    line_number,
                    start_pos: current_pos + capture.start(),
                    end_pos: current_pos + capture.end(),
                });
            }
            current_pos += line.len() + 1; // +1 for newline character
        }

        info!("Found {} measurement matches in text", matches.len());
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
    /// let text = "2 cups flour\n1 tablespoon sugar\nsome salt\n3 sachets yeast\n6 oeufs\n4 pommes";
    /// let measurement_lines = detector.extract_measurement_lines(text);
    ///
    /// assert_eq!(measurement_lines.len(), 5);
    /// assert_eq!(measurement_lines[0], (0, "2 cups flour".to_string()));
    /// assert_eq!(measurement_lines[1], (1, "1 tablespoon sugar".to_string()));
    /// assert_eq!(measurement_lines[2], (3, "3 sachets yeast".to_string()));
    /// assert_eq!(measurement_lines[3], (4, "6 oeufs".to_string()));
    /// assert_eq!(measurement_lines[4], (5, "4 pommes".to_string()));
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
    /// assert!(detector.has_measurements("6 oeufs"));  // quantity-only ingredient
    /// assert!(detector.has_measurements("4 pommes")); // quantity-only ingredient
    /// assert!(!detector.has_measurements("some flour"));
    /// assert!(!detector.has_measurements("some eggs")); // plain text without quantity
    /// # Ok::<(), regex::Error>(())
    /// ```
    pub fn has_measurements(&self, text: &str) -> bool {
        let result = self.pattern.is_match(text);
        debug!(
            "Checking for measurements in text: '{}' -> {}",
            text, result
        );
        if result {
            trace!(
                "Pattern '{}' matched in text: '{}'",
                self.pattern.as_str(),
                text
            );
        }
        result
    }

    /// Check if a measurement match is quantity-only (no measurement unit)
    ///
    /// A match is considered quantity-only if it contains a space followed by a word
    /// that is not a recognized measurement unit.
    fn is_quantity_only_match(&self, measurement_text: &str) -> bool {
        // Split the text by space
        let parts: Vec<&str> = measurement_text.split_whitespace().collect();
        if parts.len() != 2 {
            return false; // Not in "number word" format
        }

        let word_part = parts[1].to_lowercase();

        // Check if the word part is exactly a measurement unit
        let measurement_units = [
            // Volume units
            "cup", "cups", "teaspoon", "teaspoons", "tsp", "tablespoon", "tablespoons", "tbsp",
            "pint", "pints", "quart", "quarts", "gallon", "gallons", "fluid", "fl",
            // Weight units
            "g", "gram", "grams", "gramme", "grammes", "kg", "kilogram", "kilograms", "kilogramme", "kilogrammes",
            "mg", "lb", "pound", "pounds", "oz", "ounce", "ounces",
            // Volume units (metric)
            "l", "liter", "liters", "litre", "litres", "ml", "milliliter", "milliliters", "millilitre", "millilitres",
            "cc", "cl", "dl", "cm3", "mm3", "cm²", "mm²",
            // Count units
            "slice", "slices", "can", "cans", "bottle", "bottles", "stick", "sticks",
            "packet", "packets", "pkg", "bag", "bags", "dash", "dashes", "pinch", "pinches",
            "drop", "drops", "cube", "cubes", "piece", "pieces", "handful", "handfuls",
            "bar", "bars", "sheet", "sheets", "serving", "servings", "portion", "portions",
            // French units
            "tasse", "tasses", "cuillère", "cuillères", "poignée", "poignées", "sachet", "sachets",
            "paquet", "paquets", "boîte", "boîtes", "conserve", "conserves", "tranche", "tranches",
            "morceau", "morceaux", "gousse", "gousses", "brin", "brins", "feuille", "feuilles", "bouquet", "bouquets",
        ];

        // Check if the word part is exactly a measurement unit
        for unit in &measurement_units {
            if word_part == *unit {
                return false; // The word is a measurement unit, so not quantity-only
            }
        }

        true // The word is not a measurement unit, so it's quantity-only
    }

    /// Post-process an ingredient name to clean it up
    ///
    /// This method applies various cleaning operations to extract clean ingredient names:
    /// - Removes common prepositions and articles
    /// - Trims whitespace and punctuation
    /// - Limits length to prevent overly long extractions
    /// - Handles French prepositions like "de", "d'", etc.
    ///
    /// # Arguments
    ///
    /// * `raw_name` - The raw ingredient name extracted from text
    ///
    /// # Returns
    ///
    /// A cleaned and processed ingredient name
    fn post_process_ingredient_name(&self, raw_name: &str) -> String {
        if !self.config.enable_ingredient_postprocessing || raw_name.trim().is_empty() {
            trace!("Post-processing disabled or empty name: '{}'", raw_name);
            return raw_name.trim().to_string();
        }

        let mut name = raw_name.trim().to_string();
        let original_name = name.clone();

        // Remove trailing punctuation
        name = name
            .trim_end_matches(|c: char| !c.is_alphanumeric() && c != ' ' && c != '-' && c != '\'')
            .to_string();

        // Common prepositions and articles to remove (English and French)
        let prefixes_to_remove = [
            // English
            "of ", "the ", "a ", "an ", // French
            "de ", "d'", "du ", "des ", "la ", "le ", "les ", "l'", "au ", "aux ", "un ", "une ",
        ];

        for prefix in &prefixes_to_remove {
            if name.to_lowercase().starts_with(prefix) {
                name = name[prefix.len()..].trim_start().to_string();
                debug!(
                    "Removed prefix '{}' from ingredient name: '{}' -> '{}'",
                    prefix.trim(),
                    original_name,
                    name
                );
                break; // Only remove one prefix
            }
        }

        // Limit length to prevent overly long extractions
        if name.len() > self.config.max_ingredient_length {
            let truncated = name[..self.config.max_ingredient_length].to_string();
            // Try to cut at word boundary
            if let Some(last_space) = truncated.rfind(' ') {
                name = truncated[..last_space].to_string();
            } else {
                name = truncated;
            }
            warn!(
                "Ingredient name truncated due to length limit ({} > {}): '{}' -> '{}'",
                original_name.len(),
                self.config.max_ingredient_length,
                original_name,
                name
            );
        }

        // Clean up multiple spaces
        name = name.split_whitespace().collect::<Vec<&str>>().join(" ");

        trace!(
            "Post-processed ingredient name: '{}' -> '{}'",
            original_name,
            name
        );
        name.trim().to_string()
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
    /// let text = "2 cups flour\n1 cup sugar\n500g butter\n6 oeufs\n4 pommes";
    /// let units = detector.get_unique_units(text);
    ///
    /// assert!(units.iter().any(|u| u.contains("cups")));
    /// assert!(units.iter().any(|u| u.contains("cup")));
    /// assert!(units.iter().any(|u| u.contains("g")));
    /// assert!(units.iter().any(|u| u.contains("6")));  // quantity-only measurement
    /// assert!(units.iter().any(|u| u.contains("4")));  // quantity-only measurement
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
        assert!(!detector.pattern.as_str().is_empty());
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

        // Test count measurements (excluding œufs which are ingredients, not measurements)
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

        // Test count-based measurements (excluding eggs which are ingredients, not measurements)
        assert!(detector.has_measurements("2 slices bread"));
        assert!(detector.has_measurements("1 can tomatoes"));
        assert!(detector.has_measurements("4 pieces chicken"));
        assert!(detector.has_measurements("3 sachets yeast"));
        assert!(detector.has_measurements("2 paquets pasta"));
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

        // Test French ingredient name extraction (with post-processing enabled by default)
        let matches = detector.find_measurements("250 g de farine\n1 litre de lait\n2 tranches de pain");

        assert_eq!(matches.len(), 3);

        assert_eq!(matches[0].text, "250 g");
        assert_eq!(matches[0].ingredient_name, "farine"); // "de " removed by post-processing

        assert_eq!(matches[1].text, "1 litre");
        assert_eq!(matches[1].ingredient_name, "lait"); // "de " removed by post-processing

        assert_eq!(matches[2].text, "2 tranches");
        assert_eq!(matches[2].ingredient_name, "pain"); // "de " removed by post-processing
    }

    #[test]
    fn test_multi_word_ingredient_names() {
        let detector = create_detector();

        // Test multi-word ingredient names
        let matches = detector.find_measurements(
            "2 cups all-purpose flour\n1 teaspoon baking powder\n500g unsalted butter",
        );

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

    #[test]
    fn test_regex_pattern_validation() {
        let detector = create_detector();

        // Test that the regex correctly identifies various measurement formats
        let test_cases = vec![
            // Basic volume measurements
            ("1 cup", true),
            ("2 cups", true),
            ("1.5 cups", true),
            ("0.25 cups", true),
            // Weight measurements
            ("500g", true),
            ("1.5kg", true),
            ("250 grams", true),
            ("2 pounds", true),
            // Volume measurements
            ("1 tablespoon", true),
            ("2 teaspoons", true),
            ("1 tsp", true),
            ("2 tbsp", true),
            ("500 ml", true),
            ("1 liter", true),
            // Count measurements (excluding eggs/œufs which are ingredients)
            ("2 slices", true),
            ("1 can", true),
            ("4 pieces", true),
            ("3 sachets", true),
            // French measurements
            ("2 tasses", true),
            ("1 cuillère à soupe", true),
            ("250 g", true),
            // Non-measurements (should not match)
            ("recipe", false),
            ("ingredients", false),
            ("flour", false),
            ("sugar", false),
            ("salt", false),
            ("", false),
            ("123", false), // Just a number, no unit
            ("abc", false),
            ("cupboard", false),      // Contains "cup" but not as measurement
            ("tablespoonful", false), // Contains "tablespoon" but not as measurement
        ];

        for (text, should_match) in test_cases {
            assert_eq!(
                detector.has_measurements(text),
                should_match,
                "Pattern validation failed for: '{}' (expected: {})",
                text,
                should_match
            );
        }
    }

    #[test]
    fn test_regex_capture_groups() {
        let detector = create_detector();

        // Test that the regex captures complete measurement units
        let test_text = "Mix 2 cups flour with 1 tbsp sugar and 500g butter";
        let matches = detector.find_measurements(test_text);

        assert_eq!(matches.len(), 3);

        // Verify each match captures the complete measurement
        assert_eq!(matches[0].text, "2 cups");
        assert_eq!(matches[1].text, "1 tbsp");
        assert_eq!(matches[2].text, "500g");

        // Verify positions are correct
        assert_eq!(matches[0].start_pos, 4); // "Mix 2" -> position after "Mix "
        assert_eq!(matches[0].end_pos, 10); // "Mix 2 cups" -> ends at position 10
    }

    #[test]
    fn test_regex_boundary_conditions() {
        let detector = create_detector();

        // Test word boundaries and edge cases
        let boundary_tests = vec![
            ("1cup", true),    // No space between number and unit (technically matches pattern)
            ("cup1", false),   // Unit before number
            ("1 cup.", true),  // Period after measurement
            ("(1 cup)", true), // Parentheses around measurement
            ("1 cup,", true),  // Comma after measurement
            ("1 cup;", true),  // Semicolon after measurement
            ("cup of flour", false), // "cup" without number
            ("cups", false),   // Just unit, no number
            ("1", false),      // Just number, no unit
        ];

        for (text, should_match) in boundary_tests {
            assert_eq!(
                detector.has_measurements(text),
                should_match,
                "Boundary test failed for: '{}' (expected: {})",
                text,
                should_match
            );
        }
    }

    #[test]
    fn test_regex_case_insensitivity() {
        let detector = create_detector();

        // Test that the regex is case insensitive
        let case_tests = vec![
            "2 CUPS flour",
            "2 Cups flour",
            "2 cups flour",
            "500G butter",
            "500g butter",
            "1 TBSP sugar",
            "1 tbsp sugar",
            "1 Tablespoon sugar",
        ];

        for text in case_tests {
            assert!(
                detector.has_measurements(text),
                "Case insensitivity test failed for: '{}'",
                text
            );
        }
    }

    #[test]
    fn test_regex_french_accents() {
        let detector = create_detector();

        // Test that French measurements with accents work correctly
        let french_tests = vec![
            "1 cuillère à café",
            "2 cuillères à soupe",
            "1 kilogramme",
            "2 grammes",
            "1 millilitre",
            "2 litres",
            "1 tranche",
            "2 morceaux",
            "1 boîte",
            "2 sachets",
        ];

        for text in french_tests {
            assert!(
                detector.has_measurements(text),
                "French accent test failed for: '{}'",
                text
            );
        }
    }

    #[test]
    fn test_ingredient_name_postprocessing() {
        let config = MeasurementConfig {
            enable_ingredient_postprocessing: true,
            max_ingredient_length: 50,
            ..Default::default()
        };
        let detector = MeasurementDetector::with_config(config).unwrap();

        // Test basic post-processing
        let matches =
            detector.find_measurements("2 cups of flour\n1 tablespoon sugar\n500g butter");

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].ingredient_name, "flour"); // "of " removed
        assert_eq!(matches[1].ingredient_name, "sugar");
        assert_eq!(matches[2].ingredient_name, "butter");
    }

    #[test]
    fn test_french_ingredient_postprocessing() {
        let config = MeasurementConfig {
            enable_ingredient_postprocessing: true,
            ..Default::default()
        };
        let detector = MeasurementDetector::with_config(config).unwrap();

        let matches =
            detector.find_measurements("250 g de farine\n1 litre du lait\n2 tasses d'eau");

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].ingredient_name, "farine"); // "de " removed
        assert_eq!(matches[1].ingredient_name, "lait"); // "du " removed
        assert_eq!(matches[2].ingredient_name, "eau"); // "d'" removed
    }

    #[test]
    fn test_ingredient_length_limit() {
        let config = MeasurementConfig {
            enable_ingredient_postprocessing: true,
            max_ingredient_length: 20,
            ..Default::default()
        };
        let detector = MeasurementDetector::with_config(config).unwrap();

        let matches = detector
            .find_measurements("2 cups of very-long-ingredient-name-that-should-be-truncated");

        assert_eq!(matches.len(), 1);
        assert!(matches[0].ingredient_name.len() <= 20);
        assert_eq!(matches[0].ingredient_name, "very-long-ingredient"); // "of " removed, then truncated at word boundary
    }

    #[test]
    fn test_postprocessing_disabled() {
        let config = MeasurementConfig {
            enable_ingredient_postprocessing: false,
            ..Default::default()
        };
        let detector = MeasurementDetector::with_config(config).unwrap();

        let matches = detector.find_measurements("2 cups of flour");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].ingredient_name, "of flour"); // No post-processing
    }

    #[test]
    fn test_quantity_only_ingredients() {
        let detector = create_detector();

        // Test quantity-only ingredients (no measurement units)
        assert!(detector.has_measurements("6 oeufs"));
        assert!(detector.has_measurements("2 œufs"));
        assert!(detector.has_measurements("4 pommes"));
        assert!(detector.has_measurements("3 eggs"));
        assert!(detector.has_measurements("5 apples"));

        // Test that regular measurements still work
        assert!(detector.has_measurements("2 cups flour"));
        assert!(detector.has_measurements("500g sugar"));

        // Test that plain numbers don't match
        assert!(!detector.has_measurements("123"));
        assert!(!detector.has_measurements("1"));

        // Test find_measurements for quantity-only ingredients
        let matches = detector.find_measurements("6 oeufs\n4 pommes");
        assert_eq!(matches.len(), 2);

        // For quantity-only ingredients, split into quantity and ingredient name
        assert_eq!(matches[0].text, "6");
        assert_eq!(matches[0].ingredient_name, "oeufs");

        assert_eq!(matches[1].text, "4");
        assert_eq!(matches[1].ingredient_name, "pommes");
    }
}
