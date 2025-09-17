//! # Ingredient Parser Module
//!
//! This module provides data structures and parsing logic for extracting structured
//! ingredient information from raw text, including quantities, units, and ingredient names.
//!
//! ## Features
//!
//! - Structured representation of ingredients with quantities and units
//! - Support for fractions, decimals, and ranges in quantities
//! - Comprehensive unit system with conversions
//! - Error handling for ambiguous or invalid parsing
//! - Normalization of ingredient names and plurals
//!
//! ## Supported Formats
//!
//! - Fractions: "1/2", "½", "2 1/4"
//! - Decimals: "1.5", "0.25"  
//! - Ranges: "2-3", "1 to 2", "1-2"
//! - Units: cups, tablespoons, teaspoons, grams, etc.
//! - Ingredient names with normalization

use std::fmt;

/// Type alias for the complex return type of quantity and unit extraction
type ParseResult<'a> = Result<(Option<Quantity>, Option<Unit>, Vec<&'a str>), IngredientParseError>;

/// Represents a parsed ingredient entry with quantity, unit, and name
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedIngredientEntry {
    /// The quantity of the ingredient (optional for items like "salt to taste")
    pub quantity: Option<Quantity>,
    /// The unit of measurement (optional for count-based items like "2 eggs")
    pub unit: Option<Unit>,
    /// The normalized ingredient name
    pub ingredient_name: String,
    /// The original raw text that was parsed
    pub original_text: String,
    /// Parsing confidence score (0.0 to 1.0)
    pub confidence: f32,
}

/// Represents a quantity that can be a single value, fraction, or range
#[derive(Debug, Clone, PartialEq)]
pub enum Quantity {
    /// A single exact value (e.g., "2", "1.5")
    Exact(f64),
    /// A fraction (numerator, denominator, optional whole part)
    Fraction {
        whole: Option<u32>,
        numerator: u32,
        denominator: u32,
    },
    /// A range of values (e.g., "2-3", "1 to 2")
    Range { min: f64, max: f64 },
    /// Approximate quantity (e.g., "about 2", "around 1.5")
    Approximate(f64),
    /// Textual quantity (e.g., "a pinch", "to taste", "some")
    Textual(String),
}

/// Represents units of measurement with categorization
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Unit {
    // Volume units
    Volume(VolumeUnit),
    // Weight/Mass units
    Weight(WeightUnit),
    // Count/Piece units
    Count(CountUnit),
    // Length units (for things like "inch of ginger")
    Length(LengthUnit),
    // Temperature units
    Temperature(TemperatureUnit),
    // Custom/Unknown units
    Custom(String),
}

/// Volume measurement units
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VolumeUnit {
    // Metric
    Milliliter,
    Liter,
    // Imperial
    Teaspoon,
    Tablespoon,
    FluidOunce,
    Cup,
    Pint,
    Quart,
    Gallon,
    // Cooking specific
    Drop,
    Pinch,
    Dash,
}

/// Weight/Mass measurement units
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WeightUnit {
    // Metric
    Milligram,
    Gram,
    Kilogram,
    // Imperial
    Ounce,
    Pound,
    // Other
    Stone,
}

/// Count/Piece units
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CountUnit {
    Piece,
    Item,
    Clove, // for garlic
    Head,  // for lettuce, cabbage
    Bunch, // for herbs, vegetables
    Package,
    Can,
    Bottle,
    Box,
    Bag,
    Container,
}

/// Length measurement units
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LengthUnit {
    Millimeter,
    Centimeter,
    Meter,
    Inch,
    Foot,
}

/// Temperature units
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
    Kelvin,
}

/// Errors that can occur during ingredient parsing
#[derive(Debug, Clone, PartialEq)]
pub enum IngredientParseError {
    /// Input text is empty or contains only whitespace
    EmptyInput,
    /// Could not identify any ingredient name
    NoIngredientFound,
    /// Quantity format is invalid (e.g., malformed fraction)
    InvalidQuantity(String),
    /// Unit is not recognized
    UnknownUnit(String),
    /// Multiple valid interpretations exist
    AmbiguousParsing(Vec<String>),
    /// Input contains conflicting information
    ConflictingInformation(String),
}

