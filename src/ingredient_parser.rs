//! # Ingredient Parser
//!
//! This module provides functionality to parse raw OCR text into structured ingredient data.
//! It handles various formats including fractions, ranges, units, and ambiguous quantities.
//!
//! ## Features
//!
//! - Parse common ingredient formats from recipe text
//! - Handle fractions (1/2, 2 1/4, etc.)
//! - Recognize ranges (2-3, 1 to 2, etc.)
//! - Extract units and modifiers
//! - Deal with ambiguous quantities ("to taste", "some", etc.)
//! - Support multiple languages (English and French)
//!
//! ## Usage
//!
//! ```rust
//! use ingredients::ingredient_parser::parse_ingredient_list;
//!
//! let text = "2 cups flour\n1 tbsp salt\n1/2 tsp pepper";
//! let parsed = parse_ingredient_list(text);
//!
//! for ingredient in parsed.ingredients {
//!     println!("{}", ingredient);
//! }
//! ```

use crate::ingredient_model::{Ingredient, IngredientList, Quantity, QuantityType, Unit};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Regex patterns for parsing different quantity formats
static QUANTITY_PATTERNS: LazyLock<QuantityPatterns> = LazyLock::new(QuantityPatterns::new);

/// Common unit mappings and their variations
static UNIT_MAPPINGS: LazyLock<HashMap<&'static str, Unit>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    
    // Volume units
    map.insert("tsp", Unit::Teaspoons);
    map.insert("teaspoon", Unit::Teaspoons);
    map.insert("teaspoons", Unit::Teaspoons);
    map.insert("tbsp", Unit::Tablespoons);
    map.insert("tablespoon", Unit::Tablespoons);
    map.insert("tablespoons", Unit::Tablespoons);
    map.insert("cup", Unit::Cups);
    map.insert("cups", Unit::Cups);
    map.insert("c", Unit::Cups);
    map.insert("fl oz", Unit::FluidOunces);
    map.insert("fluid ounce", Unit::FluidOunces);
    map.insert("fluid ounces", Unit::FluidOunces);
    map.insert("pint", Unit::Pints);
    map.insert("pints", Unit::Pints);
    map.insert("pt", Unit::Pints);
    map.insert("quart", Unit::Quarts);
    map.insert("quarts", Unit::Quarts);
    map.insert("qt", Unit::Quarts);
    map.insert("gallon", Unit::Gallons);
    map.insert("gallons", Unit::Gallons);
    map.insert("gal", Unit::Gallons);
    map.insert("ml", Unit::Milliliters);
    map.insert("milliliter", Unit::Milliliters);
    map.insert("milliliters", Unit::Milliliters);
    map.insert("l", Unit::Liters);
    map.insert("liter", Unit::Liters);
    map.insert("liters", Unit::Liters);
    map.insert("litre", Unit::Liters);
    map.insert("litres", Unit::Liters);
    
    // Weight units
    map.insert("oz", Unit::Ounces);
    map.insert("ounce", Unit::Ounces);
    map.insert("ounces", Unit::Ounces);
    map.insert("lb", Unit::Pounds);
    map.insert("lbs", Unit::Pounds);
    map.insert("pound", Unit::Pounds);
    map.insert("pounds", Unit::Pounds);
    map.insert("g", Unit::Grams);
    map.insert("gram", Unit::Grams);
    map.insert("grams", Unit::Grams);
    map.insert("kg", Unit::Kilograms);
    map.insert("kilogram", Unit::Kilograms);
    map.insert("kilograms", Unit::Kilograms);
    
    // Count units
    map.insert("piece", Unit::Pieces);
    map.insert("pieces", Unit::Pieces);
    map.insert("item", Unit::Pieces);
    map.insert("items", Unit::Pieces);
    map.insert("dozen", Unit::Dozen);
    map.insert("doz", Unit::Dozen);
    
    // Specialized units
    map.insert("pinch", Unit::Pinches);
    map.insert("pinches", Unit::Pinches);
    map.insert("dash", Unit::Dashes);
    map.insert("dashes", Unit::Dashes);
    map.insert("clove", Unit::Cloves);
    map.insert("cloves", Unit::Cloves);
    map.insert("package", Unit::Packages);
    map.insert("packages", Unit::Packages);
    map.insert("pkg", Unit::Packages);
    map.insert("can", Unit::Cans);
    map.insert("cans", Unit::Cans);
    map.insert("bottle", Unit::Bottles);
    map.insert("bottles", Unit::Bottles);
    
    // French units
    map.insert("cuillère à café", Unit::Teaspoons);
    map.insert("cuillères à café", Unit::Teaspoons);
    map.insert("cac", Unit::Teaspoons);
    map.insert("cuillère à soupe", Unit::Tablespoons);
    map.insert("cuillères à soupe", Unit::Tablespoons);
    map.insert("cas", Unit::Tablespoons);
    map.insert("tasse", Unit::Cups);
    map.insert("tasses", Unit::Cups);
    map.insert("litre", Unit::Liters);
    map.insert("litres", Unit::Liters);
    map.insert("gramme", Unit::Grams);
    map.insert("grammes", Unit::Grams);
    map.insert("kilogramme", Unit::Kilograms);
    map.insert("kilogrammes", Unit::Kilograms);
    map.insert("pièce", Unit::Pieces);
    map.insert("pièces", Unit::Pieces);
    map.insert("gousse", Unit::Cloves);
    map.insert("gousses", Unit::Cloves);
    map.insert("boîte", Unit::Cans);
    map.insert("boîtes", Unit::Cans);
    map.insert("bouteille", Unit::Bottles);
    map.insert("bouteilles", Unit::Bottles);
    
    map
});

