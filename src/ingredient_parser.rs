//! # Ingredient Parser Module
//!
//! This module provides functionality to extract and parse ingredient lines from OCR-processed text.
//! It identifies lines that follow the pattern: quantity + measurement (optional) + ingredient name.
//!
//! ## Features
//!
//! - Parse quantity in various formats (integers, decimals, fractions)
//! - Recognize common measurement units (cups, tablespoons, grams, etc.)
//! - Extract ingredient names
//! - Filter out lines that don't match the expected pattern
//! - Handle multi-line OCR text input
//!
//! ## Example Usage
//!
//! ```rust
//! use ingredients::ingredient_parser::extract_ingredients;
//!
//! let ocr_text = "1 cup sugar\n2 eggs\n100 g flour\nSalt";
//! let ingredients = extract_ingredients(ocr_text);
//! // Returns parsed ingredients for "1 cup sugar", "2 eggs", "100 g flour"
//! // Skips "Salt" as it doesn't match the pattern
//! ```

use log::{debug, info};
use std::collections::HashSet;

/// Represents a parsed ingredient with quantity, optional measurement, and name
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedIngredient {
    /// Original line from OCR text
    pub original_line: String,
    /// Parsed quantity (e.g., "1", "2.5", "1/2")
    pub quantity: String,
    /// Optional measurement unit (e.g., "cup", "tbsp", "g")
    pub measurement: Option<String>,
    /// Ingredient name (remaining text after quantity and measurement)
    pub ingredient_name: String,
}

/// Extract ingredient lines from OCR text that match the pattern: quantity + measurement (optional) + ingredient name
///
/// # Arguments
///
/// * `ocr_text` - Multi-line text extracted from OCR processing
///
/// # Returns
///
/// Returns a vector of `ParsedIngredient` structs containing structured ingredient data.
/// Lines that don't match the expected pattern are ignored.
///
/// # Examples
///
/// ```rust
/// use ingredients::ingredient_parser::extract_ingredients;
///
/// let text = "1 cup sugar\n2 eggs\n100 g flour\nSalt\n1/2 tsp vanilla";
/// let ingredients = extract_ingredients(text);
///
/// assert_eq!(ingredients.len(), 4);
/// assert_eq!(ingredients[0].quantity, "1");
/// assert_eq!(ingredients[0].measurement, Some("cup".to_string()));
/// assert_eq!(ingredients[0].ingredient_name, "sugar");
/// ```
pub fn extract_ingredients(ocr_text: &str) -> Vec<ParsedIngredient> {
    let lines: Vec<&str> = ocr_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    info!("Processing {} lines for ingredient extraction", lines.len());

    let mut ingredients = Vec::new();
    
    for line in lines {
        if let Some(ingredient) = parse_ingredient_line(line) {
            debug!("Successfully parsed ingredient: {:?}", ingredient);
            ingredients.push(ingredient);
        } else {
            debug!("Line '{}' does not match ingredient pattern", line);
        }
    }

    info!("Extracted {} ingredients from OCR text", ingredients.len());
    ingredients
}

/// Parse a single line to extract ingredient information
///
/// # Arguments
///
/// * `line` - Single line of text to parse
///
/// # Returns
///
/// Returns `Some(ParsedIngredient)` if the line matches the expected pattern,
/// or `None` if it doesn't match.
fn parse_ingredient_line(line: &str) -> Option<ParsedIngredient> {
    // Regular expression to match ingredient patterns
    // Supports: quantity + optional measurement + ingredient name
    let re = regex::Regex::new(
        r"^(?P<quantity>\d+(?:\.\d+)?(?:/\d+)?|\d+/\d+)\s*(?P<measurement>[a-zA-Z]+)?\s+(?P<ingredient>.+)$"
    ).ok()?;

    if let Some(captures) = re.captures(line) {
        let quantity = captures.name("quantity")?.as_str().to_string();
        let measurement = captures.name("measurement").map(|m| m.as_str().to_string());
        let ingredient_name = captures.name("ingredient")?.as_str().trim().to_string();

        // Validate that the measurement is a known unit (if present)
        if let Some(ref measure) = measurement {
            if !is_valid_measurement_unit(measure) {
                debug!("Unknown measurement unit '{}' in line '{}'", measure, line);
                return None;
            }
        }

        // Ensure ingredient name is not empty
        if ingredient_name.is_empty() {
            return None;
        }

        Some(ParsedIngredient {
            original_line: line.to_string(),
            quantity,
            measurement,
            ingredient_name,
        })
    } else {
        None
    }
}

