# Ingredient and Quantity Extraction Data Model

## Overview

This document defines the data structures and parsing logic for extracting structured ingredient information from raw text. The model supports various quantity formats, units, and handles common edge cases found in ingredient lists.

## Core Data Structures

### ParsedIngredientEntry

The main structure representing a parsed ingredient with all its components:

```rust
pub struct ParsedIngredientEntry {
    pub quantity: Option<Quantity>,        // The amount (e.g., 2, 1/2, 2-3)
    pub unit: Option<Unit>,               // The measurement unit (cups, grams, etc.)
    pub ingredient_name: String,          // The ingredient name (normalized)
    pub original_text: String,           // Original text that was parsed
    pub confidence: f32,                 // Parsing confidence (0.0 to 1.0)
}
```

### Quantity

Represents different types of quantities with support for various formats:

```rust
pub enum Quantity {
    Exact(f64),                         // "2", "1.5"
    Fraction {                          // "1/2", "2 1/4"
        whole: Option<u32>,
        numerator: u32,
        denominator: u32,
    },
    Range { min: f64, max: f64 },       // "2-3", "1 to 2"
    Approximate(f64),                   // "about 2", "around 1.5"
    Textual(String),                    // "a pinch", "to taste"
}
```

### Unit

Categorized measurement units with comprehensive coverage:

```rust
pub enum Unit {
    Volume(VolumeUnit),      // cups, tablespoons, liters, etc.
    Weight(WeightUnit),      // grams, ounces, pounds, etc.
    Count(CountUnit),        // pieces, cloves, heads, etc.
    Length(LengthUnit),      // inches, centimeters, etc.
    Temperature(TemperatureUnit), // Celsius, Fahrenheit, etc.
    Custom(String),          // Unknown or custom units
}
```

## Supported Formats

### Quantity Formats

| Format | Example | Parsed As |
|--------|---------|-----------|
| **Integers** | `2 cups flour` | `Exact(2.0)` |
| **Decimals** | `1.5 tablespoons oil` | `Exact(1.5)` |
| **Simple Fractions** | `1/2 teaspoon salt` | `Fraction { whole: None, numerator: 1, denominator: 2 }` |
| **Mixed Numbers** | `2 1/4 cups sugar` | `Fraction { whole: Some(2), numerator: 1, denominator: 4 }` |
| **Unicode Fractions** | `½ cup milk` | `Fraction { whole: None, numerator: 1, denominator: 2 }` |
| **Ranges (Hyphen)** | `2-3 cloves garlic` | `Range { min: 2.0, max: 3.0 }` |
| **Ranges (Words)** | `1 to 2 tablespoons` | `Range { min: 1.0, max: 2.0 }` |
| **Approximate** | `about 2 cups` | `Approximate(2.0)` |
| **Textual** | `pinch of salt` | `Textual("pinch")` |

### Unit Categories

#### Volume Units
- **Metric**: `ml`, `milliliter`, `liter`, `l`
- **Imperial**: `teaspoon`, `tsp`, `tablespoon`, `tbsp`, `cup`, `pint`, `quart`, `gallon`
- **Cooking**: `pinch`, `dash`, `drop`

#### Weight Units
- **Metric**: `mg`, `gram`, `g`, `kilogram`, `kg`
- **Imperial**: `ounce`, `oz`, `pound`, `lb`

#### Count Units
- **Generic**: `piece`, `item`
- **Specific**: `clove`, `head`, `bunch`, `can`, `package`, `bottle`, `box`, `bag`

## Examples

### Typical Cases

```rust
// Basic quantity with unit
"2 cups flour"
→ ParsedIngredientEntry {
    quantity: Some(Exact(2.0)),
    unit: Some(Volume(Cup)),
    ingredient_name: "flour",
    confidence: 0.9
}

// Fraction with unit
"1/2 teaspoon vanilla extract"
→ ParsedIngredientEntry {
    quantity: Some(Fraction { whole: None, numerator: 1, denominator: 2 }),
    unit: Some(Volume(Teaspoon)),
    ingredient_name: "vanilla extract",
    confidence: 0.9
}

// Count without explicit unit
"3 large eggs"
→ ParsedIngredientEntry {
    quantity: Some(Exact(3.0)),
    unit: None,
    ingredient_name: "large eggs",
    confidence: 0.7
}
```

### Edge Cases

#### Ranges
```rust
// Hyphen range
"2-3 tablespoons olive oil"
→ ParsedIngredientEntry {
    quantity: Some(Range { min: 2.0, max: 3.0 }),
    unit: Some(Volume(Tablespoon)),
    ingredient_name: "olive oil",
    confidence: 0.9
}

// Word range
"1 to 2 pounds chicken breast"
→ ParsedIngredientEntry {
    quantity: Some(Range { min: 1.0, max: 2.0 }),
    unit: Some(Weight(Pound)),
    ingredient_name: "chicken breast",
    confidence: 0.8
}
```

