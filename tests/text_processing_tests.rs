#[cfg(test)]
mod tests {
    use ingredients::text_processing::{MeasurementDetector, MeasurementConfig};

    fn create_detector() -> MeasurementDetector {
        MeasurementDetector::new().unwrap()
    }

    #[test]
    fn test_measurement_detector_creation() {
        let detector = create_detector();
        assert!(!detector.pattern_str().is_empty());
    }

    #[test]
    fn test_basic_measurement_detection() {
        let detector = create_detector();

        // Test basic measurements
        assert!(detector.has_measurements("2 cups flour"));
        assert!(detector.has_measurements("1 tablespoon sugar"));
        assert!(detector.has_measurements("500g butter"));
        assert!(detector.has_measurements("1 kg tomatoes"));
        assert!(detector.has_measurements("250 ml milk"));
    }

    #[test]
    fn test_no_measurement_detection() {
        let detector = create_detector();

        assert!(!detector.has_measurements("some flour"));
        assert!(!detector.has_measurements("add salt"));
        assert!(!detector.has_measurements(""));
    }

    #[test]
    fn test_extract_measurement_lines() {
        let detector = create_detector();
        let text = "2 cups flour\n1 tablespoon sugar\nsome salt\nto taste";

        let lines = detector.extract_measurement_lines(text);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], (0, "2 cups flour".to_string()));
        assert_eq!(lines[1], (1, "1 tablespoon sugar".to_string()));
    }

    #[test]
    fn test_find_measurements_with_positions() {
        let detector = create_detector();
        let text = "Mix 2 cups flour with 1 tbsp sugar";

        let matches = detector.find_measurements(text);

        assert_eq!(matches.len(), 2);

        // First match: "2 cups"
        assert_eq!(matches[0].text, "2 cups");
        assert_eq!(matches[0].line_number, 0);
        assert_eq!(matches[0].start_pos, 4);
        assert_eq!(matches[0].end_pos, 10);

        // Second match: "1 tbsp"
        assert_eq!(matches[1].text, "1 tbsp");
        assert_eq!(matches[1].line_number, 0);
    }

    #[test]
    fn test_french_measurements() {
        let detector = create_detector();

        // Test French measurements
        assert!(detector.has_measurements("2 tasses de farine"));
        assert!(detector.has_measurements("1 cuillère à soupe de sucre"));
        assert!(detector.has_measurements("500 g de beurre"));
        assert!(detector.has_measurements("1 kg de tomates"));
    }

    #[test]
    fn test_comprehensive_french_measurements() {
        let detector = create_detector();

        // Test volume measurements
        assert!(detector.has_measurements("2 tasses de lait"));
        assert!(detector.has_measurements("1 cuillère à café de sel"));
        assert!(detector.has_measurements("3 cuillères à soupe d'huile"));
        assert!(detector.has_measurements("250 ml d'eau"));
        assert!(detector.has_measurements("1 litre de jus"));

        // Test weight measurements
        assert!(detector.has_measurements("500 grammes de sucre"));
        assert!(detector.has_measurements("1 kilogramme de pommes"));
        assert!(detector.has_measurements("200 g de chocolat"));

        // Test count measurements (excluding œufs which are ingredients, not measurements)
        assert!(detector.has_measurements("2 tranches de pain"));
        assert!(detector.has_measurements("1 boîte de conserve"));
        assert!(detector.has_measurements("4 morceaux de poulet"));
        assert!(detector.has_measurements("1 sachet de levure"));
        assert!(detector.has_measurements("2 paquets de pâtes"));
        assert!(detector.has_measurements("1 poignée d'amandes"));
        assert!(detector.has_measurements("3 gousses d'ail"));
        assert!(detector.has_measurements("1 brin de persil"));
        assert!(detector.has_measurements("2 feuilles de laurier"));
        assert!(detector.has_measurements("1 bouquet de thym"));
    }

    #[test]
    fn test_abbreviations() {
        let detector = create_detector();

        // Test abbreviations
        assert!(detector.has_measurements("1 tsp salt"));
        assert!(detector.has_measurements("2 tbsp oil"));
        assert!(detector.has_measurements("1 lb beef"));
        assert!(detector.has_measurements("8 oz water"));
    }

    #[test]
    fn test_plural_forms() {
        let detector = create_detector();

        // Test plural forms
        assert!(detector.has_measurements("2 cups"));
        assert!(detector.has_measurements("1 tablespoon"));
        assert!(detector.has_measurements("3 teaspoons"));
        assert!(detector.has_measurements("4 ounces"));
    }

    #[test]
    fn test_decimal_numbers() {
        let detector = create_detector();

        // Test decimal numbers
        assert!(detector.has_measurements("2.5 cups flour"));
        assert!(detector.has_measurements("0.5 kg sugar"));
        assert!(detector.has_measurements("1.25 liters milk"));
    }

    #[test]
    fn test_count_measurements() {
        let detector = create_detector();

        // Test count-based measurements (excluding eggs which are ingredients, not measurements)
        assert!(detector.has_measurements("2 slices bread"));
        assert!(detector.has_measurements("1 can tomatoes"));
        assert!(detector.has_measurements("4 pieces chicken"));
        assert!(detector.has_measurements("3 sachets yeast"));
        assert!(detector.has_measurements("2 paquets pasta"));
    }

    #[test]
    fn test_unique_units_extraction() {
        let detector = create_detector();
        let text = "2 cups flour\n1 cup sugar\n500g butter\n200g flour";

        let units = detector.get_unique_units(text);

        // Should contain the measurement parts
        assert!(units.iter().any(|u| u.contains("cups")));
        assert!(units.iter().any(|u| u.contains("cup")));
        assert!(units.iter().any(|u| u.contains("g")));
    }

    #[test]
    fn test_multi_line_text() {
        let detector = create_detector();
        let text = "Ingredients:\n2 cups flour\n1 tablespoon sugar\n1 teaspoon salt\n\nInstructions:\nMix well";

        let matches = detector.find_measurements(text);

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].line_number, 1); // "2 cups flour"
        assert_eq!(matches[1].line_number, 2); // "1 tablespoon sugar"
        assert_eq!(matches[2].line_number, 3); // "1 teaspoon salt"
    }

    #[test]
    fn test_custom_pattern() {
        let pattern = r"\b\d+\s*(?:cups?|tablespoons?)\b";
        let detector = MeasurementDetector::with_pattern(pattern).unwrap();

        assert!(detector.has_measurements("2 cups flour"));
        assert!(detector.has_measurements("1 tablespoon sugar"));
        assert!(!detector.has_measurements("500g butter")); // g not in custom pattern
    }

    #[test]
    fn test_case_insensitive_matching() {
        let detector = create_detector();

        assert!(detector.has_measurements("2 CUPS flour"));
        assert!(detector.has_measurements("1 Tablespoon sugar"));
        assert!(detector.has_measurements("500G butter"));
    }

    #[test]
    fn test_ingredient_name_extraction() {
        let detector = create_detector();

        // Test basic ingredient name extraction
        let matches = detector.find_measurements("2 cups flour\n1 tablespoon sugar\n500g butter");

        assert_eq!(matches.len(), 3);

        assert_eq!(matches[0].text, "2 cups");
        assert_eq!(matches[0].ingredient_name, "flour");

        assert_eq!(matches[1].text, "1 tablespoon");
        assert_eq!(matches[1].ingredient_name, "sugar");

        assert_eq!(matches[2].text, "500g");
        assert_eq!(matches[2].ingredient_name, "butter");
    }

    #[test]
    fn test_french_ingredient_name_extraction() {
        let detector = create_detector();

        // Test French ingredient name extraction (with post-processing enabled by default)
        let matches =
            detector.find_measurements("250 g de farine\n1 litre de lait\n2 tranches de pain");

        assert_eq!(matches.len(), 3);

        assert_eq!(matches[0].text, "250 g");
        assert_eq!(matches[0].ingredient_name, "farine"); // "de " removed by post-processing

        assert_eq!(matches[1].text, "1 litre");
        assert_eq!(matches[1].ingredient_name, "lait"); // "de " removed by post-processing

        assert_eq!(matches[2].text, "2 tranches");
        assert_eq!(matches[2].ingredient_name, "pain"); // "de " removed by post-processing
    }

    #[test]
    fn test_multi_word_ingredient_names() {
        let detector = create_detector();

        // Test multi-word ingredient names
        let matches = detector.find_measurements(
            "2 cups all-purpose flour\n1 teaspoon baking powder\n500g unsalted butter",
        );

        assert_eq!(matches.len(), 3);

        assert_eq!(matches[0].text, "2 cups");
        assert_eq!(matches[0].ingredient_name, "all-purpose flour");

        assert_eq!(matches[1].text, "1 teaspoon");
        assert_eq!(matches[1].ingredient_name, "baking powder");

        assert_eq!(matches[2].text, "500g");
        assert_eq!(matches[2].ingredient_name, "unsalted butter");
    }

    #[test]
    fn test_measurement_at_end_of_line() {
        let detector = create_detector();

        // Test when measurement is at the end of the line (no ingredient name)
        let matches = detector.find_measurements("Add 2 cups\nMix 1 tablespoon\nBake at 350");

        assert_eq!(matches.len(), 2);

        assert_eq!(matches[0].text, "2 cups");
        assert_eq!(matches[0].ingredient_name, "");

        assert_eq!(matches[1].text, "1 tablespoon");
        assert_eq!(matches[1].ingredient_name, "");
    }

    #[test]
    fn test_regex_pattern_validation() {
        let detector = create_detector();

        // Test that the regex correctly identifies various measurement formats
        let test_cases = vec![
            // Basic volume measurements
            ("1 cup", true),
            ("2 cups", true),
            ("1.5 cups", true),
            ("0.25 cups", true),
            // Weight measurements
            ("500g", true),
            ("1.5kg", true),
            ("250 grams", true),
            ("2 pounds", true),
            // Volume measurements
            ("1 tablespoon", true),
            ("2 teaspoons", true),
            ("1 tsp", true),
            ("2 tbsp", true),
            ("500 ml", true),
            ("1 liter", true),
            // Count measurements (excluding eggs/œufs which are ingredients)
            ("2 slices", true),
            ("1 can", true),
            ("4 pieces", true),
            ("3 sachets", true),
            // French measurements
            ("2 tasses", true),
            ("1 cuillère à soupe", true),
            ("250 g", true),
            // Non-measurements (should not match)
            ("recipe", false),
            ("ingredients", false),
            ("flour", false),
            ("sugar", false),
            ("salt", false),
            ("", false),
            ("123", false), // Just a number, no unit
            ("abc", false),
            ("cupboard", false),      // Contains "cup" but not as measurement
            ("tablespoonful", false), // Contains "tablespoon" but not as measurement
        ];

        for (text, should_match) in test_cases {
            assert_eq!(
                detector.has_measurements(text),
                should_match,
                "Pattern validation failed for: '{}' (expected: {})",
                text,
                should_match
            );
        }
    }

    #[test]
    fn test_regex_capture_groups() {
        let detector = create_detector();

        // Test that the regex captures complete measurement units
        let test_text = "Mix 2 cups flour with 1 tbsp sugar and 500g butter";
        let matches = detector.find_measurements(test_text);

        assert_eq!(matches.len(), 3);

        // Verify each match captures the complete measurement
        assert_eq!(matches[0].text, "2 cups");
        assert_eq!(matches[1].text, "1 tbsp");
        assert_eq!(matches[2].text, "500g");

        // Verify positions are correct
        assert_eq!(matches[0].start_pos, 4); // "Mix 2" -> position after "Mix "
        assert_eq!(matches[0].end_pos, 10); // "Mix 2 cups" -> ends at position 10
    }

    #[test]
    fn test_regex_boundary_conditions() {
        let detector = create_detector();

        // Test word boundaries and edge cases
        let boundary_tests = vec![
            ("1cup", true),    // No space between number and unit (technically matches pattern)
            ("cup1", false),   // Unit before number
            ("1 cup.", true),  // Period after measurement
            ("(1 cup)", true), // Parentheses around measurement
            ("1 cup,", true),  // Comma after measurement
            ("1 cup;", true),  // Semicolon after measurement
            ("cup of flour", false), // "cup" without number
            ("cups", false),   // Just unit, no number
            ("1", false),      // Just number, no unit
        ];

        for (text, should_match) in boundary_tests {
            assert_eq!(
                detector.has_measurements(text),
                should_match,
                "Boundary test failed for: '{}' (expected: {})",
                text,
                should_match
            );
        }
    }

    #[test]
    fn test_regex_case_insensitivity() {
        let detector = create_detector();

        // Test that the regex is case insensitive
        let case_tests = vec![
            "2 CUPS flour",
            "2 Cups flour",
            "2 cups flour",
            "500G butter",
            "500g butter",
            "1 TBSP sugar",
            "1 tbsp sugar",
            "1 Tablespoon sugar",
        ];

        for text in case_tests {
            assert!(
                detector.has_measurements(text),
                "Case insensitivity test failed for: '{}'",
                text
            );
        }
    }

    #[test]
    fn test_regex_french_accents() {
        let detector = create_detector();

        // Test that French measurements with accents work correctly
        let french_tests = vec![
            "1 cuillère à café",
            "2 cuillères à soupe",
            "1 kilogramme",
            "2 grammes",
            "1 millilitre",
            "2 litres",
            "1 tranche",
            "2 morceaux",
            "1 boîte",
            "2 sachets",
        ];

        for text in french_tests {
            assert!(
                detector.has_measurements(text),
                "French accent test failed for: '{}'",
                text
            );
        }
    }

    #[test]
    fn test_ingredient_name_postprocessing() {
        let config = MeasurementConfig {
            enable_ingredient_postprocessing: true,
            max_ingredient_length: 50,
            ..Default::default()
        };
        let detector = MeasurementDetector::with_config(config).unwrap();

        // Test basic post-processing
        let matches =
            detector.find_measurements("2 cups of flour\n1 tablespoon sugar\n500g butter");

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].ingredient_name, "flour"); // "of " removed
        assert_eq!(matches[1].ingredient_name, "sugar");
        assert_eq!(matches[2].ingredient_name, "butter");
    }

    #[test]
    fn test_french_ingredient_postprocessing() {
        let config = MeasurementConfig {
            enable_ingredient_postprocessing: true,
            ..Default::default()
        };
        let detector = MeasurementDetector::with_config(config).unwrap();

        let matches =
            detector.find_measurements("250 g de farine\n1 litre du lait\n2 tasses d'eau");

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].ingredient_name, "farine"); // "de " removed
        assert_eq!(matches[1].ingredient_name, "lait"); // "du " removed
        assert_eq!(matches[2].ingredient_name, "eau"); // "d'" removed
    }

    #[test]
    fn test_ingredient_length_limit() {
        let config = MeasurementConfig {
            enable_ingredient_postprocessing: true,
            max_ingredient_length: 20,
            ..Default::default()
        };
        let detector = MeasurementDetector::with_config(config).unwrap();

        let matches = detector
            .find_measurements("2 cups of very-long-ingredient-name-that-should-be-truncated");

        assert_eq!(matches.len(), 1);
        assert!(matches[0].ingredient_name.len() <= 20);
        assert_eq!(matches[0].ingredient_name, "very-long-ingredient"); // "of " removed, then truncated at word boundary
    }

    #[test]
    fn test_postprocessing_disabled() {
        let config = MeasurementConfig {
            enable_ingredient_postprocessing: false,
            ..Default::default()
        };
        let detector = MeasurementDetector::with_config(config).unwrap();

        let matches = detector.find_measurements("2 cups of flour");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].ingredient_name, "of flour"); // No post-processing
    }

    #[test]
    fn test_fraction_measurements() {
        let detector = create_detector();

        // Test fraction measurements
        assert!(detector.has_measurements("1/2 cup flour"));
        assert!(detector.has_measurements("3/4 teaspoon salt"));
        assert!(detector.has_measurements("1/4 kg sugar"));
        assert!(detector.has_measurements("2/3 litre milk"));
        assert!(detector.has_measurements("1/8 teaspoon vanilla"));
    }

    #[test]
    fn test_fraction_ingredient_extraction() {
        let detector = create_detector();

        // Test fraction ingredient name extraction
        let matches = detector.find_measurements("1/2 cup flour\n3/4 teaspoon salt\n1/4 kg sugar");

        assert_eq!(matches.len(), 3);

        assert_eq!(matches[0].text, "1/2 cup");
        assert_eq!(matches[0].ingredient_name, "flour");

        assert_eq!(matches[1].text, "3/4 teaspoon");
        assert_eq!(matches[1].ingredient_name, "salt");

        assert_eq!(matches[2].text, "1/4 kg");
        assert_eq!(matches[2].ingredient_name, "sugar");
    }

    #[test]
    fn test_unicode_fraction_characters() {
        let detector = create_detector();

        // Test Unicode fraction characters (now supported!)
        assert!(detector.has_measurements("½ cup flour")); // Unicode ½ character
        assert!(detector.has_measurements("⅓ teaspoon salt")); // Unicode ⅓ character
        assert!(detector.has_measurements("¼ kg sugar")); // Unicode ¼ character

        // ASCII fractions still work
        assert!(detector.has_measurements("1/2 cup flour"));
        assert!(detector.has_measurements("1/3 teaspoon salt"));
        assert!(detector.has_measurements("1/4 kg sugar"));
    }
}