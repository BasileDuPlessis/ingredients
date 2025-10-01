//! # Localization Tests
//!
//! This module contains unit tests for the localization functionality,
//! testing message retrieval and formatting with various edge cases.

use ingredients::localization::LocalizationManager;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_localization() -> LocalizationManager {
        // Create a new localization manager for each test
        LocalizationManager::new().expect("Failed to create localization manager")
    }

    #[test]
    fn test_get_message_existing_key() {
        let manager = setup_localization();

        let message = manager.get_message_in_language("help-commands", "en", None);
        assert!(!message.is_empty());
        assert!(message.contains("Commands"));
    }

    #[test]
    fn test_get_message_nonexistent_key() {
        let manager = setup_localization();

        let message = manager.get_message_in_language("nonexistent-key", "en", None);
        assert!(message.starts_with("Missing translation:"));
    }

    #[test]
    fn test_get_message_unsupported_language() {
        let manager = setup_localization();

        let message = manager.get_message_in_language("help-commands", "unsupported", None);
        // Should fall back to English
        assert!(!message.is_empty());
        assert!(message.contains("Commands"));
    }

    #[test]
    fn test_get_message_with_args() {
        let manager = setup_localization();

        let mut args = HashMap::new();
        args.insert("recipe_name", "Test Recipe");
        args.insert("ingredient_count", "5");

        let message = manager.get_message_in_language("recipe-complete", "en", Some(&args));
        assert!(!message.is_empty());
        assert!(message.contains("Test Recipe"));
        assert!(message.contains("5"));
    }

    #[test]
    fn test_get_message_missing_args() {
        let manager = setup_localization();

        // Test with missing required args - should handle gracefully
        let message = manager.get_message_in_language("recipe-complete", "en", None);
        // Either returns the message with placeholder or handles error
        assert!(!message.is_empty());
    }

    #[test]
    fn test_french_localization() {
        let manager = setup_localization();

        let message = manager.get_message_in_language("help-commands", "fr", None);
        assert!(!message.is_empty());
        // French message should be different from English
        let english_message = manager.get_message_in_language("help-commands", "en", None);
        assert_ne!(message, english_message);
    }

    #[test]
    fn test_language_detection() {
        setup_localization();
        use ingredients::localization::detect_language;

        assert_eq!(detect_language(Some("en")), "en");
        assert_eq!(detect_language(Some("en-US")), "en");
        assert_eq!(detect_language(Some("fr")), "fr");
        assert_eq!(detect_language(Some("fr-CA")), "fr");
        assert_eq!(detect_language(None), "en"); // Default to English
        assert_eq!(detect_language(Some("unsupported")), "en"); // Fallback to English
    }

    #[test]
    fn test_convenience_functions() {
        // Initialize the global localization manager for this test
        ingredients::localization::init_localization().expect("Failed to initialize localization");

        // Test t_lang function
        let message = ingredients::localization::t_lang("help-commands", Some("en"));
        assert!(!message.is_empty());

        // Test t_args_lang function
        let args = vec![("recipe_name", "Test Recipe"), ("ingredient_count", "3")];
        let message_with_args =
            ingredients::localization::t_args_lang("recipe-complete", &args, Some("en"));
        assert!(!message_with_args.is_empty());
        assert!(message_with_args.contains("Test Recipe"));
        assert!(message_with_args.contains("3"));
    }
}
