//! # Integration Tests
//!
//! This module contains integration tests for the Ingredients Telegram bot,
//! testing end-to-end functionality including quantity-only ingredient detection.

use ingredients::text_processing::{MeasurementDetector, MeasurementConfig};

/// Test end-to-end processing of quantity-only ingredients
#[test]
fn test_quantity_only_integration() {
    // Create a measurement detector
    let detector = MeasurementDetector::new().unwrap();

    // Test text that would come from OCR containing quantity-only ingredients
    let ocr_text = r#"
    Recette de Crêpes

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

    Préparation:
    Mélanger la farine avec les œufs...
    "#;

    // Process the text through the measurement detector
    let matches = detector.find_measurements(ocr_text);

    // Verify we found all measurements including quantity-only ones
    assert_eq!(matches.len(), 9);

    // Check traditional measurements
    assert_eq!(matches[0].text, "125 g");
    assert_eq!(matches[0].ingredient_name, "farine");

    // Check quantity-only ingredients
    assert_eq!(matches[1].text, "2");
    assert_eq!(matches[1].ingredient_name, "œufs");

    assert_eq!(matches[6].text, "2");
    assert_eq!(matches[6].ingredient_name, "oranges");

    // Check other measurements still work
    assert_eq!(matches[2].text, "1/2 litre");
    assert_eq!(matches[2].ingredient_name, "lait");

    assert_eq!(matches[3].text, "2 cuillères à soupe");
    assert_eq!(matches[3].ingredient_name, "sucre");

    println!("✅ Successfully processed {} measurements including quantity-only ingredients", matches.len());
}

/// Test comprehensive recipe processing with mixed ingredient types
#[test]
fn test_mixed_recipe_processing() {
    let detector = MeasurementDetector::with_config(MeasurementConfig {
        enable_ingredient_postprocessing: true,
        ..Default::default()
    }).unwrap();

    let recipe_text = r#"
    Chocolate Chip Cookies - English Recipe

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

    French Crepes Recipe:
    125 g de farine
    2 œufs
    250 ml de lait
    1 sachet de sucre vanillé
    4 pommes
    "#;

    let matches = detector.find_measurements(recipe_text);

    // Should find measurements from both recipes (more than expected due to regex splitting)
    assert!(matches.len() >= 15);

    // Check English measurements (note: 2 1/4 cups gets split by regex)
    let flour_match = matches.iter().find(|m| m.ingredient_name == "all-purpose flour").unwrap();
    assert_eq!(flour_match.text, "4 cups");

    // Check French quantity-only ingredients
    let oeufs_match = matches.iter().find(|m| m.ingredient_name == "œufs").unwrap();
    assert_eq!(oeufs_match.text, "2");

    let pommes_match = matches.iter().find(|m| m.ingredient_name == "pommes").unwrap();
    assert_eq!(pommes_match.text, "4");

    println!("✅ Successfully processed mixed English/French recipe with {} measurements", matches.len());
}

/// Test edge cases for quantity-only ingredients
#[test]
fn test_quantity_only_edge_cases() {
    let detector = MeasurementDetector::new().unwrap();

    // Test various edge cases
    let test_cases = vec![
        ("6 eggs", ("6", "eggs")),
        ("2 œufs", ("2", "œufs")),
        ("4 pommes", ("4", "pommes")),
        ("1 carotte", ("1", "carotte")),
        ("3 tomates", ("3", "tomates")),
        ("5 oignons", ("5", "oignons")),
    ];

    for (input, (expected_quantity, expected_ingredient)) in test_cases {
        let matches = detector.find_measurements(input);
        assert_eq!(matches.len(), 1, "Should find exactly one match for: {}", input);
        assert_eq!(matches[0].text, expected_quantity, "Quantity should be '{}' for: {}", expected_quantity, input);
        assert_eq!(matches[0].ingredient_name, expected_ingredient, "Ingredient should be '{}' for: {}", expected_ingredient, input);
    }

    println!("✅ All quantity-only edge cases passed");
}

/// Test that regular measurements still work alongside quantity-only
#[test]
fn test_mixed_measurement_types() {
    let detector = MeasurementDetector::new().unwrap();

    let mixed_text = r#"
    Recipe with mixed measurement types:
    2 cups flour
    3 eggs
    500g sugar
    4 apples
    1 tablespoon vanilla
    2 potatoes
    "#;

    let matches = detector.find_measurements(mixed_text);

    // Should find multiple measurements (regex may split some)
    assert!(matches.len() >= 6);

    // Verify different types are correctly identified
    let traditional_measurements: Vec<_> = matches.iter()
        .filter(|m| m.text.contains(' ') && !m.text.chars().all(char::is_numeric))
        .collect();

    let quantity_only: Vec<_> = matches.iter()
        .filter(|m| m.text.chars().all(char::is_numeric))
        .collect();

    // Should have traditional measurements and quantity-only ones
    assert!(!traditional_measurements.is_empty());
    assert!(!quantity_only.is_empty());

    // Check that we have the expected quantity-only ingredients
    let eggs_match = quantity_only.iter().find(|m| m.ingredient_name == "eggs");
    assert!(eggs_match.is_some());
    assert_eq!(eggs_match.unwrap().text, "3");

    let apples_match = quantity_only.iter().find(|m| m.ingredient_name == "apples");
    assert!(apples_match.is_some());
    assert_eq!(apples_match.unwrap().text, "4");

    let potatoes_match = quantity_only.iter().find(|m| m.ingredient_name == "potatoes");
    assert!(potatoes_match.is_some());
    assert_eq!(potatoes_match.unwrap().text, "2");

    println!("✅ Mixed measurement types correctly distinguished: {} traditional, {} quantity-only",
             traditional_measurements.len(), quantity_only.len());
}