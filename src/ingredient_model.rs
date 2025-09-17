//! # Ingredient and Quantity Data Model
//!
//! This module defines data structures for representing ingredients and their quantities
//! as extracted from text. It handles various formats including fractions, ranges, units,
//! plurals, and ambiguous cases.
//!
//! ## Core Concepts
//!
//! - **Ingredient**: A food item with an optional quantity
//! - **Quantity**: A measurement that can be numeric, fractional, or a range
//! - **Unit**: Measurement unit (cups, tablespoons, grams, etc.)
//! - **Modifier**: Additional descriptors (chopped, diced, fresh, etc.)
//!
//! ## Usage
//!
//! ```rust
//! use ingredients::ingredient_model::{Ingredient, Quantity, Unit};
//!
//! // Simple ingredient with quantity
//! let flour = Ingredient::new("flour")
//!     .with_quantity(Quantity::exact(2.0, Unit::Cups));
//!
//! // Complex ingredient with range and modifier
//! let onions = Ingredient::new("onions")
//!     .with_quantity(Quantity::range(2.0, 3.0, Unit::Pieces))
//!     .with_modifier("diced");
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a parsed ingredient with optional quantity and modifiers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ingredient {
    /// The name of the ingredient (e.g., "flour", "onions", "olive oil")
    pub name: String,
    
    /// Optional quantity measurement
    pub quantity: Option<Quantity>,
    
    /// Optional preparation/description modifiers (e.g., "diced", "fresh", "extra virgin")
    pub modifier: Option<String>,
    
    /// Optional additional notes or uncertainty markers
    pub notes: Option<String>,
    
    /// Confidence level in the parsing (0.0 to 1.0)
    pub confidence: f32,
}

/// Represents a quantity measurement with support for various formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    /// The type of quantity measurement
    pub measurement: QuantityType,
    
    /// The unit of measurement
    pub unit: Unit,
    
    /// Whether the quantity is approximate (e.g., "about 2 cups")
    pub is_approximate: bool,
}

/// Different types of quantity measurements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuantityType {
    /// Exact amount (e.g., "2 cups")
    Exact(f64),
    
    /// Fractional amount (e.g., "1/2 cup", "2 1/4 tbsp")
    Fraction {
        /// Whole number part (optional)
        whole: Option<u32>,
        /// Numerator of the fraction
        numerator: u32,
        /// Denominator of the fraction
        denominator: u32,
    },
    
    /// Range of amounts (e.g., "2-3 cups", "1 to 2 tablespoons")
    Range {
        /// Minimum amount
        min: f64,
        /// Maximum amount
        max: f64,
    },
    
    /// Ambiguous or unclear quantity (e.g., "some", "a little", "to taste")
    Ambiguous(String),
}

/// Measurement units with normalization support
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Unit {
    // Volume units
    /// Teaspoons
    Teaspoons,
    /// Tablespoons
    Tablespoons,
    /// Fluid ounces
    FluidOunces,
    /// Cups
    Cups,
    /// Pints
    Pints,
    /// Quarts
    Quarts,
    /// Gallons
    Gallons,
    /// Milliliters
    Milliliters,
    /// Liters
    Liters,
    
    // Weight units
    /// Ounces
    Ounces,
    /// Pounds
    Pounds,
    /// Grams
    Grams,
    /// Kilograms
    Kilograms,
    
    // Count/piece units
    /// Individual pieces/items
    Pieces,
    /// Dozen
    Dozen,
    
    // Specialized units
    /// Pinches (very small amounts)
    Pinches,
    /// Dashes (small amounts)
    Dashes,
    /// Cloves (for garlic)
    Cloves,
    /// Packages/containers
    Packages,
    /// Cans
    Cans,
    /// Bottles
    Bottles,
    
    /// Unknown or unspecified unit
    Unknown(String),
}

/// Represents a collection of parsed ingredients from a recipe or ingredient list
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IngredientList {
    /// List of parsed ingredients
    pub ingredients: Vec<Ingredient>,
    
    /// Original raw text that was parsed
    pub original_text: String,
    
    /// Overall confidence in the parsing
    pub overall_confidence: f32,
    
    /// Any unparsed or problematic lines
    pub unparsed_lines: Vec<String>,
}

