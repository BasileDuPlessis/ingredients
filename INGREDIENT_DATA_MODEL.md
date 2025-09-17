# Ingredient and Quantity Extraction Data Model

## Overview

This document defines the comprehensive data model for ingredient and quantity extraction in the Ingredients Telegram bot. The model handles structured parsing of ingredient lists from OCR text, supporting various formats, edge cases, and ambiguous scenarios.

## Core Data Structures

### Ingredient
Represents a single ingredient with optional quantity and modifiers.

```rust
pub struct Ingredient {
    pub name: String,           // The ingredient name (e.g., "flour", "olive oil")
    pub quantity: Option<Quantity>, // Optional quantity measurement
    pub modifier: Option<String>,   // Optional preparation notes (e.g., "diced", "fresh")
    pub notes: Option<String>,      // Additional notes or uncertainty markers
    pub confidence: f32,            // Parsing confidence (0.0 to 1.0)
}
```

### Quantity
Represents a measurement with support for various formats.

```rust
pub struct Quantity {
    pub measurement: QuantityType, // The type of measurement
    pub unit: Unit,               // The unit of measurement
    pub is_approximate: bool,     // Whether the quantity is approximate
}
```

### QuantityType
Enum supporting different quantity formats.

```rust
pub enum QuantityType {
    Exact(f64),                   // e.g., "2 cups"
    Fraction {                    // e.g., "1/2 cup", "2 1/4 tbsp"
        whole: Option<u32>,
        numerator: u32,
        denominator: u32,
    },
    Range {                       // e.g., "2-3 cups", "1 to 2 tbsp"
        min: f64,
        max: f64,
    },
    Ambiguous(String),           // e.g., "to taste", "some", "a little"
}
```

### Unit
Comprehensive unit support with normalization.

```rust
pub enum Unit {
    // Volume units
    Teaspoons, Tablespoons, FluidOunces, Cups, Pints, Quarts, Gallons,
    Milliliters, Liters,
    
    // Weight units
    Ounces, Pounds, Grams, Kilograms,
    
    // Count units
    Pieces, Dozen,
    
    // Specialized units
    Pinches, Dashes, Cloves, Packages, Cans, Bottles,
    
    // Unknown unit
    Unknown(String),
}
```

### IngredientList
Collection of parsed ingredients with metadata.

```rust
pub struct IngredientList {
    pub ingredients: Vec<Ingredient>,     // Successfully parsed ingredients
    pub original_text: String,           // Original OCR text
    pub overall_confidence: f32,         // Overall parsing confidence
    pub unparsed_lines: Vec<String>,     // Lines that couldn't be parsed
}
```

## Database Schema

### Tables

#### ingredient_entries
Stores structured ingredient data.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PRIMARY KEY | Unique identifier |
| telegram_id | INTEGER NOT NULL | Telegram user ID |
| original_text | TEXT NOT NULL | Original OCR text |
| parsed_ingredients | TEXT NOT NULL | JSON serialized IngredientList |
| parsing_confidence | REAL NOT NULL | Overall parsing confidence |
| created_at | DATETIME | Creation timestamp |

#### entries (legacy)
Maintains backward compatibility for simple text storage.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PRIMARY KEY | Unique identifier |
| telegram_id | INTEGER NOT NULL | Telegram user ID |
| content | TEXT NOT NULL | Raw text content |
| created_at | DATETIME | Creation timestamp |

### Full-Text Search
Both tables support FTS5 for efficient searching:
- `ingredient_entries_fts` - for structured ingredient search
- `entries_fts` - for legacy text search

## Parsing Examples

### Typical Cases

#### Simple Ingredients
```
Input: "2 cups flour"
Output: Ingredient {
    name: "flour",
    quantity: Some(Quantity {
        measurement: Exact(2.0),
        unit: Cups,
        is_approximate: false
    }),
    confidence: 1.0
}
```

#### Fractions
```
Input: "1/2 cup sugar"
Output: Quantity {
    measurement: Fraction {
        whole: None,
        numerator: 1,
        denominator: 2
    },
    unit: Cups
}

Input: "2 1/4 tablespoons butter"
Output: Quantity {
    measurement: Fraction {
        whole: Some(2),
        numerator: 1,
        denominator: 4
    },
    unit: Tablespoons
}
```

#### Ranges
```
Input: "2-3 medium onions"
Output: Ingredient {
    name: "medium onions",
    quantity: Some(Quantity {
        measurement: Range { min: 2.0, max: 3.0 },
        unit: Pieces
    })
}

Input: "1 to 2 tablespoons olive oil"
Output: Quantity {
    measurement: Range { min: 1.0, max: 2.0 },
    unit: Tablespoons
}
```

#### With Modifiers
```
Input: "2 cups flour (all-purpose)"
Output: Ingredient {
    name: "flour",
    quantity: Some(Quantity::exact(2.0, Unit::Cups)),
    modifier: Some("all-purpose")
}

Input: "3 tomatoes, diced"
Output: Ingredient {
    name: "tomatoes",
    quantity: Some(Quantity::exact(3.0, Unit::Pieces)),
    modifier: Some("diced")
}
```

### Edge Cases

