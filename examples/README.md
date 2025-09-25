# Recipe Parser Example

This example demonstrates how to use the `text_processing` module from the Ingredients Telegram bot to extract measurements and ingredients from recipe text.

## Features Demonstrated

- **Basic Recipe Parsing**: Extract measurements from standard English recipes
- **French Recipe Support**: Handle French measurements with proper post-processing
- **Custom Patterns**: Use custom regex patterns for specific measurement types
- **Complex Recipe Analysis**: Parse recipes with multiple ingredients per line
- **Measurement Statistics**: Extract unique measurement units from text
- **Post-Processing Control**: Compare results with and without ingredient name cleaning
- **Error Handling**: Proper error handling for invalid regex patterns

## Running the Example

```bash
cargo run --example recipe_parser
```

## Sample Output

The example processes several different recipe types and shows:

1. **Chocolate Chip Cookies** - Standard English recipe with various measurements
2. **Crêpes Suzette** - French recipe with proper accent handling and post-processing
3. **Volume-Only Detection** - Custom pattern that only detects volume measurements
4. **Complex Salad Dressing** - Recipe with multiple ingredients per line
5. **Grocery Shopping List** - Measurement statistics and unit extraction
6. **Post-Processing Comparison** - Before/after comparison of ingredient cleaning
7. **Error Handling** - Demonstrates proper error handling for invalid patterns

## Key Features Highlighted

- **Bilingual Support**: English and French measurement detection
- **Ingredient Post-Processing**: Automatic cleaning of prepositions and articles
- **Flexible Configuration**: Customizable patterns and processing options
- **Performance**: Lazy-loaded regex patterns for efficiency
- **Robust Parsing**: Handles complex real-world recipe formats