impl Ingredient {
    /// Create a new ingredient with just a name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            quantity: None,
            modifier: None,
            notes: None,
            confidence: 1.0,
        }
    }
    
    /// Add a quantity to this ingredient
    pub fn with_quantity(mut self, quantity: Quantity) -> Self {
        self.quantity = Some(quantity);
        self
    }
    
    /// Add a modifier to this ingredient
    pub fn with_modifier(mut self, modifier: &str) -> Self {
        self.modifier = Some(modifier.to_string());
        self
    }
    
    /// Add notes to this ingredient
    pub fn with_notes(mut self, notes: &str) -> Self {
        self.notes = Some(notes.to_string());
        self
    }
    
    /// Set the confidence level
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
    
    /// Check if this ingredient has a measurable quantity
    pub fn has_quantity(&self) -> bool {
        self.quantity.is_some()
    }
    
    /// Get the estimated amount in a normalized unit (if possible)
    pub fn estimated_amount(&self) -> Option<f64> {
        self.quantity.as_ref()?.estimated_value()
    }
}

impl Quantity {
    /// Create an exact quantity
    pub fn exact(amount: f64, unit: Unit) -> Self {
        Self {
            measurement: QuantityType::Exact(amount),
            unit,
            is_approximate: false,
        }
    }
    
    /// Create a fractional quantity
    pub fn fraction(whole: Option<u32>, numerator: u32, denominator: u32, unit: Unit) -> Self {
        Self {
            measurement: QuantityType::Fraction {
                whole,
                numerator,
                denominator,
            },
            unit,
            is_approximate: false,
        }
    }
    
    /// Create a range quantity
    pub fn range(min: f64, max: f64, unit: Unit) -> Self {
        Self {
            measurement: QuantityType::Range { min, max },
            unit,
            is_approximate: false,
        }
    }
    
    /// Create an ambiguous quantity
    pub fn ambiguous(description: &str, unit: Unit) -> Self {
        Self {
            measurement: QuantityType::Ambiguous(description.to_string()),
            unit,
            is_approximate: true,
        }
    }
    
    /// Mark this quantity as approximate
    pub fn approximate(mut self) -> Self {
        self.is_approximate = true;
        self
    }
    
    /// Get an estimated numeric value for this quantity
    pub fn estimated_value(&self) -> Option<f64> {
        match &self.measurement {
            QuantityType::Exact(amount) => Some(*amount),
            QuantityType::Fraction { whole, numerator, denominator } => {
                let whole_part = whole.unwrap_or(0) as f64;
                let fractional_part = *numerator as f64 / *denominator as f64;
                Some(whole_part + fractional_part)
            }
            QuantityType::Range { min, max } => Some((min + max) / 2.0),
            QuantityType::Ambiguous(_) => None,
        }
    }
    
    /// Check if this quantity represents a range
    pub fn is_range(&self) -> bool {
        matches!(self.measurement, QuantityType::Range { .. })
    }
    
    /// Check if this quantity is ambiguous/unclear
    pub fn is_ambiguous(&self) -> bool {
        matches!(self.measurement, QuantityType::Ambiguous(_))
    }
}

impl Unit {
    /// Get a human-readable string representation of the unit
    pub fn display_name(&self) -> &'static str {
        match self {
            Unit::Teaspoons => "tsp",
            Unit::Tablespoons => "tbsp",
            Unit::FluidOunces => "fl oz",
            Unit::Cups => "cups",
            Unit::Pints => "pints",
            Unit::Quarts => "quarts",
            Unit::Gallons => "gallons",
            Unit::Milliliters => "ml",
            Unit::Liters => "L",
            Unit::Ounces => "oz",
            Unit::Pounds => "lbs",
            Unit::Grams => "g",
            Unit::Kilograms => "kg",
            Unit::Pieces => "pieces",
            Unit::Dozen => "dozen",
            Unit::Pinches => "pinches",
            Unit::Dashes => "dashes",
            Unit::Cloves => "cloves",
            Unit::Packages => "packages",
            Unit::Cans => "cans",
            Unit::Bottles => "bottles",
            Unit::Unknown(_) => "unknown",
        }
    }
    
    /// Check if this is a volume unit
    pub fn is_volume(&self) -> bool {
        matches!(
            self,
            Unit::Teaspoons
                | Unit::Tablespoons
                | Unit::FluidOunces
                | Unit::Cups
                | Unit::Pints
                | Unit::Quarts
                | Unit::Gallons
                | Unit::Milliliters
                | Unit::Liters
        )
    }
    
    /// Check if this is a weight unit
    pub fn is_weight(&self) -> bool {
        matches!(
            self,
            Unit::Ounces | Unit::Pounds | Unit::Grams | Unit::Kilograms
        )
    }
    
    /// Check if this is a count unit
    pub fn is_count(&self) -> bool {
        matches!(
            self,
            Unit::Pieces | Unit::Dozen | Unit::Cloves | Unit::Packages | Unit::Cans | Unit::Bottles
        )
    }
}

