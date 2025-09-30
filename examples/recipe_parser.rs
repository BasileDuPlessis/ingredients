//! # Recipe Parser Example
//!
//! This example demonstrates how to use the `text_processing` module to extract
//! measurements and ingredients from recipe text. It shows various configuration
//! options and real-world recipe parsing scenarios, including support for
//! quantity-only ingredients (e.g., "6 oeufs", "4 pommes") that don't have
//! measurement units.

use ingredients::text_processing::{MeasurementConfig, MeasurementDetector};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🍳 Recipe Measurement Parser Example");
    println!("=====================================\n");

    // Example 1: Basic usage with default settings
    println!("📖 Example 1: Basic Recipe Parsing");
    println!("-----------------------------------");

    let basic_detector = MeasurementDetector::new()?;
    let simple_recipe = r#"
    Classic Chocolate Chip Cookies

    Ingredients:
    2 1/4 cups all-purpose flour
    1 teaspoon baking soda
    1 teaspoon salt
    1 cup unsalted butter, softened
    3/4 cup granulated sugar
    3/4 cup brown sugar
    2 large eggs
    2 teaspoons vanilla extract
    2 cups chocolate chips

    Instructions:
    Preheat oven to 375°F...
    "#;

    let matches = basic_detector.extract_ingredient_measurements(simple_recipe);

    println!("Found {} measurements:", matches.len());
    for (i, measurement) in matches.iter().enumerate() {
        println!(
            "  {}. {} → \"{}\" (line {})",
            i + 1,
            measurement.text,
            measurement.ingredient_name,
            measurement.line_number + 1
        );
    }

    println!("\n");

    // Example 2: French recipe with post-processing
    println!("🇫🇷 Example 2: French Recipe with Post-Processing");
    println!("------------------------------------------------");

    let french_config = MeasurementConfig {
        enable_ingredient_postprocessing: true,
        max_ingredient_length: 100,
        ..Default::default()
    };

    let french_detector = MeasurementDetector::with_config(french_config)?;
    let french_recipe = r#"
    Crêpes Suzette

    Ingrédients:
    125 g de farine
    2 œufs
    1/2 litre de lait
    2 cuillères à soupe de sucre
    1 pincée de sel
    50 g de beurre fondu
    2 oranges
    100 g de sucre en poudre
    4 cuillères à soupe de Grand Marnier
    "#;

    let french_matches = french_detector.extract_ingredient_measurements(french_recipe);

    println!(
        "Found {} measurements (with post-processing):",
        french_matches.len()
    );
    for (i, measurement) in french_matches.iter().enumerate() {
        println!(
            "  {}. {} → \"{}\" (line {})",
            i + 1,
            measurement.text,
            measurement.ingredient_name,
            measurement.line_number + 1
        );
    }

    println!("\n");

    // Example 3: Custom pattern for specific measurements
    println!("🎯 Example 3: Custom Pattern (Volume Only)");
    println!("------------------------------------------");

    let volume_only_detector = MeasurementDetector::with_pattern(
        r"(?i)\b\d*\.?\d+\s+(?:cup(?:s)?|tablespoon(?:s)?|teaspoon(?:s)?|pint(?:s)?|quart(?:s)?|gallon(?:s)?|fluid\s+ounce(?:s)?|fl\s+oz)\b",
    )?;

    let mixed_recipe = r#"
    Mixed Measurements:
    2 cups flour
    500g sugar
    1 tablespoon vanilla
    250 ml milk
    1 teaspoon salt
    3 eggs
    "#;

    let volume_matches = volume_only_detector.extract_ingredient_measurements(mixed_recipe);

    println!(
        "Found {} volume measurements (custom pattern):",
        volume_matches.len()
    );
    for (i, measurement) in volume_matches.iter().enumerate() {
        println!(
            "  {}. {} → \"{}\"",
            i + 1,
            measurement.text,
            measurement.ingredient_name
        );
    }

    println!("\n");

    // Example 4: Complex recipe with multiple ingredients per line
    println!("🔍 Example 4: Complex Recipe Analysis");
    println!("-------------------------------------");

    let complex_detector = MeasurementDetector::new()?;
    let complex_recipe = r#"
    Gourmet Salad Dressing

    Ingredients:
    1/2 cup mayonnaise, 1/4 cup sour cream, 2 tablespoons Dijon mustard
    1 tablespoon lemon juice, 1 teaspoon Worcestershire sauce, salt and pepper to taste
    2 cloves garlic, minced, 1/4 cup fresh parsley, chopped
    "#;

    let complex_matches = complex_detector.extract_ingredient_measurements(complex_recipe);

    println!(
        "Found {} measurements in complex recipe:",
        complex_matches.len()
    );
    for (i, measurement) in complex_matches.iter().enumerate() {
        println!(
            "  {}. {} → \"{}\" (line {})",
            i + 1,
            measurement.text,
            measurement.ingredient_name,
            measurement.line_number + 1
        );
    }

    println!("\n");

    // Example 5: Measurement extraction and unique units
    println!("📊 Example 5: Measurement Statistics");
    println!("------------------------------------");

    let stats_detector = MeasurementDetector::new()?;
    let shopping_list = r#"
    Weekly Grocery List:
    2 lbs ground beef
    1 gallon milk
    5 lbs potatoes
    2 dozen eggs
    1 lb butter
    3 cups rice
    2 lbs carrots
    1 pint heavy cream
    "#;

    let units = stats_detector.get_unique_units(shopping_list);
    let measurements = stats_detector.extract_ingredient_measurements(shopping_list);

    println!("Total measurements found: {}", measurements.len());
    println!("Unique measurement units: {}", units.len());
    println!("Units found:");
    for unit in &units {
        println!("  - {}", unit);
    }

    println!("\n");

    // Example 6: Post-processing disabled for comparison
    println!("⚙️  Example 6: Post-Processing Comparison");
    println!("-----------------------------------------");

    let no_postprocess_config = MeasurementConfig {
        enable_ingredient_postprocessing: false,
        ..Default::default()
    };

    let no_pp_detector = MeasurementDetector::with_config(no_postprocess_config)?;
    let comparison_text =
        "2 cups of all-purpose flour\n1 tablespoon of olive oil\n500g of dark chocolate";

    println!("With post-processing:");
    let with_pp = basic_detector.extract_ingredient_measurements(comparison_text);
    for measurement in &with_pp {
        println!(
            "  {} → \"{}\"",
            measurement.text, measurement.ingredient_name
        );
    }

    println!("\nWithout post-processing:");
    let without_pp = no_pp_detector.extract_ingredient_measurements(comparison_text);
    for measurement in &without_pp {
        println!(
            "  {} → \"{}\"",
            measurement.text, measurement.ingredient_name
        );
    }

    println!("\n");

    // Example 7: Error handling
    println!("🚨 Example 7: Error Handling");
    println!("----------------------------");

    match MeasurementDetector::with_pattern(r"[invalid regex") {
        Ok(_) => println!("Unexpected success with invalid regex"),
        Err(e) => println!("Expected error with invalid regex: {}", e),
    }

    println!("\n✨ Recipe parsing examples completed!");

    Ok(())
}