/// Ambiguous quantity indicators
static AMBIGUOUS_INDICATORS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "to taste", "taste", "some", "a little", "a bit", "handful", "bunch",
        "several", "few", "many", "enough", "as needed", "optional",
        "à goût", "au goût", "un peu", "quelques", "plusieurs", "suffisamment",
        "selon le goût", "facultatif", "optionnel",
    ]
});

/// Compiled regex patterns for parsing
struct QuantityPatterns {
    /// Matches exact amounts: "2", "1.5", "0.25"
    exact: Regex,
    /// Matches fractions: "1/2", "2 1/4", "1⁄2"
    fraction: Regex,
    /// Matches ranges: "2-3", "1 to 2", "2 or 3"
    range: Regex,
    /// Matches the full ingredient line
    ingredient_line: Regex,
}

impl QuantityPatterns {
    fn new() -> Self {
        Self {
            exact: Regex::new(r"^\d+(?:\.\d+)?$").unwrap(),
            fraction: Regex::new(r"^(?:(\d+)\s+)?(\d+)[⁄/](\d+)$").unwrap(),
            range: Regex::new(r"^(\d+(?:\.\d+)?)\s*[-–—to|or]\s*(\d+(?:\.\d+)?)$").unwrap(),
            ingredient_line: Regex::new(
                r"^(?:(?P<qty>[\d\s⁄/.,\-–—]+(?:to|or|à)?[\d\s⁄/.,]*)\s*)?(?P<unit>[a-zA-Zà-ÿ\s\.]+?)?\s+(?P<ingredient>.+?)(?:\s*\((?P<modifier>.+?)\))?$"
            ).unwrap(),
        }
    }
}

/// Parse a full ingredient list from OCR text
pub fn parse_ingredient_list(text: &str) -> IngredientList {
    let mut list = IngredientList::new(text.to_string());
    
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        match parse_ingredient_line(line) {
            Ok(ingredient) => list.add_ingredient(ingredient),
            Err(_) => list.add_unparsed_line(line.to_string()),
        }
    }
    
    list
}

