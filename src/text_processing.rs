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
//! - **Fraction support**: Recognizes fractional quantities (e.g., "1/2 litre", "3/4 cup")
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

// Default comprehensive regex pattern for measurement units (now supports quantity-only ingredients and fractions)
const DEFAULT_PATTERN: &str = r#"(?i)(\d*\.?\d+|\d+/\d+|[½⅓⅔¼¾⅕⅖⅗⅘⅙⅚⅛⅜⅝⅞⅟])(?:\s*(?:cup(?:s)?|teaspoon(?:s)?|tsp(?:\.?)|tablespoon(?:s)?|tbsp(?:\.?)|pint(?:s)?|quart(?:s)?|gallon(?:s)?|oz|ounce(?:s)?|lb(?:\.?)|pound(?:s)?|mg|gram(?:me)?s?|kilogram(?:me)?s?|kg|g|liter(?:s)?|litre(?:s)?|millilitre(?:s)?|ml|cm3|mm3|cm²|mm²|cl|dl|l|slice(?:s)?|can(?:s)?|bottle(?:s)?|stick(?:s)?|packet(?:s)?|pkg|bag(?:s)?|dash(?:es)?|pinch(?:es)?|drop(?:s)?|cube(?:s)?|piece(?:s)?|handful(?:s)?|bar(?:s)?|sheet(?:s)?|serving(?:s)?|portion(?:s)?|tasse(?:s)?|cuillère(?:s)?(?:\s+à\s+(?:café|soupe))?|poignée(?:s)?|sachet(?:s)?|paquet(?:s)?|boîte(?:s)?|conserve(?:s)?|tranche(?:s)?|morceau(?:x)?|gousse(?:s)?|brin(?:s)?|feuille(?:s)?|bouquet(?:s)?)|\s+\w+)"#;

// Lazy static regex for default pattern to avoid recompilation
lazy_static! {
    static ref DEFAULT_REGEX: Regex =
        Regex::new(DEFAULT_PATTERN).expect("Default measurement pattern should be valid");
}

/// Measurement detector using regex patterns for English and French units
pub struct MeasurementDetector {
    /// Compiled regex pattern for detecting measurements
    pattern: Regex,
    /// Configuration options
    config: MeasurementConfig,
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn with_config(config: MeasurementConfig) -> Result<Self, regex::Error> {
        let pattern = if let Some(custom_pattern) = &config.custom_pattern {
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
    /// let text = "2 cups flour\n1/2 cup sugar\nsome salt\n3 sachets yeast\n6 oeufs\n4 pommes";
    /// let measurement_lines = detector.extract_measurement_lines(text);
    ///
    /// assert_eq!(measurement_lines.len(), 5);
    /// assert_eq!(measurement_lines[0], (0, "2 cups flour".to_string()));
    /// assert_eq!(measurement_lines[1], (1, "1/2 cup sugar".to_string()));
    /// assert_eq!(measurement_lines[2], (3, "3 sachets yeast".to_string()));
    /// assert_eq!(measurement_lines[3], (4, "6 oeufs".to_string()));
    /// assert_eq!(measurement_lines[4], (5, "4 pommes".to_string()));
    /// # Ok::<(), regex::Error>(())
    /// ```
    #[allow(dead_code)]
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
    /// assert!(detector.has_measurements("1/2 cup sugar"));  // fraction support
    /// assert!(detector.has_measurements("6 oeufs"));  // quantity-only ingredient
    /// assert!(detector.has_measurements("4 pommes")); // quantity-only ingredient
    /// assert!(!detector.has_measurements("some flour"));
    /// assert!(!detector.has_measurements("some eggs")); // plain text without quantity
    /// # Ok::<(), regex::Error>(())
    /// ```
    #[allow(dead_code)]
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
            "cup",
            "cups",
            "teaspoon",
            "teaspoons",
            "tsp",
            "tablespoon",
            "tablespoons",
            "tbsp",
            "pint",
            "pints",
            "quart",
            "quarts",
            "gallon",
            "gallons",
            "fluid",
            "fl",
            // Weight units
            "g",
            "gram",
            "grams",
            "gramme",
            "grammes",
            "kg",
            "kilogram",
            "kilograms",
            "kilogramme",
            "kilogrammes",
            "mg",
            "lb",
            "pound",
            "pounds",
            "oz",
            "ounce",
            "ounces",
            // Volume units (metric)
            "l",
            "liter",
            "liters",
            "litre",
            "litres",
            "ml",
            "milliliter",
            "milliliters",
            "millilitre",
            "millilitres",
            "cc",
            "cl",
            "dl",
            "cm3",
            "mm3",
            "cm²",
            "mm²",
            // Count units
            "slice",
            "slices",
            "can",
            "cans",
            "bottle",
            "bottles",
            "stick",
            "sticks",
            "packet",
            "packets",
            "pkg",
            "bag",
            "bags",
            "dash",
            "dashes",
            "pinch",
            "pinches",
            "drop",
            "drops",
            "cube",
            "cubes",
            "piece",
            "pieces",
            "handful",
            "handfuls",
            "bar",
            "bars",
            "sheet",
            "sheets",
            "serving",
            "servings",
            "portion",
            "portions",
            // French units
            "tasse",
            "tasses",
            "cuillère",
            "cuillères",
            "poignée",
            "poignées",
            "sachet",
            "sachets",
            "paquet",
            "paquets",
            "boîte",
            "boîtes",
            "conserve",
            "conserves",
            "tranche",
            "tranches",
            "morceau",
            "morceaux",
            "gousse",
            "gousses",
            "brin",
            "brins",
            "feuille",
            "feuilles",
            "bouquet",
            "bouquets",
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
    /// let text = "2 cups flour\n1/2 cup sugar\n500g butter\n6 oeufs\n4 pommes";
    /// let units = detector.get_unique_units(text);
    ///
    /// assert!(units.iter().any(|u| u.contains("cups")));
    /// assert!(units.iter().any(|u| u.contains("cup")));
    /// assert!(units.iter().any(|u| u.contains("1/2"))); // fraction support
    /// assert!(units.iter().any(|u| u.contains("g")));
    /// assert!(units.iter().any(|u| u.contains("6")));  // quantity-only measurement
    /// assert!(units.iter().any(|u| u.contains("4")));  // quantity-only measurement
    /// # Ok::<(), regex::Error>(())
    /// ```
    #[allow(dead_code)]
    pub fn get_unique_units(&self, text: &str) -> HashSet<String> {
        self.pattern
            .find_iter(text)
            .map(|m| m.as_str().to_lowercase())
            .collect()
    }
}

impl MeasurementDetector {
    /// Get the regex pattern as a string (for testing purposes)
    pub fn pattern_str(&self) -> &str {
        self.pattern.as_str()
    }
}


