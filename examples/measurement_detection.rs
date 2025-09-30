//! # Text Processing Examples
//!
//! This example demonstrates how to use the measurement detection functionality
//! implemented for issue #36.

use ingredients::text_processing::MeasurementDetector;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a measurement detector
    let detector = MeasurementDetector::new()?;

    // Example ingredient text with various measurements
    let ingredient_text = r#"
Ingredients for chocolate cake:
- 2 cups all-purpose flour
- 1.5 teaspoons baking powder
- 0.5 cup unsweetened cocoa powder
- 1 cup granulated sugar
- 2 large eggs
- 1 teaspoon vanilla extract
- 0.75 cup milk
- 0.5 cup vegetable oil
- 1 cup boiling water

Instructions:
Mix dry ingredients, then add wet ingredients...
"#;

    println!("🔍 Analyzing ingredient text for measurements...\n");

    // Check if text contains measurements
    if detector.has_measurements(ingredient_text) {
        println!("✅ Text contains measurements!");

        // Find all measurements
        let measurements = detector.find_measurements(ingredient_text);
        println!("📏 Found {} measurements:", measurements.len());

        for (i, measurement) in measurements.iter().enumerate() {
            println!(
                "  • Measurement {}: '{}' (line {}, pos {}-{})",
                i + 1,
                measurement.text,
                measurement.line_number + 1,
                measurement.start_pos,
                measurement.end_pos
            );
        }

        // Extract lines containing measurements
        let measurement_lines = detector.extract_measurement_lines(ingredient_text);
        println!("\n📝 Lines containing measurements:");
        for (line_num, line) in measurement_lines {
            println!("  {}. {}", line_num + 1, line.trim());
        }
    } else {
        println!("❌ No measurements found in the text.");
    }

    // Test with French measurements
    let french_text = "Ingrédients :\n- 250 g de farine\n- 3 œufs\n- 1 litre de lait\n- 2 cuillères à soupe de sucre";
    println!("\n🇫🇷 Testing French measurements:");
    let french_measurements = detector.find_measurements(french_text);
    for measurement in french_measurements {
        println!(
            "  • '{}' (line {})",
            measurement.text,
            measurement.line_number + 1
        );
    }

    Ok(())
}
