# Ingredient and Quantity Data Model Implementation

This implementation provides a comprehensive data model for extracting and representing ingredients and their quantities from OCR text, addressing the requirements specified in issue #23.

## 🎯 Implementation Overview

### Core Components

1. **Data Structures** (`src/ingredient_model.rs`)
   - `Ingredient`: Represents a food item with optional quantity and modifiers
   - `Quantity`: Supports various measurement formats (exact, fractions, ranges, ambiguous)
   - `Unit`: Comprehensive unit system with categorization
   - `IngredientList`: Collection with confidence scoring

2. **Parsing Engine** (`src/ingredient_parser.rs`)
   - Regex-based parsing for natural language ingredient lists
   - Multi-language support (English and French)
   - Handles edge cases and ambiguous quantities

3. **Database Integration** (`src/db.rs`)
   - Extended schema with structured ingredient storage
   - JSON serialization for complex data structures
   - Full-text search capabilities
   - Backward compatibility with existing text storage

4. **Integration Layer** (`src/ingredient_integration.rs`)
   - Connects OCR pipeline with structured parsing
   - Display formatting and categorization
   - Usage examples and integration patterns

## ✨ Key Features Implemented

### Quantity Format Support
- ✅ **Exact amounts**: "2 cups", "1.5 tablespoons"
- ✅ **Fractions**: "1/2 cup", "2 1/4 teaspoons"  
- ✅ **Ranges**: "2-3 onions", "1 to 2 tablespoons"
- ✅ **Ambiguous**: "salt to taste", "some herbs", "a handful"

### Unit Recognition
- ✅ **Volume units**: cups, tablespoons, teaspoons, liters, etc.
- ✅ **Weight units**: pounds, ounces, grams, kilograms
- ✅ **Count units**: pieces, dozen, cloves, packages
- ✅ **Specialized units**: pinches, dashes, cans, bottles

### Multi-Language Support
- ✅ **English**: Standard and abbreviated units
- ✅ **French**: "grammes", "cuillères à soupe", "tasses"
- ✅ **Extensible**: Easy to add more languages

### Edge Case Handling
- ✅ **No quantities**: "eggs", "vanilla extract"
- ✅ **Unknown units**: Graceful fallback to `Unit::Unknown`
- ✅ **Malformed input**: Confidence scoring and partial parsing
- ✅ **Complex modifiers**: "(diced)", "fresh", "extra virgin"

## 📊 Parsing Examples

### Typical Cases
```rust
"2 cups flour" → Ingredient { 
    name: "flour", 
    quantity: Exact(2.0, Cups), 
    confidence: 1.0 
}

"1/2 teaspoon salt" → Ingredient {
    name: "salt",
    quantity: Fraction { whole: None, numerator: 1, denominator: 2 },
    unit: Teaspoons
}
```

### Edge Cases
```rust
"salt to taste" → Ingredient {
    name: "salt to taste",
    quantity: Ambiguous("to taste"),
    confidence: 0.6
}

"250 g farine" → Ingredient {  // French
    name: "farine",
    quantity: Exact(250.0, Grams)
}
```

## 🗄️ Database Schema

### New Table: `ingredient_entries`
```sql
CREATE TABLE ingredient_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    telegram_id INTEGER NOT NULL,
    original_text TEXT NOT NULL,
    parsed_ingredients TEXT NOT NULL,  -- JSON serialized IngredientList
    parsing_confidence REAL NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### Full-Text Search
```sql
CREATE VIRTUAL TABLE ingredient_entries_fts USING fts5(
    original_text, 
    content='ingredient_entries', 
    content_rowid='id'
);
```

## 🧪 Testing and Validation

Run the validation script:
```bash
./test_data_model.sh
```

### Test Results
- ✅ Core data structures defined and validated
- ✅ Parsing logic implemented with comprehensive unit tests
- ✅ Database schema extended with proper integration
- ✅ 18+ documented examples covering typical and edge cases
- ✅ Multi-language support validated
- ✅ Confidence scoring system implemented

## 📖 Usage Examples

### Basic Parsing
```rust
use ingredients::ingredient_parser::parse_ingredient_list;

let text = "2 cups flour\n1/2 tsp salt\n3 eggs";
let parsed = parse_ingredient_list(text);

println!("Parsed {} ingredients with {:.1}% confidence", 
         parsed.parsed_count(), 
         parsed.overall_confidence * 100.0);
```

### Database Integration
```rust
use ingredients::db::{create_ingredient_entry, read_ingredient_entry};

// Store structured data
let entry_id = create_ingredient_entry(&conn, telegram_id, &ingredient_list)?;

// Retrieve and use
let entry = read_ingredient_entry(&conn, entry_id)?.unwrap();
let parsed = get_parsed_ingredients(&entry)?;
```

### Advanced Features
```rust
// Quantity estimation
let amount = ingredient.estimated_amount(); // Option<f64>

// Unit categorization  
if ingredient.quantity.unit.is_volume() {
    println!("This is a volume measurement");
}

// Confidence assessment
if ingredient.confidence < 0.7 {
    println!("Low confidence parsing - may need review");
}
```

## 🚀 Integration with Existing System

### Backward Compatibility
- ✅ Existing `entries` table and functions preserved
- ✅ Legacy text-based search continues to work
- ✅ New structured parsing is additive, not replacing

### OCR Pipeline Integration
```rust
// In the OCR processing flow:
let ocr_text = extract_text_from_image(...).await?;
let ingredient_list = parse_ingredient_list(&ocr_text);
let entry_id = create_ingredient_entry(&conn, telegram_id, &ingredient_list)?;

// Enhanced user response with structured data
let display_text = format_parsed_ingredients_for_display(&conn, entry_id)?;
bot.send_message(chat_id, &display_text).await?;
```

## 📈 Future Enhancements

The data model is designed to support future enhancements:

1. **Unit Conversion**: Easy to add metric/imperial conversion
2. **Recipe Scaling**: Multiply all quantities by a factor  
3. **Nutritional Analysis**: Link ingredients to nutrition databases
4. **Smart Suggestions**: Suggest missing ingredients or quantities
5. **Machine Learning**: Train models on user corrections

## 🎯 Success Metrics

- ✅ **Comprehensive Format Support**: Handles exact, fractional, range, and ambiguous quantities
- ✅ **Robust Unit System**: 20+ unit types with categorization
- ✅ **Multi-Language Ready**: English and French, extensible to other languages
- ✅ **Edge Case Resilience**: Graceful handling of malformed or ambiguous input
- ✅ **Confidence Scoring**: 0.0-1.0 scoring system for parsing quality assessment
- ✅ **Database Integration**: Structured storage with full-text search capabilities
- ✅ **Backward Compatibility**: Preserves existing functionality while adding new features
- ✅ **Comprehensive Documentation**: Detailed examples for typical and edge cases
- ✅ **Test Coverage**: Extensive test suite validating all major functionality

The implementation successfully addresses all requirements from issue #23 and provides a solid foundation for future enhancements to the Ingredients Telegram bot.