#### Ambiguous Quantities
```
Input: "salt to taste"
Output: Ingredient {
    name: "salt to taste",
    quantity: Some(Quantity {
        measurement: Ambiguous("to taste"),
        unit: Unknown(""),
        is_approximate: true
    }),
    confidence: 0.6
}

Input: "some fresh herbs"
Output: Ingredient {
    name: "some fresh herbs",
    quantity: Some(Quantity::ambiguous("some", Unit::Unknown(""))),
    confidence: 0.6
}
```

#### Complex Fractions
```
Input: "1⁄2 teaspoon vanilla extract"
Output: Quantity {
    measurement: Fraction {
        whole: None,
        numerator: 1,
        denominator: 2
    },
    unit: Teaspoons
}
```

#### Decimal Quantities
```
Input: "1.5 pounds ground beef"
Output: Quantity {
    measurement: Exact(1.5),
    unit: Pounds
}
```

#### No Quantity
```
Input: "eggs"
Output: Ingredient {
    name: "eggs",
    quantity: None,
    confidence: 0.5  // Lower confidence for incomplete parsing
}
```

#### Unknown Units
```
Input: "2 bottles wine"
Output: Ingredient {
    name: "wine",
    quantity: Some(Quantity {
        measurement: Exact(2.0),
        unit: Bottles
    })
}

Input: "1 package cream cheese"
Output: Unit::Packages
```

### Multi-Language Support

#### French Examples
```
Input: "250 g farine"
Output: Ingredient {
    name: "farine",
    quantity: Some(Quantity::exact(250.0, Unit::Grams))
}

Input: "2 cas huile d'olive"
Output: Ingredient {
    name: "huile d'olive",
    quantity: Some(Quantity::exact(2.0, Unit::Tablespoons))
}

Input: "sel au goût"
Output: Ingredient {
    name: "sel au goût",
    quantity: Some(Quantity::ambiguous("au goût", Unit::Unknown("")))
}
```

### Complex Lists

#### Mixed Format List
```
Input: 
"2 cups all-purpose flour
1/2 cup sugar
3 large eggs
1-2 tablespoons milk
salt to taste
vanilla extract"

Output: IngredientList {
    ingredients: [
        Ingredient { name: "all-purpose flour", quantity: Exact(2.0, Cups) },
        Ingredient { name: "sugar", quantity: Fraction(1, 2, Cups) },
        Ingredient { name: "large eggs", quantity: Exact(3.0, Pieces) },
        Ingredient { name: "milk", quantity: Range(1.0, 2.0, Tablespoons) },
        Ingredient { name: "salt to taste", quantity: Ambiguous("to taste") },
        Ingredient { name: "vanilla extract", quantity: None }
    ],
    unparsed_lines: [],
    overall_confidence: 0.85
}
```

## Confidence Scoring

### Factors Affecting Confidence

1. **Parsing Success Rate**: Ratio of successfully parsed lines to total lines
2. **Quantity Recognition**: Higher confidence for recognized quantities vs. ambiguous ones
3. **Unit Recognition**: Known units vs. unknown units
4. **Pattern Matching**: Clean regex matches vs. fallback parsing

### Confidence Levels

- **1.0**: Perfect parsing with recognized quantities and units
- **0.9**: Good parsing with minor ambiguities
- **0.7**: Partial parsing with some unknown elements
- **0.5**: Fallback parsing (ingredient name only)
- **0.3**: High uncertainty, possible misparse
- **0.0**: Failed to parse

## Usage in the Application

### Parsing Flow
1. OCR extracts raw text from image
2. `ingredient_parser::parse_ingredient_list()` processes the text
3. Structured data is stored in `ingredient_entries` table
4. Both original text and structured data are searchable

### Search Capabilities
- Full-text search on original OCR text
- Structured queries on ingredient names
- Quantity-based filtering (future enhancement)
- Unit normalization for comparisons

### Backward Compatibility
- Legacy `entries` table remains for simple text storage
- New structured parsing is additive, not replacing
- Existing search functionality continues to work

## Future Enhancements

### Planned Features
1. **Unit Conversion**: Automatic conversion between metric and imperial
2. **Quantity Aggregation**: Sum quantities of same ingredients
3. **Recipe Scaling**: Scale all quantities by a factor
4. **Nutritional Analysis**: Integration with nutrition databases
5. **Smart Suggestions**: Suggest missing ingredients or quantities

### Advanced Parsing
1. **Context-Aware Parsing**: Use surrounding ingredients for better unit guessing
2. **Machine Learning**: Train models on user corrections
3. **Image-Based Hints**: Use OCR confidence and layout for better parsing
4. **Multi-Recipe Detection**: Handle multiple recipes in one image

## Error Handling

### Parse Errors
- Invalid number formats
- Division by zero in fractions
- Unknown units (stored as `Unit::Unknown`)
- Malformed ingredient lines

### Recovery Strategies
- Graceful degradation to text-only storage
- Partial parsing with confidence scores
- User feedback integration for corrections
- Fallback to legacy entry format when needed

This data model provides a robust foundation for ingredient extraction while maintaining flexibility for future enhancements and backward compatibility with existing functionality.