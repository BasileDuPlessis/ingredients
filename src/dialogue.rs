//! Recipe name dialogue module for handling conversation state with users.

use crate::text_processing::MeasurementMatch;
use serde::{Deserialize, Serialize};
use teloxide::dispatching::dialogue::{Dialogue, InMemStorage};

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
    ReviewIngredients {
        recipe_name: String,
        ingredients: Vec<MeasurementMatch>,
        language_code: Option<String>,
        message_id: Option<i32>, // ID of the review message to edit
        extracted_text: String, // Store the original OCR text
    },
    EditingIngredient {
        recipe_name: String,
        ingredients: Vec<MeasurementMatch>,
        editing_index: usize,
        language_code: Option<String>,
        message_id: Option<i32>, // ID of the review message to edit after editing
        extracted_text: String, // Store the original OCR text
    },
    WaitingForRecipeNameAfterConfirm {
        ingredients: Vec<MeasurementMatch>,
        language_code: Option<String>,
        extracted_text: String, // Store the original OCR text
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
