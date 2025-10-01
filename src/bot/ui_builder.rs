//! UI Builder module for creating keyboards and formatting messages

use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

// Import localization
use crate::localization::t_lang;

// Import text processing types
use crate::text_processing::MeasurementMatch;

/// Format ingredients as a simple numbered list for review
pub fn format_ingredients_list(
    ingredients: &[MeasurementMatch],
    language_code: Option<&str>,
) -> String {
    let mut result = String::new();

    for (i, ingredient) in ingredients.iter().enumerate() {
        let ingredient_display = if ingredient.ingredient_name.is_empty() {
            format!("❓ {}", t_lang("unknown-ingredient", language_code))
        } else {
            ingredient.ingredient_name.clone()
        };

        let measurement_display = if let Some(ref unit) = ingredient.measurement {
            format!("{} {}", ingredient.quantity, unit)
        } else {
            ingredient.quantity.clone()
        };

        result.push_str(&format!(
            "{}. **{}** → {}\n",
            i + 1,
            measurement_display,
            ingredient_display
        ));
    }

    result
}

/// Create inline keyboard for ingredient review
pub fn create_ingredient_review_keyboard(
    ingredients: &[MeasurementMatch],
    language_code: Option<&str>,
) -> InlineKeyboardMarkup {
    let mut buttons = Vec::new();

    // Create Edit and Delete buttons for each ingredient
    for (i, ingredient) in ingredients.iter().enumerate() {
        let ingredient_display = if ingredient.ingredient_name.is_empty() {
            format!("❓ {}", t_lang("unknown-ingredient", language_code))
        } else {
            ingredient.ingredient_name.clone()
        };

        let measurement_display = if let Some(ref unit) = ingredient.measurement {
            format!("{} {}", ingredient.quantity, unit)
        } else {
            ingredient.quantity.clone()
        };

        let display_text = format!("{} → {}", measurement_display, ingredient_display);
        // Truncate if too long for button
        let button_text = if display_text.len() > 20 {
            format!("{}...", &display_text[..17])
        } else {
            display_text
        };

        buttons.push(vec![
            InlineKeyboardButton::callback(format!("✏️ {}", button_text), format!("edit_{}", i)),
            InlineKeyboardButton::callback(format!("🗑️ {}", button_text), format!("delete_{}", i)),
        ]);
    }

    // Add Confirm and Cancel buttons at the bottom
    buttons.push(vec![
        InlineKeyboardButton::callback(
            format!("✅ {}", t_lang("review-confirm", language_code)),
            "confirm".to_string(),
        ),
        InlineKeyboardButton::callback(
            format!("❌ {}", t_lang("cancel", language_code)),
            "cancel_review".to_string(),
        ),
    ]);

    InlineKeyboardMarkup::new(buttons)
}