//! Recipe name dialogue module for handling conversation state with users.

use serde::{Deserialize, Serialize};
use teloxide::dispatching::dialogue::{Dialogue, InMemStorage};
use crate::text_processing::MeasurementMatch;

/// Represents the conversation state for recipe name dialogue
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum RecipeDialogueState {
    #[default]
    Start,
    WaitingForRecipeName {
        extracted_text: String,
        ingredients: Vec<MeasurementMatch>,
        language_code: Option<String>,
    },
}

/// Type alias for our recipe dialogue
pub type RecipeDialogue = Dialogue<RecipeDialogueState, InMemStorage<RecipeDialogueState>>;

/// Validates a recipe name input
pub fn validate_recipe_name(name: &str) -> Result<String, &'static str> {
    let trimmed = name.trim();
    
    if trimmed.is_empty() {
        return Err("empty");
    }
    
    if trimmed.len() > 255 {
        return Err("too_long");
    }
    
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    
    #[test]
    fn test_recipe_name_trimming() {
        let result = validate_recipe_name("  Test Recipe  ");
        assert_eq!(result.unwrap(), "Test Recipe");
    }
}