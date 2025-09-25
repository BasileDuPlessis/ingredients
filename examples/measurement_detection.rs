//! # Text Processing Examples
//!
//! This example demonstrates how to use the measurement detection functionality
//! implemented for issue #36.

// For this example, we'll implement a simple version inline
// In a real application, you'd import from the text_processing module

use regex::Regex;

/// Simple measurement detector for demonstration
struct MeasurementDetector {
    pattern: Regex,
}

impl MeasurementDetector {
    fn new() -> Result<Self, regex::Error> {
        let pattern_str = r#"(?i)\b\d*\.?\d+\s*(?:cup(?:s)?|teaspoon(?:s)?|tsp(?:\.?)|tablespoon(?:s)?|tbsp(?:\.?)|pint(?:s)?|quart(?:s)?|gallon(?:s)?|oz|ounce(?:s)?|lb(?:\.?)|pound(?:s)?|mg|g|gram(?:me)?s?|kg|kilogram(?:me)?s?|l|liter(?:s)?|litre(?:s)?|ml|millilitre(?:s)?|cc|cl|dl|cm3|mm3|cm²|mm²|slice(?:s)?|can(?:s)?|bottle(?:s)?|stick(?:s)?|packet(?:s)?|pkg|bag(?:s)?|dash(?:es)?|pinch(?:es)?|drop(?:s)?|cube(?:s)?|piece(?:s)?|handful(?:s)?|bar(?:s)?|sheet(?:s)?|serving(?:s)?|portion(?:s)?|tasse(?:s)?|cuillère(?:s)?(?:\s+à\s+(?:café|soupe))?|poignée(?:s)?|sachet(?:s)?|paquet(?:s)?|boîte(?:s)?|conserve(?:s)?|tranche(?:s)?|morceau(?:x)?|gousse(?:s)?|brin(?:s)?|feuille(?:s)?|bouquet(?:s)?|egg(?:s)?)\b"#;
        let pattern = Regex::new(pattern_str)?;
        Ok(Self { pattern })
    }

    fn find_measurements(&self, text: &str) -> Vec<String> {
        self.pattern
            .find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect()
    }

    fn has_measurements(&self, text: &str) -> bool {
        self.pattern.is_match(text)
    }

    fn extract_measurement_lines(&self, text: &str) -> Vec<(usize, String)> {
        text.lines()
            .enumerate()
            .filter(|(_, line)| self.pattern.is_match(line))
            .map(|(i, line)| (i, line.to_string()))
            .collect()
    }
}

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
            println!("  • Measurement {}: '{}'", i + 1, measurement);
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
        println!("  • '{}'", measurement);
    }

    Ok(())
}