/// Parse a single ingredient line
pub fn parse_ingredient_line(line: &str) -> Result<Ingredient, ParseError> {
    let line = line.trim();
    
    // Try to match the ingredient line pattern
    if let Some(captures) = QUANTITY_PATTERNS.ingredient_line.captures(line) {
        let qty_str = captures.name("qty").map(|m| m.as_str().trim());
        let unit_str = captures.name("unit").map(|m| m.as_str().trim());
        let ingredient_name = captures.name("ingredient")
            .map(|m| m.as_str().trim())
            .unwrap_or("")
            .to_string();
        let modifier = captures.name("modifier").map(|m| m.as_str().trim().to_string());
        
        if ingredient_name.is_empty() {
            return Err(ParseError::NoIngredientName);
        }
        
        let mut ingredient = Ingredient::new(&ingredient_name);
        
        if let Some(modifier) = modifier {
            ingredient = ingredient.with_modifier(&modifier);
        }
        
        // Parse quantity if present
        if let (Some(qty_str), Some(unit_str)) = (qty_str, unit_str) {
            if !qty_str.is_empty() && !unit_str.is_empty() {
                let unit = parse_unit(unit_str)?;
                let quantity = parse_quantity(qty_str, unit)?;
                ingredient = ingredient.with_quantity(quantity);
            }
        }
        
        // Check for ambiguous quantities in the ingredient name
        let lower_ingredient = ingredient_name.to_lowercase();
        for &indicator in AMBIGUOUS_INDICATORS.iter() {
            if lower_ingredient.contains(indicator) {
                let ambiguous_qty = Quantity::ambiguous(indicator, Unit::Unknown("".to_string()));
                ingredient = ingredient.with_quantity(ambiguous_qty).with_confidence(0.6);
                break;
            }
        }
        
        Ok(ingredient)
    } else {
        // Fallback: treat the entire line as an ingredient name
        let ingredient = Ingredient::new(line).with_confidence(0.5);
        Ok(ingredient)
    }
}

/// Parse a quantity string into a Quantity object
fn parse_quantity(qty_str: &str, unit: Unit) -> Result<Quantity, ParseError> {
    let qty_str = qty_str.trim();
    
    // Check for ambiguous indicators first
    let lower_qty = qty_str.to_lowercase();
    for &indicator in AMBIGUOUS_INDICATORS.iter() {
        if lower_qty.contains(indicator) {
            return Ok(Quantity::ambiguous(qty_str, unit));
        }
    }
    
    // Try range pattern
    if let Some(captures) = QUANTITY_PATTERNS.range.captures(qty_str) {
        let min: f64 = captures[1].parse().map_err(|_| ParseError::InvalidNumber)?;
        let max: f64 = captures[2].parse().map_err(|_| ParseError::InvalidNumber)?;
        return Ok(Quantity::range(min, max, unit));
    }
    
    // Try fraction pattern
    if let Some(captures) = QUANTITY_PATTERNS.fraction.captures(qty_str) {
        let whole = captures.get(1).and_then(|m| m.as_str().parse().ok());
        let numerator: u32 = captures[2].parse().map_err(|_| ParseError::InvalidNumber)?;
        let denominator: u32 = captures[3].parse().map_err(|_| ParseError::InvalidNumber)?;
        
        if denominator == 0 {
            return Err(ParseError::DivisionByZero);
        }
        
        return Ok(Quantity::fraction(whole, numerator, denominator, unit));
    }
    
    // Try exact number
    if let Ok(amount) = qty_str.parse::<f64>() {
        return Ok(Quantity::exact(amount, unit));
    }
    
    // If nothing else works, treat as ambiguous
    Ok(Quantity::ambiguous(qty_str, unit))
}

/// Parse a unit string into a Unit enum
fn parse_unit(unit_str: &str) -> Result<Unit, ParseError> {
    let unit_str = unit_str.trim().to_lowercase();
    
    // Direct lookup in the mappings
    if let Some(unit) = UNIT_MAPPINGS.get(unit_str.as_str()) {
        return Ok(unit.clone());
    }
    
    // Try without pluralization
    let singular = if unit_str.ends_with('s') && unit_str.len() > 1 {
        &unit_str[..unit_str.len() - 1]
    } else {
        &unit_str
    };
    
    if let Some(unit) = UNIT_MAPPINGS.get(singular) {
        return Ok(unit.clone());
    }
    
    // Return unknown unit
    Ok(Unit::Unknown(unit_str))
}