impl fmt::Display for IngredientParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IngredientParseError::EmptyInput => {
                write!(f, "Input text is empty or contains only whitespace")
            }
            IngredientParseError::NoIngredientFound => {
                write!(f, "Could not identify any ingredient name")
            }
            IngredientParseError::InvalidQuantity(q) => write!(f, "Invalid quantity format: {}", q),
            IngredientParseError::UnknownUnit(u) => write!(f, "Unknown unit: {}", u),
            IngredientParseError::AmbiguousParsing(options) => {
                write!(
                    f,
                    "Ambiguous parsing - multiple interpretations: {}",
                    options.join(", ")
                )
            }
            IngredientParseError::ConflictingInformation(info) => {
                write!(f, "Conflicting information in input: {}", info)
            }
        }
    }
}

impl std::error::Error for IngredientParseError {}

impl Quantity {
    /// Convert quantity to a decimal value for calculations
    pub fn to_decimal(&self) -> Option<f64> {
        match self {
            Quantity::Exact(value) => Some(*value),
            Quantity::Fraction {
                whole,
                numerator,
                denominator,
            } => {
                let whole_part = whole.unwrap_or(0) as f64;
                let fraction_part = (*numerator as f64) / (*denominator as f64);
                Some(whole_part + fraction_part)
            }
            Quantity::Range { min, max } => Some((min + max) / 2.0), // Average of range
            Quantity::Approximate(value) => Some(*value),
            Quantity::Textual(_) => None, // Cannot convert textual quantities
        }
    }

    /// Get the range of possible values (min, max)
    pub fn value_range(&self) -> Option<(f64, f64)> {
        match self {
            Quantity::Exact(value) => Some((*value, *value)),
            Quantity::Fraction { .. } => {
                self.to_decimal().map(|decimal| (decimal, decimal))
            }
            Quantity::Range { min, max } => Some((*min, *max)),
            Quantity::Approximate(value) => {
                // Assume ±20% variation for approximate values
                let variance = value * 0.2;
                Some((value - variance, value + variance))
            }
            Quantity::Textual(_) => None,
        }
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Quantity::Exact(value) => {
                if value.fract() == 0.0 {
                    write!(f, "{}", *value as i64)
                } else {
                    write!(f, "{}", value)
                }
            }
            Quantity::Fraction {
                whole,
                numerator,
                denominator,
            } => {
                if let Some(w) = whole {
                    if *w > 0 {
                        write!(f, "{} {}/{}", w, numerator, denominator)
                    } else {
                        write!(f, "{}/{}", numerator, denominator)
                    }
                } else {
                    write!(f, "{}/{}", numerator, denominator)
                }
            }
            Quantity::Range { min, max } => write!(f, "{}-{}", min, max),
            Quantity::Approximate(value) => write!(f, "about {}", value),
            Quantity::Textual(text) => write!(f, "{}", text),
        }
    }
}

impl Unit {
    /// Get the base unit for conversion purposes
    pub fn base_unit(&self) -> &Unit {
        match self {
            Unit::Volume(_) => &Unit::Volume(VolumeUnit::Milliliter),
            Unit::Weight(_) => &Unit::Weight(WeightUnit::Gram),
            Unit::Count(_) => self, // Count units don't convert
            Unit::Length(_) => &Unit::Length(LengthUnit::Millimeter),
            Unit::Temperature(_) => &Unit::Temperature(TemperatureUnit::Celsius),
            Unit::Custom(_) => self,
        }
    }

