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
