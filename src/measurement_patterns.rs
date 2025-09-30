//! # Measurement Patterns Module
//!
//! This module contains regex patterns and constants used for measurement detection.

use lazy_static::lazy_static;
use regex::Regex;

// Default comprehensive regex pattern for measurement units (now supports quantity-only ingredients and fractions)
// Uses named capture groups: quantity, measurement, and ingredient
pub const DEFAULT_PATTERN: &str = r#"(?i)(?P<quantity>\d*\.?\d+|\d+/\d+|[½⅓⅔¼¾⅕⅖⅗⅘⅙⅚⅛⅜⅝⅞⅟])(?:\s*(?P<measurement>cup(?:s)?|teaspoon(?:s)?|tsp(?:\.?)|tablespoon(?:s)?|tbsp(?:\.?)|pint(?:s)?|quart(?:s)?|gallon(?:s)?|oz|ounce(?:s)?|lb(?:\.?)|pound(?:s)?|mg|gram(?:me)?s?|kilogram(?:me)?s?|kg|g|liter(?:s)?|litre(?:s)?|millilitre(?:s)?|ml|cm3|mm3|cm²|mm²|cl|dl|l|slice(?:s)?|can(?:s)?|bottle(?:s)?|stick(?:s)?|packet(?:s)?|pkg|bag(?:s)?|dash(?:es)?|pinch(?:es)?|drop(?:s)?|cube(?:s)?|piece(?:s)?|handful(?:s)?|bar(?:s)?|sheet(?:s)?|serving(?:s)?|portion(?:s)?|tasse(?:s)?|cuillère(?:s)?(?:\s+à\s+(?:café|soupe))?|poignée(?:s)?|sachet(?:s)?|paquet(?:s)?|boîte(?:s)?|conserve(?:s)?|tranche(?:s)?|morceau(?:x)?|gousse(?:s)?|brin(?:s)?|feuille(?:s)?|bouquet(?:s)?)|\s+(?P<ingredient>\w+))"#;

// Lazy static regex for default pattern to avoid recompilation
lazy_static! {
    pub static ref DEFAULT_REGEX: Regex =
        Regex::new(DEFAULT_PATTERN).expect("Default measurement pattern should be valid");
}