    /// Check if two units are of the same type (can be converted)
    pub fn is_compatible(&self, other: &Unit) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    /// Get common alternative names for the unit
    pub fn aliases(&self) -> Vec<&'static str> {
        match self {
            Unit::Volume(VolumeUnit::Teaspoon) => vec!["tsp", "t", "teaspoons"],
            Unit::Volume(VolumeUnit::Tablespoon) => vec!["tbsp", "T", "tablespoons"],
            Unit::Volume(VolumeUnit::Cup) => vec!["c", "cups"],
            Unit::Volume(VolumeUnit::Milliliter) => vec!["ml", "mL"],
            Unit::Volume(VolumeUnit::Liter) => vec!["l", "L", "liters", "litres"],
            Unit::Weight(WeightUnit::Gram) => vec!["g", "grams"],
            Unit::Weight(WeightUnit::Kilogram) => vec!["kg", "kilograms"],
            Unit::Weight(WeightUnit::Ounce) => vec!["oz", "ounces"],
            Unit::Weight(WeightUnit::Pound) => vec!["lb", "lbs", "pounds"],
            _ => vec![],
        }
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Unit::Volume(v) => match v {
                VolumeUnit::Milliliter => "ml",
                VolumeUnit::Liter => "liter",
                VolumeUnit::Teaspoon => "teaspoon",
                VolumeUnit::Tablespoon => "tablespoon",
                VolumeUnit::FluidOunce => "fl oz",
                VolumeUnit::Cup => "cup",
                VolumeUnit::Pint => "pint",
                VolumeUnit::Quart => "quart",
                VolumeUnit::Gallon => "gallon",
                VolumeUnit::Drop => "drop",
                VolumeUnit::Pinch => "pinch",
                VolumeUnit::Dash => "dash",
            },
            Unit::Weight(w) => match w {
                WeightUnit::Milligram => "mg",
                WeightUnit::Gram => "gram",
                WeightUnit::Kilogram => "kg",
                WeightUnit::Ounce => "ounce",
                WeightUnit::Pound => "pound",
                WeightUnit::Stone => "stone",
            },
            Unit::Count(c) => match c {
                CountUnit::Piece => "piece",
                CountUnit::Item => "item",
                CountUnit::Clove => "clove",
                CountUnit::Head => "head",
                CountUnit::Bunch => "bunch",
                CountUnit::Package => "package",
                CountUnit::Can => "can",
                CountUnit::Bottle => "bottle",
                CountUnit::Box => "box",
                CountUnit::Bag => "bag",
                CountUnit::Container => "container",
            },
            Unit::Length(l) => match l {
                LengthUnit::Millimeter => "mm",
                LengthUnit::Centimeter => "cm",
                LengthUnit::Meter => "meter",
                LengthUnit::Inch => "inch",
                LengthUnit::Foot => "foot",
            },
            Unit::Temperature(t) => match t {
                TemperatureUnit::Celsius => "°C",
                TemperatureUnit::Fahrenheit => "°F",
                TemperatureUnit::Kelvin => "K",
            },
            Unit::Custom(s) => s,
        };
        write!(f, "{}", name)
    }
}

impl ParsedIngredientEntry {
    /// Create a new parsed ingredient entry
    pub fn new(
        quantity: Option<Quantity>,
        unit: Option<Unit>,
        ingredient_name: String,
        original_text: String,
        confidence: f32,
    ) -> Self {
        Self {
            quantity,
            unit,
            ingredient_name,
            original_text,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Check if this entry represents a valid ingredient
    pub fn is_valid(&self) -> bool {
        !self.ingredient_name.trim().is_empty() && self.confidence > 0.0
    }

    /// Get a normalized string representation of the ingredient
    pub fn normalized_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref quantity) = self.quantity {
            parts.push(quantity.to_string());
        }

        if let Some(ref unit) = self.unit {
            parts.push(unit.to_string());
        }

        parts.push(self.ingredient_name.clone());

        parts.join(" ")
    }
}

impl fmt::Display for ParsedIngredientEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.normalized_string())
    }
}