/// Check if a string represents a valid measurement unit
///
/// # Arguments
///
/// * `unit` - The measurement unit to validate
///
/// # Returns
///
/// Returns `true` if the unit is recognized, `false` otherwise.
fn is_valid_measurement_unit(unit: &str) -> bool {
    // Create a set of common measurement units
    let valid_units: HashSet<&str> = [
        // Volume measurements
        "cup", "cups", "c",
        "tablespoon", "tablespoons", "tbsp", "tbs", "T",
        "teaspoon", "teaspoons", "tsp", "t",
        "fluid ounce", "fluid ounces", "fl oz", "floz",
        "pint", "pints", "pt",
        "quart", "quarts", "qt",
        "gallon", "gallons", "gal",
        "liter", "liters", "l", "L",
        "milliliter", "milliliters", "ml", "mL",
        
        // Weight measurements
        "gram", "grams", "g",
        "kilogram", "kilograms", "kg",
        "ounce", "ounces", "oz",
        "pound", "pounds", "lb", "lbs",
        
        // Length measurements (for ingredients like pasta)
        "inch", "inches", "in",
        "centimeter", "centimeters", "cm",
        
        // Count-based measurements
        "piece", "pieces", "pc", "pcs",
        "slice", "slices",
        "clove", "cloves",
        "head", "heads",
        
        // Other common units
        "can", "cans",
        "package", "packages", "pkg",
        "bottle", "bottles",
        "jar", "jars",
        "box", "boxes",
    ].iter().cloned().collect();

    // Check both the exact unit and lowercase version
    valid_units.contains(unit) || valid_units.contains(&unit.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ingredients_basic() {
        let text = "1 cup sugar\n2 eggs\n100 g flour";
        let ingredients = extract_ingredients(text);
        
        assert_eq!(ingredients.len(), 3);
        
        assert_eq!(ingredients[0].quantity, "1");
        assert_eq!(ingredients[0].measurement, Some("cup".to_string()));
        assert_eq!(ingredients[0].ingredient_name, "sugar");
        
        assert_eq!(ingredients[1].quantity, "2");
        assert_eq!(ingredients[1].measurement, None);
        assert_eq!(ingredients[1].ingredient_name, "eggs");
        
        assert_eq!(ingredients[2].quantity, "100");
        assert_eq!(ingredients[2].measurement, Some("g".to_string()));
        assert_eq!(ingredients[2].ingredient_name, "flour");
    }

    #[test]
    fn test_extract_ingredients_with_fractions() {
        let text = "1/2 cup milk\n1.5 tbsp vanilla\n2/3 tsp salt";
        let ingredients = extract_ingredients(text);
        
        assert_eq!(ingredients.len(), 3);
        
        assert_eq!(ingredients[0].quantity, "1/2");
        assert_eq!(ingredients[0].measurement, Some("cup".to_string()));
        assert_eq!(ingredients[0].ingredient_name, "milk");
        
        assert_eq!(ingredients[1].quantity, "1.5");
        assert_eq!(ingredients[1].measurement, Some("tbsp".to_string()));
        assert_eq!(ingredients[1].ingredient_name, "vanilla");
        
        assert_eq!(ingredients[2].quantity, "2/3");
        assert_eq!(ingredients[2].measurement, Some("tsp".to_string()));
        assert_eq!(ingredients[2].ingredient_name, "salt");
    }

    #[test]
    fn test_extract_ingredients_skip_invalid_lines() {
        let text = "1 cup sugar\nSalt\nMix well\n2 eggs\nBake for 30 minutes";
        let ingredients = extract_ingredients(text);
        
        assert_eq!(ingredients.len(), 2);
        assert_eq!(ingredients[0].ingredient_name, "sugar");
        assert_eq!(ingredients[1].ingredient_name, "eggs");
    }

    #[test]
    fn test_extract_ingredients_no_measurement() {
        let text = "2 eggs\n3 bananas\n1 onion";
        let ingredients = extract_ingredients(text);
        
        assert_eq!(ingredients.len(), 3);
        
        for ingredient in &ingredients {
            assert!(ingredient.measurement.is_none());
        }
        
        assert_eq!(ingredients[0].ingredient_name, "eggs");
        assert_eq!(ingredients[1].ingredient_name, "bananas");
        assert_eq!(ingredients[2].ingredient_name, "onion");
    }

    #[test]
    fn test_extract_ingredients_complex_names() {
        let text = "1 cup all-purpose flour\n2 tbsp olive oil, extra virgin\n1/2 tsp black pepper, freshly ground";
        let ingredients = extract_ingredients(text);
        
        assert_eq!(ingredients.len(), 3);
        assert_eq!(ingredients[0].ingredient_name, "all-purpose flour");
        assert_eq!(ingredients[1].ingredient_name, "olive oil, extra virgin");
        assert_eq!(ingredients[2].ingredient_name, "black pepper, freshly ground");
    }

    #[test]
    fn test_parse_ingredient_line_valid() {
        let line = "1 cup sugar";
        let ingredient = parse_ingredient_line(line).unwrap();
        
        assert_eq!(ingredient.quantity, "1");
        assert_eq!(ingredient.measurement, Some("cup".to_string()));
        assert_eq!(ingredient.ingredient_name, "sugar");
        assert_eq!(ingredient.original_line, "1 cup sugar");
    }

    #[test]
    fn test_parse_ingredient_line_no_measurement() {
        let line = "2 eggs";
        let ingredient = parse_ingredient_line(line).unwrap();
        
        assert_eq!(ingredient.quantity, "2");
        assert_eq!(ingredient.measurement, None);
        assert_eq!(ingredient.ingredient_name, "eggs");
    }

    #[test]
    fn test_parse_ingredient_line_invalid() {
        assert!(parse_ingredient_line("Salt").is_none());
        assert!(parse_ingredient_line("Mix well").is_none());
        assert!(parse_ingredient_line("Bake for 30 minutes").is_none());
        assert!(parse_ingredient_line("").is_none());
    }

    #[test]
    fn test_is_valid_measurement_unit() {
        // Test common valid units
        assert!(is_valid_measurement_unit("cup"));
        assert!(is_valid_measurement_unit("tbsp"));
        assert!(is_valid_measurement_unit("g"));
        assert!(is_valid_measurement_unit("oz"));
        assert!(is_valid_measurement_unit("kg"));
        
        // Test case insensitivity
        assert!(is_valid_measurement_unit("CUP"));
        assert!(is_valid_measurement_unit("Tbsp"));
        
        // Test invalid units
        assert!(!is_valid_measurement_unit("xyz"));
        assert!(!is_valid_measurement_unit("invalid"));
    }

    #[test]
    fn test_extract_ingredients_empty_input() {
        let text = "";
        let ingredients = extract_ingredients(text);
        assert_eq!(ingredients.len(), 0);
    }

    #[test]
    fn test_extract_ingredients_whitespace_handling() {
        let text = "  1 cup sugar  \n\n  2   eggs  \n  ";
        let ingredients = extract_ingredients(text);
        
        assert_eq!(ingredients.len(), 2);
        assert_eq!(ingredients[0].ingredient_name, "sugar");
        assert_eq!(ingredients[1].ingredient_name, "eggs");
    }
}