/// Errors that can occur during parsing
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    NoIngredientName,
    InvalidNumber,
    DivisionByZero,
    UnknownUnit(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::NoIngredientName => write!(f, "No ingredient name found"),
            ParseError::InvalidNumber => write!(f, "Invalid number format"),
            ParseError::DivisionByZero => write!(f, "Division by zero in fraction"),
            ParseError::UnknownUnit(unit) => write!(f, "Unknown unit: {}", unit),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_ingredient() {
        let result = parse_ingredient_line("2 cups flour").unwrap();
        assert_eq!(result.name, "flour");
        assert!(result.has_quantity());
        assert_eq!(result.estimated_amount(), Some(2.0));
    }

    #[test]
    fn test_parse_fraction_ingredient() {
        let result = parse_ingredient_line("1/2 cup sugar").unwrap();
        assert_eq!(result.name, "sugar");
        assert_eq!(result.estimated_amount(), Some(0.5));
        
        let result = parse_ingredient_line("2 1/4 cups butter").unwrap();
        assert_eq!(result.name, "butter");
        assert_eq!(result.estimated_amount(), Some(2.25));
    }

    #[test]
    fn test_parse_range_ingredient() {
        let result = parse_ingredient_line("2-3 tbsp olive oil").unwrap();
        assert_eq!(result.name, "olive oil");
        assert_eq!(result.estimated_amount(), Some(2.5));
        assert!(result.quantity.unwrap().is_range());
    }

    #[test]
    fn test_parse_with_modifier() {
        let result = parse_ingredient_line("2 cups flour (all-purpose)").unwrap();
        assert_eq!(result.name, "flour");
        assert_eq!(result.modifier, Some("all-purpose".to_string()));
    }

    #[test]
    fn test_parse_ambiguous_quantity() {
        let result = parse_ingredient_line("salt to taste").unwrap();
        assert_eq!(result.name, "salt to taste");
        assert!(result.quantity.is_some());
        assert!(result.quantity.unwrap().is_ambiguous());
    }

    #[test]
    fn test_parse_no_quantity() {
        let result = parse_ingredient_line("eggs").unwrap();
        assert_eq!(result.name, "eggs");
        assert!(!result.has_quantity());
    }

    #[test]
    fn test_parse_ingredient_list() {
        let text = "2 cups flour\n1 tbsp salt\n1/2 tsp pepper\nsome mysterious ingredient";
        let list = parse_ingredient_list(text);
        
        assert_eq!(list.parsed_count(), 4); // All lines can be parsed as ingredients
        assert_eq!(list.ingredients[0].name, "flour");
        assert_eq!(list.ingredients[1].name, "salt");
        assert_eq!(list.ingredients[2].name, "pepper");
        assert_eq!(list.ingredients[3].name, "some mysterious ingredient");
    }

    #[test]
    fn test_parse_french_units() {
        let result = parse_ingredient_line("250 g farine").unwrap();
        assert_eq!(result.name, "farine");
        assert!(result.has_quantity());
        
        let result = parse_ingredient_line("2 cas huile d'olive").unwrap();
        assert_eq!(result.name, "huile d'olive");
        assert!(result.has_quantity());
    }

    #[test]
    fn test_unit_parsing() {
        assert_eq!(parse_unit("cups").unwrap(), Unit::Cups);
        assert_eq!(parse_unit("cup").unwrap(), Unit::Cups);
        assert_eq!(parse_unit("c").unwrap(), Unit::Cups);
        assert_eq!(parse_unit("tsp").unwrap(), Unit::Teaspoons);
        assert_eq!(parse_unit("tablespoons").unwrap(), Unit::Tablespoons);
        
        // Test unknown unit
        if let Unit::Unknown(name) = parse_unit("unknown_unit").unwrap() {
            assert_eq!(name, "unknown_unit");
        } else {
            panic!("Expected unknown unit");
        }
    }

    #[test]
    fn test_quantity_parsing() {
        // Exact
        let qty = parse_quantity("2.5", Unit::Cups).unwrap();
        assert_eq!(qty.estimated_value(), Some(2.5));
        
        // Fraction
        let qty = parse_quantity("1/2", Unit::Cups).unwrap();
        assert_eq!(qty.estimated_value(), Some(0.5));
        
        // Range
        let qty = parse_quantity("2-3", Unit::Cups).unwrap();
        assert_eq!(qty.estimated_value(), Some(2.5));
        assert!(qty.is_range());
        
        // Ambiguous
        let qty = parse_quantity("to taste", Unit::Cups).unwrap();
        assert!(qty.is_ambiguous());
    }
}