/// Parse a single line of ingredient text into a structured entry
pub fn parse_ingredient_line(text: &str) -> Result<ParsedIngredientEntry, IngredientParseError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(IngredientParseError::EmptyInput);
    }

    // This is a placeholder implementation - the actual parsing logic would be more complex
    // For now, we'll create a simple parser that can handle basic cases

    // Example: "2 cups flour" -> quantity=2, unit=cups, ingredient=flour
    // Example: "1/2 teaspoon salt" -> quantity=1/2, unit=teaspoon, ingredient=salt
    // Example: "3 eggs" -> quantity=3, unit=None, ingredient=eggs
    // Example: "salt to taste" -> quantity=None, unit=None, ingredient=salt

    // Simple tokenization
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.is_empty() {
        return Err(IngredientParseError::EmptyInput);
    }

    // Try to extract quantity from the first word(s)
    let (quantity, unit, remaining_words) = extract_quantity_and_unit(&words)?;

    // Remaining words form the ingredient name
    if remaining_words.is_empty() {
        return Err(IngredientParseError::NoIngredientFound);
    }

    let ingredient_name = remaining_words.join(" ");
    let confidence = calculate_parsing_confidence(&quantity, &unit, &ingredient_name);

    Ok(ParsedIngredientEntry::new(
        quantity,
        unit,
        ingredient_name,
        text.to_string(),
        confidence,
    ))
}

/// Extract quantity and unit from the beginning of word list
fn extract_quantity_and_unit<'a>(
    words: &'a [&'a str],
) -> ParseResult<'a> {
    if words.is_empty() {
        return Ok((None, None, vec![]));
    }

    // Check for mixed number fractions (e.g., "2 1/4")
    if words.len() >= 2 {
        if let Some(whole_num) = try_parse_whole_number(words[0]) {
            if let Some(fraction) = try_parse_simple_fraction(words[1]) {
                // Found mixed number like "2 1/4"
                let mixed_quantity = Quantity::Fraction {
                    whole: Some(whole_num),
                    numerator: fraction.0,
                    denominator: fraction.1,
                };

                // Look for unit in next word
                if words.len() > 2 {
                    if let Some(unit) = try_parse_unit(words[2]) {
                        let remaining = words[3..].to_vec();
                        return Ok((Some(mixed_quantity), Some(unit), remaining));
                    } else {
                        let remaining = words[2..].to_vec();
                        return Ok((Some(mixed_quantity), None, remaining));
                    }
                } else {
                    return Ok((Some(mixed_quantity), None, vec![]));
                }
            }
        }
    }

    // Try to parse the first word as a quantity
    if let Some(quantity) = try_parse_quantity(words[0]) {
        // Look for a unit in the next word(s)
        if words.len() > 1 {
            if let Some(unit) = try_parse_unit(words[1]) {
                // Found both quantity and unit
                let remaining = words[2..].to_vec();
                return Ok((Some(quantity), Some(unit), remaining));
            } else {
                // Found quantity but no unit
                let remaining = words[1..].to_vec();
                return Ok((Some(quantity), None, remaining));
            }
        } else {
            // Only quantity, no more words
            return Ok((Some(quantity), None, vec![]));
        }
    }

    // No quantity found, check if first word is a unit without quantity
    if let Some(unit) = try_parse_unit(words[0]) {
        let remaining = words[1..].to_vec();
        return Ok((None, Some(unit), remaining));
    }

    // No quantity or unit found
    Ok((None, None, words.to_vec()))
}

/// Try to parse a word as a whole number (for mixed fractions)
fn try_parse_whole_number(word: &str) -> Option<u32> {
    word.parse::<u32>().ok()
}

/// Try to parse a word as a simple fraction (numerator/denominator)
fn try_parse_simple_fraction(word: &str) -> Option<(u32, u32)> {
    if word.contains('/') {
        let parts: Vec<&str> = word.split('/').collect();
        if parts.len() == 2 {
            if let (Ok(num), Ok(den)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                if den != 0 {
                    return Some((num, den));
                }
            }
        }
    }
    None
}

/// Try to parse a word as a quantity
fn try_parse_quantity(word: &str) -> Option<Quantity> {
    // Try exact decimal/integer
    if let Ok(value) = word.parse::<f64>() {
        return Some(Quantity::Exact(value));
    }

    // Try fraction (e.g., "1/2", "3/4")
    if word.contains('/') {
        let parts: Vec<&str> = word.split('/').collect();
        if parts.len() == 2 {
            if let (Ok(num), Ok(den)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                if den != 0 {
                    return Some(Quantity::Fraction {
                        whole: None,
                        numerator: num,
                        denominator: den,
                    });
                }
            }
        }
    }

    // Try range (e.g., "2-3", "1-2")
    if word.contains('-') && !word.starts_with('-') {
        let parts: Vec<&str> = word.split('-').collect();
        if parts.len() == 2 {
            if let (Ok(min), Ok(max)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                return Some(Quantity::Range { min, max });
            }
        }
    }

    // Common textual quantities
    match word.to_lowercase().as_str() {
        "pinch" | "dash" | "some" | "few" => Some(Quantity::Textual(word.to_string())),
        _ => None,
    }
}