impl IngredientList {
    /// Create a new empty ingredient list
    pub fn new(original_text: String) -> Self {
        Self {
            ingredients: Vec::new(),
            original_text,
            overall_confidence: 1.0,
            unparsed_lines: Vec::new(),
        }
    }
    
    /// Add an ingredient to the list
    pub fn add_ingredient(&mut self, ingredient: Ingredient) {
        self.ingredients.push(ingredient);
        self.recalculate_confidence();
    }
    
    /// Add an unparsed line
    pub fn add_unparsed_line(&mut self, line: String) {
        self.unparsed_lines.push(line);
        self.recalculate_confidence();
    }
    
    /// Get the number of successfully parsed ingredients
    pub fn parsed_count(&self) -> usize {
        self.ingredients.len()
    }
    
    /// Get the number of unparsed lines
    pub fn unparsed_count(&self) -> usize {
        self.unparsed_lines.len()
    }
    
    /// Calculate overall parsing success rate
    pub fn success_rate(&self) -> f32 {
        let total_lines = self.parsed_count() + self.unparsed_count();
        if total_lines == 0 {
            return 1.0;
        }
        self.parsed_count() as f32 / total_lines as f32
    }
    
    /// Recalculate the overall confidence score
    fn recalculate_confidence(&mut self) {
        if self.ingredients.is_empty() {
            self.overall_confidence = if self.unparsed_lines.is_empty() { 1.0 } else { 0.0 };
            return;
        }
        
        let avg_ingredient_confidence: f32 = self.ingredients.iter()
            .map(|i| i.confidence)
            .sum::<f32>() / self.ingredients.len() as f32;
        
        let success_rate = self.success_rate();
        
        // Overall confidence is a combination of success rate and average ingredient confidence
        self.overall_confidence = (avg_ingredient_confidence + success_rate) / 2.0;
    }
}

impl fmt::Display for Ingredient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(quantity) = &self.quantity {
            write!(f, "{} {}", quantity, self.name)?;
        } else {
            write!(f, "{}", self.name)?;
        }
        
        if let Some(modifier) = &self.modifier {
            write!(f, " ({})", modifier)?;
        }
        
        Ok(())
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_approximate {
            write!(f, "~")?;
        }
        
        match &self.measurement {
            QuantityType::Exact(amount) => {
                if amount.fract() == 0.0 {
                    write!(f, "{}", amount as i64)?;
                } else {
                    write!(f, "{}", amount)?;
                }
            }
            QuantityType::Fraction { whole, numerator, denominator } => {
                if let Some(w) = whole {
                    write!(f, "{} {}/{}", w, numerator, denominator)?;
                } else {
                    write!(f, "{}/{}", numerator, denominator)?;
                }
            }
            QuantityType::Range { min, max } => {
                write!(f, "{}-{}", min, max)?;
            }
            QuantityType::Ambiguous(desc) => {
                write!(f, "{}", desc)?;
            }
        }
        
        write!(f, " {}", self.unit.display_name())
    }
}