#### Missing Units
```rust
// Countable items
"4 tomatoes"
→ ParsedIngredientEntry {
    quantity: Some(Exact(4.0)),
    unit: None,
    ingredient_name: "tomatoes",
    confidence: 0.7
}

// Weight implied
"1 onion, diced"
→ ParsedIngredientEntry {
    quantity: Some(Exact(1.0)),
    unit: None,
    ingredient_name: "onion, diced",
    confidence: 0.6
}
```

#### Textual Quantities
```rust
// Imprecise measurements
"salt to taste"
→ ParsedIngredientEntry {
    quantity: None,
    unit: None,
    ingredient_name: "salt to taste",
    confidence: 0.5
}

"a pinch of black pepper"
→ ParsedIngredientEntry {
    quantity: Some(Textual("pinch")),
    unit: None,
    ingredient_name: "of black pepper",
    confidence: 0.6
}
```

#### Plurals and Normalization
```rust
// Singular/plural unit handling
"1 cup sugar" vs "2 cups sugar"
→ Both parsed with unit: Volume(Cup)

// Ingredient name normalization
"2 lbs. ground beef" vs "2 pounds ground beef"
→ Both parsed with unit: Weight(Pound)
```

#### Ambiguous Cases
```rust
// Multiple valid interpretations
"2 8-oz cans tomato sauce"
→ Could be:
   - quantity: Exact(2.0), unit: Count(Can), ingredient: "8-oz tomato sauce"
   - quantity: Exact(16.0), unit: Weight(Ounce), ingredient: "tomato sauce"
→ Returns AmbiguousParsing error with alternatives
```

## Error Handling

The parsing system includes comprehensive error handling for various edge cases:

```rust
pub enum IngredientParseError {
    EmptyInput,                         // Empty or whitespace-only input
    NoIngredientFound,                  // No ingredient name identified
    InvalidQuantity(String),            // Malformed quantity (e.g., "1/0")
    UnknownUnit(String),               // Unrecognized unit
    AmbiguousParsing(Vec<String>),     // Multiple valid interpretations
    ConflictingInformation(String),     // Contradictory information
}
```

## Confidence Scoring

The confidence score (0.0 to 1.0) indicates the reliability of the parsing:

- **0.9-1.0**: High confidence - complete quantity, unit, and ingredient
- **0.7-0.8**: Good confidence - missing one component (usually unit)
- **0.5-0.6**: Medium confidence - ambiguous or textual quantities
- **0.0-0.4**: Low confidence - unclear parsing

Factors affecting confidence:
- Presence of recognized quantity format
- Presence of recognized unit
- Clear ingredient name identification
- Absence of ambiguity

## Usage Examples

### Basic Parsing
```rust
use crate::ingredient_parser::parse_ingredient_line;

let result = parse_ingredient_line("2 cups all-purpose flour")?;
println!("Quantity: {:?}", result.quantity);
println!("Unit: {:?}", result.unit);
println!("Ingredient: {}", result.ingredient_name);
println!("Confidence: {:.2}", result.confidence);
```

### Batch Processing
```rust
let ingredient_lines = vec![
    "2 cups flour",
    "1/2 teaspoon salt",
    "3 large eggs",
    "1-2 tablespoons olive oil",
    "salt to taste"
];

for line in ingredient_lines {
    match parse_ingredient_line(line) {
        Ok(entry) => println!("✓ {}: {}", line, entry.normalized_string()),
        Err(e) => println!("✗ {}: {}", line, e),
    }
}
```

### Quantity Calculations
```rust
if let Some(decimal_value) = result.quantity.and_then(|q| q.to_decimal()) {
    println!("Decimal equivalent: {}", decimal_value);
}

if let Some((min, max)) = result.quantity.and_then(|q| q.value_range()) {
    println!("Value range: {} to {}", min, max);
}
```

## Future Enhancements

### Planned Features
1. **Unit Conversions**: Convert between compatible units (cups ↔ ml, oz ↔ grams)
2. **Recipe Scaling**: Scale quantities up or down proportionally
3. **Nutritional Integration**: Link with nutritional databases
4. **Multi-language Support**: Parse ingredients in different languages
5. **Context Awareness**: Use recipe context to resolve ambiguities

### Advanced Parsing
1. **Complex Fractions**: Support for fractions like "2 1/3" in text form
2. **Temperature Ranges**: Handle "350-375°F" for cooking instructions
3. **Preparation Methods**: Extract "diced", "chopped", "minced" modifiers
4. **Alternative Ingredients**: Parse "1 cup butter or margarine"

## Integration with Existing System

The ingredient parser integrates with the existing OCR and database system:

1. **OCR Output**: Raw text from image processing
2. **Line-by-Line Parsing**: Each line processed through `parse_ingredient_line()`
3. **Structured Storage**: Parsed entries stored alongside original text
4. **Enhanced Search**: Search by ingredient name, quantity ranges, or units

This structured approach enables more sophisticated recipe analysis, nutrition calculation, and intelligent search capabilities while maintaining compatibility with the existing simple text storage system.