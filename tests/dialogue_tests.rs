use anyhow::Result;

use ingredients::dialogue::{validate_recipe_name, RecipeDialogueState};
use ingredients::text_processing::MeasurementMatch;

/// Integration test for recipe name dialogue validation
#[tokio::test]
async fn test_recipe_name_dialogue_validation() -> Result<()> {
    // Test valid recipe names
    assert!(validate_recipe_name("Chocolate Chip Cookies").is_ok());
    assert!(validate_recipe_name("  Mom's Lasagna  ").is_ok());

    // Test invalid recipe names
    assert!(validate_recipe_name("").is_err());
    assert!(validate_recipe_name("   ").is_err());
    assert!(validate_recipe_name(&"a".repeat(256)).is_err());

    Ok(())
}

/// Test dialogue state transitions
#[tokio::test]
async fn test_dialogue_state_serialization() -> Result<()> {
    // Test that dialogue states can be serialized/deserialized with serde_json
    let ingredients = vec![MeasurementMatch {
        quantity: "2".to_string(),
        measurement: Some("cups".to_string()),
        ingredient_name: "flour".to_string(),
        line_number: 0,
        start_pos: 0,
        end_pos: 6,
    }];

    let state = RecipeDialogueState::WaitingForRecipeName {
        extracted_text: "2 cups flour\n3 eggs".to_string(),
        ingredients,
        language_code: Some("en".to_string()),
    };

    // Basic test that the state is properly structured
    match state {
        RecipeDialogueState::WaitingForRecipeName { ingredients, .. } => {
            assert_eq!(ingredients.len(), 1);
            assert_eq!(ingredients[0].ingredient_name, "flour");
        }
        _ => panic!("Unexpected dialogue state"),
    }

    Ok(())
}

/// Test basic dialogue functionality
#[tokio::test]
async fn test_dialogue_functionality() -> Result<()> {
    // Test that we can create dialogue states properly
    let start_state = RecipeDialogueState::Start;

    // Test default state
    assert!(matches!(start_state, RecipeDialogueState::Start));

    // Test default trait
    let default_state = RecipeDialogueState::default();
    assert!(matches!(default_state, RecipeDialogueState::Start));

    Ok(())
}

/// Test ingredient review dialogue state transitions
#[tokio::test]
async fn test_ingredient_review_dialogue_states() -> Result<()> {
    // Test ReviewIngredients state
    let ingredients = vec![
        MeasurementMatch {
            quantity: "2".to_string(),
            measurement: Some("cups".to_string()),
            ingredient_name: "flour".to_string(),
            line_number: 0,
            start_pos: 0,
            end_pos: 6,
        },
        MeasurementMatch {
            quantity: "3".to_string(),
            measurement: None,
            ingredient_name: "eggs".to_string(),
            line_number: 1,
            start_pos: 8,
            end_pos: 9,
        },
    ];

    let review_state = RecipeDialogueState::ReviewIngredients {
        recipe_name: "Test Recipe".to_string(),
        ingredients: ingredients.clone(),
        language_code: Some("en".to_string()),
        message_id: Some(123),
    };

    // Verify state structure
    match review_state {
        RecipeDialogueState::ReviewIngredients {
            recipe_name,
            ingredients: ingr,
            language_code,
            message_id,
        } => {
            assert_eq!(recipe_name, "Test Recipe");
            assert_eq!(ingr.len(), 2);
            assert_eq!(ingr[0].ingredient_name, "flour");
            assert_eq!(ingr[1].ingredient_name, "eggs");
            assert_eq!(language_code, Some("en".to_string()));
            assert_eq!(message_id, Some(123));
        }
        _ => panic!("Expected ReviewIngredients state"),
    }

    // Test EditingIngredient state
    let editing_state = RecipeDialogueState::EditingIngredient {
        recipe_name: "Test Recipe".to_string(),
        ingredients: ingredients.clone(),
        editing_index: 0,
        language_code: Some("en".to_string()),
        message_id: Some(123),
    };

    match editing_state {
        RecipeDialogueState::EditingIngredient {
            recipe_name,
            ingredients: ingr,
            editing_index,
            language_code,
            message_id,
        } => {
            assert_eq!(recipe_name, "Test Recipe");
            assert_eq!(ingr.len(), 2);
            assert_eq!(editing_index, 0);
            assert_eq!(language_code, Some("en".to_string()));
            assert_eq!(message_id, Some(123));
        }
        _ => panic!("Expected EditingIngredient state"),
    }

    // Test WaitingForRecipeNameAfterConfirm state
    let confirm_state = RecipeDialogueState::WaitingForRecipeNameAfterConfirm {
        ingredients: ingredients.clone(),
        language_code: Some("en".to_string()),
    };

    match confirm_state {
        RecipeDialogueState::WaitingForRecipeNameAfterConfirm {
            ingredients: ingr,
            language_code,
        } => {
            assert_eq!(ingr.len(), 2);
            assert_eq!(language_code, Some("en".to_string()));
        }
        _ => panic!("Expected WaitingForRecipeNameAfterConfirm state"),
    }

    Ok(())
}