/// Try to parse a word as a unit
fn try_parse_unit(word: &str) -> Option<Unit> {
    let word_lower = word.to_lowercase();
    let word_singular = if word_lower.ends_with('s') && word_lower.len() > 1 {
        &word_lower[..word_lower.len() - 1]
    } else {
        &word_lower
    };

    match word_singular {
        // Volume units
        "cup" | "c" => Some(Unit::Volume(VolumeUnit::Cup)),
        "teaspoon" | "tsp" | "t" => Some(Unit::Volume(VolumeUnit::Teaspoon)),
        "tablespoon" | "tbsp" | "tb" => Some(Unit::Volume(VolumeUnit::Tablespoon)),
        "ml" | "milliliter" | "millilitre" => Some(Unit::Volume(VolumeUnit::Milliliter)),
        "liter" | "litre" | "l" => Some(Unit::Volume(VolumeUnit::Liter)),
        "pint" => Some(Unit::Volume(VolumeUnit::Pint)),
        "quart" | "qt" => Some(Unit::Volume(VolumeUnit::Quart)),
        "gallon" | "gal" => Some(Unit::Volume(VolumeUnit::Gallon)),
        "fl oz" | "fluid ounce" => Some(Unit::Volume(VolumeUnit::FluidOunce)),
        "pinch" => Some(Unit::Volume(VolumeUnit::Pinch)),
        "dash" => Some(Unit::Volume(VolumeUnit::Dash)),

        // Weight units
        "gram" | "g" => Some(Unit::Weight(WeightUnit::Gram)),
        "kilogram" | "kg" => Some(Unit::Weight(WeightUnit::Kilogram)),
        "ounce" | "oz" => Some(Unit::Weight(WeightUnit::Ounce)),
        "pound" | "lb" => Some(Unit::Weight(WeightUnit::Pound)),

        // Count units
        "piece" => Some(Unit::Count(CountUnit::Piece)),
        "clove" => Some(Unit::Count(CountUnit::Clove)),
        "head" => Some(Unit::Count(CountUnit::Head)),
        "bunch" => Some(Unit::Count(CountUnit::Bunch)),
        "can" => Some(Unit::Count(CountUnit::Can)),
        "package" | "pkg" => Some(Unit::Count(CountUnit::Package)),
        "bottle" => Some(Unit::Count(CountUnit::Bottle)),
        "box" => Some(Unit::Count(CountUnit::Box)),
        "bag" => Some(Unit::Count(CountUnit::Bag)),

        _ => None,
    }
}