impl fmt::Display for IngredientList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Ingredient List ({} parsed, {} unparsed, {:.1}% confidence):", 
                 self.parsed_count(), self.unparsed_count(), self.overall_confidence * 100.0)?;
        
        for ingredient in &self.ingredients {
            writeln!(f, "  • {}", ingredient)?;
        }
        
        if !self.unparsed_lines.is_empty() {
            writeln!(f, "Unparsed:")?;
            for line in &self.unparsed_lines {
                writeln!(f, "  ? {}", line)?;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingredient_creation() {
        let ingredient = Ingredient::new("flour")
            .with_quantity(Quantity::exact(2.0, Unit::Cups))
            .with_modifier("all-purpose")
            .with_confidence(0.9);
        
        assert_eq!(ingredient.name, "flour");
        assert!(ingredient.has_quantity());
        assert_eq!(ingredient.estimated_amount(), Some(2.0));
        assert_eq!(ingredient.modifier, Some("all-purpose".to_string()));
        assert_eq!(ingredient.confidence, 0.9);
    }

    #[test]
    fn test_quantity_exact() {
        let qty = Quantity::exact(1.5, Unit::Cups);
        assert_eq!(qty.estimated_value(), Some(1.5));
        assert!(!qty.is_range());
        assert!(!qty.is_ambiguous());
    }

    #[test]
    fn test_quantity_fraction() {
        let qty = Quantity::fraction(Some(2), 1, 4, Unit::Cups);
        assert_eq!(qty.estimated_value(), Some(2.25));
        
        let qty_no_whole = Quantity::fraction(None, 3, 4, Unit::Cups);
        assert_eq!(qty_no_whole.estimated_value(), Some(0.75));
    }

    #[test]
    fn test_quantity_range() {
        let qty = Quantity::range(2.0, 3.0, Unit::Tablespoons);
        assert_eq!(qty.estimated_value(), Some(2.5));
        assert!(qty.is_range());
        assert!(!qty.is_ambiguous());
    }

    #[test]
    fn test_quantity_ambiguous() {
        let qty = Quantity::ambiguous("to taste", Unit::Unknown("".to_string()));
        assert_eq!(qty.estimated_value(), None);
        assert!(!qty.is_range());
        assert!(qty.is_ambiguous());
        assert!(qty.is_approximate);
    }

    #[test]
    fn test_unit_properties() {
        assert!(Unit::Cups.is_volume());
        assert!(!Unit::Cups.is_weight());
        assert!(!Unit::Cups.is_count());
        
        assert!(Unit::Pounds.is_weight());
        assert!(!Unit::Pounds.is_volume());
        assert!(!Unit::Pounds.is_count());
        
        assert!(Unit::Pieces.is_count());
        assert!(!Unit::Pieces.is_volume());
        assert!(!Unit::Pieces.is_weight());
    }

    #[test]
    fn test_ingredient_list() {
        let mut list = IngredientList::new("2 cups flour\n1 tbsp salt".to_string());
        
        list.add_ingredient(
            Ingredient::new("flour")
                .with_quantity(Quantity::exact(2.0, Unit::Cups))
        );
        
        list.add_ingredient(
            Ingredient::new("salt")
                .with_quantity(Quantity::exact(1.0, Unit::Tablespoons))
        );
        
        assert_eq!(list.parsed_count(), 2);
        assert_eq!(list.unparsed_count(), 0);
        assert_eq!(list.success_rate(), 1.0);
    }

    #[test]
    fn test_ingredient_list_with_unparsed() {
        let mut list = IngredientList::new("2 cups flour\nsome mysterious ingredient".to_string());
        
        list.add_ingredient(
            Ingredient::new("flour")
                .with_quantity(Quantity::exact(2.0, Unit::Cups))
        );
        
        list.add_unparsed_line("some mysterious ingredient".to_string());
        
        assert_eq!(list.parsed_count(), 1);
        assert_eq!(list.unparsed_count(), 1);
        assert_eq!(list.success_rate(), 0.5);
        assert!(list.overall_confidence < 1.0);
    }

    #[test]
    fn test_display_formatting() {
        let ingredient = Ingredient::new("onions")
            .with_quantity(Quantity::range(2.0, 3.0, Unit::Pieces))
            .with_modifier("diced");
        
        let display = format!("{}", ingredient);
        assert!(display.contains("onions"));
        assert!(display.contains("2-3"));
        assert!(display.contains("diced"));
    }

    #[test]
    fn test_fraction_display() {
        let qty = Quantity::fraction(Some(1), 1, 2, Unit::Cups);
        let display = format!("{}", qty);
        assert_eq!(display, "1 1/2 cups");
        
        let qty_no_whole = Quantity::fraction(None, 3, 4, Unit::Teaspoons);
        let display = format!("{}", qty_no_whole);
        assert_eq!(display, "3/4 tsp");
    }
}