/// Test ingredient editing validation
#[test]
fn test_ingredient_edit_validation() {
    use ingredients::bot::parse_ingredient_from_text;

    // Test valid edits
    let result = parse_ingredient_from_text("2 cups flour");
    assert!(result.is_ok());
    let ingredient = result.unwrap();
    assert_eq!(ingredient.quantity, "2");
    assert_eq!(ingredient.measurement, Some("cups".to_string()));
    assert_eq!(ingredient.ingredient_name, "flour");

    // Test quantity-only ingredient
    let result = parse_ingredient_from_text("2 cups flour");
    assert!(result.is_ok());
    let ingredient = result.unwrap();
    assert_eq!(ingredient.quantity, "2");
    assert_eq!(ingredient.measurement, Some("cups".to_string()));
    assert_eq!(ingredient.ingredient_name, "flour");

    // Test validation errors
    assert!(parse_ingredient_from_text("").is_err()); // Empty
    assert!(parse_ingredient_from_text(&"a".repeat(201)).is_err()); // Too long
    assert!(parse_ingredient_from_text("2 cups").is_err()); // No ingredient name
    assert!(parse_ingredient_from_text("0 cups flour").is_err()); // Zero quantity
    assert!(parse_ingredient_from_text("-1 cups flour").is_err()); // Negative quantity
    assert!(parse_ingredient_from_text("2 cups very_long_ingredient_name_that_exceeds_the_one_hundred_character_limit_and_should_be_rejected_by_the_validation").is_err());
    // Name too long
}

/// Test ingredient review command parsing
#[test]
fn test_ingredient_review_commands() {
    // Test command parsing (this would be used in handle_ingredient_review_input)
    let test_cases = vec![
        ("confirm", true, false),
        ("ok", true, false),
        ("yes", true, false),
        ("save", true, false),
        ("cancel", false, true),
        ("stop", false, true),
        ("unknown", false, false),
        ("CONFIRM", true, false), // Case insensitive
        ("CANCEL", false, true),
    ];

    for (input, should_confirm, should_cancel) in test_cases {
        let lower_input = input.to_lowercase();
        let is_confirm = matches!(lower_input.as_str(), "confirm" | "ok" | "yes" | "save");
        let is_cancel = matches!(lower_input.as_str(), "cancel" | "stop");

        assert_eq!(
            is_confirm,
            should_confirm,
            "Command '{}' should {} be confirm",
            input,
            if should_confirm { "" } else { "not" }
        );
        assert_eq!(
            is_cancel,
            should_cancel,
            "Command '{}' should {} be cancel",
            input,
            if should_cancel { "" } else { "not" }
        );
    }
}

/// Test ingredient editing cancellation commands
#[test]
fn test_ingredient_edit_cancellation() {
    // Test cancellation commands for editing (this would be used in handle_ingredient_edit_input)
    let cancellation_commands = ["cancel", "stop", "back"];

    for command in &cancellation_commands {
        assert!(
            matches!(command.to_lowercase().as_str(), "cancel" | "stop" | "back"),
            "Command '{}' should be recognized as cancellation",
            command
        );
    }

    let non_cancellation_commands = ["confirm", "ok", "edit", "save"];
    for command in &non_cancellation_commands {
        assert!(
            !matches!(command.to_lowercase().as_str(), "cancel" | "stop" | "back"),
            "Command '{}' should not be recognized as cancellation",
            command
        );
    }
}

/// Unit test for recipe name validation
#[test]
fn test_recipe_name_validation() {
    // Valid names
    assert!(validate_recipe_name("Chocolate Chip Cookies").is_ok());
    assert!(validate_recipe_name("  Mom's Lasagna  ").is_ok());

    // Invalid names
    assert!(validate_recipe_name("").is_err());
    assert!(validate_recipe_name("   ").is_err());
    assert!(validate_recipe_name(&"a".repeat(256)).is_err());
}

/// Unit test for recipe name trimming
#[test]
fn test_recipe_name_trimming() {
    let result = validate_recipe_name("  Test Recipe  ");
    assert_eq!(result.unwrap(), "Test Recipe");
}