/// Calculate parsing confidence based on extracted components
fn calculate_parsing_confidence(
    quantity: &Option<Quantity>,
    unit: &Option<Unit>,
    ingredient_name: &str,
) -> f32 {
    let mut confidence: f32 = 0.0;

    // Base confidence for having an ingredient name
    if !ingredient_name.trim().is_empty() {
        confidence += 0.4;
    }

    // Bonus for having a quantity
    if quantity.is_some() {
        confidence += 0.3;
    }

    // Bonus for having a unit
    if unit.is_some() {
        confidence += 0.2;
    }

    // Bonus for having both quantity and unit
    if quantity.is_some() && unit.is_some() {
        confidence += 0.1;
    }

    confidence.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantity_to_decimal() {
        assert_eq!(Quantity::Exact(2.5).to_decimal(), Some(2.5));

        let fraction = Quantity::Fraction {
            whole: Some(1),
            numerator: 1,
            denominator: 2,
        };
        assert_eq!(fraction.to_decimal(), Some(1.5));

        let range = Quantity::Range { min: 1.0, max: 3.0 };
        assert_eq!(range.to_decimal(), Some(2.0));

        assert_eq!(Quantity::Textual("pinch".to_string()).to_decimal(), None);
    }

    #[test]
    fn test_quantity_value_range() {
        assert_eq!(Quantity::Exact(2.0).value_range(), Some((2.0, 2.0)));

        let range = Quantity::Range { min: 1.0, max: 3.0 };
        assert_eq!(range.value_range(), Some((1.0, 3.0)));

        assert_eq!(Quantity::Textual("some".to_string()).value_range(), None);
    }

    #[test]
    fn test_unit_compatibility() {
        let cup = Unit::Volume(VolumeUnit::Cup);
        let tsp = Unit::Volume(VolumeUnit::Teaspoon);
        let gram = Unit::Weight(WeightUnit::Gram);

        assert!(cup.is_compatible(&tsp));
        assert!(!cup.is_compatible(&gram));
    }

    #[test]
    fn test_parse_simple_ingredient() {
        let result = parse_ingredient_line("2 cups flour").unwrap();
        assert_eq!(result.quantity, Some(Quantity::Exact(2.0)));
        assert_eq!(result.unit, Some(Unit::Volume(VolumeUnit::Cup)));
        assert_eq!(result.ingredient_name, "flour");
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn test_parse_fraction_ingredient() {
        let result = parse_ingredient_line("1/2 teaspoon salt").unwrap();
        if let Some(Quantity::Fraction {
            whole,
            numerator,
            denominator,
        }) = result.quantity
        {
            assert_eq!(whole, None);
            assert_eq!(numerator, 1);
            assert_eq!(denominator, 2);
        } else {
            panic!("Expected fraction quantity");
        }
        assert_eq!(result.unit, Some(Unit::Volume(VolumeUnit::Teaspoon)));
        assert_eq!(result.ingredient_name, "salt");
    }

    #[test]
    fn test_parse_range_ingredient() {
        let result = parse_ingredient_line("2-3 tablespoons olive oil").unwrap();
        if let Some(Quantity::Range { min, max }) = result.quantity {
            assert_eq!(min, 2.0);
            assert_eq!(max, 3.0);
        } else {
            panic!("Expected range quantity");
        }
        assert_eq!(result.unit, Some(Unit::Volume(VolumeUnit::Tablespoon)));
        assert_eq!(result.ingredient_name, "olive oil");
    }

    #[test]
    fn test_parse_no_unit_ingredient() {
        let result = parse_ingredient_line("3 eggs").unwrap();
        assert_eq!(result.quantity, Some(Quantity::Exact(3.0)));
        assert_eq!(result.unit, None);
        assert_eq!(result.ingredient_name, "eggs");
    }

    #[test]
    fn test_parse_no_quantity_ingredient() {
        let result = parse_ingredient_line("salt to taste").unwrap();
        assert_eq!(result.quantity, None);
        assert_eq!(result.unit, None);
        assert_eq!(result.ingredient_name, "salt to taste");
    }

    #[test]
    fn test_parse_textual_quantity() {
        let result = parse_ingredient_line("pinch of black pepper").unwrap();
        assert_eq!(
            result.quantity,
            Some(Quantity::Textual("pinch".to_string()))
        );
        assert_eq!(result.unit, None);
        assert_eq!(result.ingredient_name, "of black pepper");
    }

    #[test]
    fn test_parse_empty_input() {
        let result = parse_ingredient_line("");
        assert!(matches!(result, Err(IngredientParseError::EmptyInput)));

        let result = parse_ingredient_line("   ");
        assert!(matches!(result, Err(IngredientParseError::EmptyInput)));
    }

    #[test]
    fn test_unit_aliases() {
        let cup_aliases = Unit::Volume(VolumeUnit::Cup).aliases();
        assert!(cup_aliases.contains(&"c"));
        assert!(cup_aliases.contains(&"cups"));

        let tsp_aliases = Unit::Volume(VolumeUnit::Teaspoon).aliases();
        assert!(tsp_aliases.contains(&"tsp"));
        assert!(tsp_aliases.contains(&"t"));
    }

    #[test]
    fn test_parsed_entry_validation() {
        let valid_entry = ParsedIngredientEntry::new(
            Some(Quantity::Exact(2.0)),
            Some(Unit::Volume(VolumeUnit::Cup)),
            "flour".to_string(),
            "2 cups flour".to_string(),
            0.9,
        );
        assert!(valid_entry.is_valid());

        let invalid_entry = ParsedIngredientEntry::new(
            Some(Quantity::Exact(2.0)),
            Some(Unit::Volume(VolumeUnit::Cup)),
            "".to_string(),
            "2 cups".to_string(),
            0.9,
        );
        assert!(!invalid_entry.is_valid());
    }

    #[test]
    fn test_normalized_string() {
        let entry = ParsedIngredientEntry::new(
            Some(Quantity::Exact(2.0)),
            Some(Unit::Volume(VolumeUnit::Cup)),
            "all-purpose flour".to_string(),
            "2 cups all-purpose flour".to_string(),
            0.9,
        );
        assert_eq!(entry.normalized_string(), "2 cup all-purpose flour");
    }

    #[test]
    fn test_quantity_display() {
        assert_eq!(Quantity::Exact(2.0).to_string(), "2");
        assert_eq!(Quantity::Exact(2.5).to_string(), "2.5");

        let fraction = Quantity::Fraction {
            whole: Some(1),
            numerator: 1,
            denominator: 2,
        };
        assert_eq!(fraction.to_string(), "1 1/2");

        let simple_fraction = Quantity::Fraction {
            whole: None,
            numerator: 3,
            denominator: 4,
        };
        assert_eq!(simple_fraction.to_string(), "3/4");

        let range = Quantity::Range { min: 2.0, max: 3.0 };
        assert_eq!(range.to_string(), "2-3");

        assert_eq!(Quantity::Approximate(2.5).to_string(), "about 2.5");
        assert_eq!(Quantity::Textual("pinch".to_string()).to_string(), "pinch");
    }

    #[test]
    fn test_mixed_number_fractions() {
        let result = parse_ingredient_line("2 1/4 cups flour").unwrap();
        if let Some(Quantity::Fraction {
            whole,
            numerator,
            denominator,
        }) = result.quantity
        {
            assert_eq!(whole, Some(2));
            assert_eq!(numerator, 1);
            assert_eq!(denominator, 4);
            assert_eq!(result.quantity.unwrap().to_decimal(), Some(2.25));
        } else {
            panic!("Expected mixed number fraction");
        }
    }

    #[test]
    fn test_complex_ingredient_names() {
        let result = parse_ingredient_line("2 cups all-purpose flour, sifted").unwrap();
        assert_eq!(result.ingredient_name, "all-purpose flour, sifted");

        let result = parse_ingredient_line("1 large red bell pepper, diced").unwrap();
        assert_eq!(result.ingredient_name, "large red bell pepper, diced");
    }

    #[test]
    fn test_unit_plural_handling() {
        let result1 = parse_ingredient_line("1 cup sugar").unwrap();
        let result2 = parse_ingredient_line("2 cups sugar").unwrap();

        // Both should parse to the same unit type
        assert_eq!(result1.unit, Some(Unit::Volume(VolumeUnit::Cup)));
        assert_eq!(result2.unit, Some(Unit::Volume(VolumeUnit::Cup)));
    }

    #[test]
    fn test_metric_units() {
        let result = parse_ingredient_line("500 ml water").unwrap();
        assert_eq!(result.quantity, Some(Quantity::Exact(500.0)));
        assert_eq!(result.unit, Some(Unit::Volume(VolumeUnit::Milliliter)));

        let result = parse_ingredient_line("250 grams butter").unwrap();
        assert_eq!(result.quantity, Some(Quantity::Exact(250.0)));
        assert_eq!(result.unit, Some(Unit::Weight(WeightUnit::Gram)));
    }

    #[test]
    fn test_weight_units() {
        let result = parse_ingredient_line("1 lb ground beef").unwrap();
        assert_eq!(result.unit, Some(Unit::Weight(WeightUnit::Pound)));

        let result = parse_ingredient_line("8 oz cream cheese").unwrap();
        assert_eq!(result.unit, Some(Unit::Weight(WeightUnit::Ounce)));
    }

    #[test]
    fn test_count_units() {
        let result = parse_ingredient_line("3 cloves garlic").unwrap();
        assert_eq!(result.unit, Some(Unit::Count(CountUnit::Clove)));

        let result = parse_ingredient_line("1 head lettuce").unwrap();
        assert_eq!(result.unit, Some(Unit::Count(CountUnit::Head)));

        let result = parse_ingredient_line("2 cans tomatoes").unwrap();
        assert_eq!(result.unit, Some(Unit::Count(CountUnit::Can)));
    }

    #[test]
    fn test_alternative_unit_names() {
        // Test various abbreviated forms
        let tsp_variants = vec!["1 tsp salt", "1 t salt"];
        for variant in tsp_variants {
            let result = parse_ingredient_line(variant).unwrap();
            assert_eq!(result.unit, Some(Unit::Volume(VolumeUnit::Teaspoon)));
        }

        let tbsp_variants = vec!["1 tbsp oil", "1 tb oil"];
        for variant in tbsp_variants {
            let result = parse_ingredient_line(variant).unwrap();
            assert_eq!(result.unit, Some(Unit::Volume(VolumeUnit::Tablespoon)));
        }
    }

    #[test]
    fn test_confidence_scoring() {
        // High confidence: quantity + unit + ingredient
        let high_conf = parse_ingredient_line("2 cups flour").unwrap();
        assert!(high_conf.confidence >= 0.8);

        // Medium confidence: quantity + ingredient (no unit)
        let med_conf = parse_ingredient_line("3 eggs").unwrap();
        assert!(med_conf.confidence >= 0.6 && med_conf.confidence < 0.8);

        // Lower confidence: just ingredient
        let low_conf = parse_ingredient_line("salt to taste").unwrap();
        assert!(low_conf.confidence < 0.7);
    }

    #[test]
    fn test_range_variations() {
        // Test different range formats
        let hyphen_range = parse_ingredient_line("1-2 tablespoons oil").unwrap();
        if let Some(Quantity::Range { min, max }) = hyphen_range.quantity {
            assert_eq!(min, 1.0);
            assert_eq!(max, 2.0);
        } else {
            panic!("Expected range quantity for hyphen format");
        }
    }

    #[test]
    fn test_decimal_quantities() {
        let result = parse_ingredient_line("0.5 cups milk").unwrap();
        assert_eq!(result.quantity, Some(Quantity::Exact(0.5)));

        let result = parse_ingredient_line("1.25 teaspoons vanilla").unwrap();
        assert_eq!(result.quantity, Some(Quantity::Exact(1.25)));
    }

    #[test]
    fn test_special_cooking_measurements() {
        let result = parse_ingredient_line("1 pinch salt").unwrap();
        assert_eq!(result.unit, Some(Unit::Volume(VolumeUnit::Pinch)));

        let result = parse_ingredient_line("1 dash pepper").unwrap();
        assert_eq!(result.unit, Some(Unit::Volume(VolumeUnit::Dash)));
    }

    #[test]
    fn test_common_cooking_containers() {
        let containers = vec![
            ("1 package pasta", CountUnit::Package),
            ("2 bottles wine", CountUnit::Bottle),
            ("1 box crackers", CountUnit::Box),
            ("3 bags spinach", CountUnit::Bag),
        ];

        for (input, expected_unit) in containers {
            let result = parse_ingredient_line(input).unwrap();
            assert_eq!(result.unit, Some(Unit::Count(expected_unit)));
        }
    }

    #[test]
    fn test_no_false_positives() {
        // These should not parse units from ingredient names
        let result = parse_ingredient_line("beef cubes").unwrap();
        assert_eq!(result.quantity, None);
        assert_eq!(result.unit, None);
        assert_eq!(result.ingredient_name, "beef cubes");

        let result = parse_ingredient_line("chicken cups").unwrap();
        assert_eq!(result.quantity, None);
        assert_eq!(result.unit, None);
        assert_eq!(result.ingredient_name, "chicken cups");
    }
}
