//! Callback Handler module for processing inline keyboard callback queries

use anyhow::Result;
use sqlx::postgres::PgPool;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::{debug, error};

// Import localization
use crate::localization::t_lang;

// Import dialogue types
use crate::dialogue::{RecipeDialogue, RecipeDialogueState};

// Import UI builder functions
use super::ui_builder::{format_ingredients_list, create_ingredient_review_keyboard};

/// Handle callback queries from inline keyboards
pub async fn callback_handler(
    bot: Bot,
    q: teloxide::types::CallbackQuery,
    _pool: Arc<PgPool>,
    dialogue: RecipeDialogue,
) -> Result<()> {
    debug!(user_id = %q.from.id, "Received callback query from user");

    // Check dialogue state
    let dialogue_state = dialogue.get().await?;
    debug!(user_id = %q.from.id, dialogue_state = ?dialogue_state, "Retrieved dialogue state");

    match dialogue_state {
        Some(RecipeDialogueState::ReviewIngredients {
            recipe_name,
            mut ingredients,
            language_code: dialogue_lang_code,
            message_id,
            extracted_text,
        }) => {
            let data = q.data.as_deref().unwrap_or("");
            if let Some(msg) = &q.message {
                if data.starts_with("edit_") {
                    // Handle edit button - transition to editing state
                    let index: usize = data.strip_prefix("edit_").unwrap().parse().unwrap_or(0);
                    if index < ingredients.len() {
                        let ingredient = &ingredients[index];
                        let edit_prompt = format!(
                            "✏️ {}\n\n{}: **{} {}**\n\n{}",
                            t_lang("edit-ingredient-prompt", dialogue_lang_code.as_deref()),
                            t_lang("current-ingredient", dialogue_lang_code.as_deref()),
                            ingredient.quantity,
                            ingredient.measurement.as_deref().unwrap_or(""),
                            ingredient.ingredient_name
                        );
                        bot.send_message(msg.chat().id, edit_prompt)
                            .await?;

                        // Transition to editing state
                        dialogue
                            .update(RecipeDialogueState::EditingIngredient {
                                recipe_name: recipe_name.clone(),
                                ingredients: ingredients.clone(),
                                editing_index: index,
                                language_code: dialogue_lang_code.clone(),
                                message_id,
                                extracted_text: extracted_text.clone(),
                            })
                            .await?;
                    }
                } else if data.starts_with("delete_") {
                    // Handle delete button
                    let index: usize = data.strip_prefix("delete_").unwrap().parse().unwrap_or(0);

                    if index < ingredients.len() {
                        ingredients.remove(index);

                        // Check if all ingredients were deleted
                        if ingredients.is_empty() {
                            // All ingredients deleted - inform user and provide options
                            let empty_message = format!(
                                "🗑️ **{}**\n\n{}\n\n{}",
                                t_lang("review-title", dialogue_lang_code.as_deref()),
                                t_lang("review-no-ingredients", dialogue_lang_code.as_deref()),
                                t_lang("review-no-ingredients-help", dialogue_lang_code.as_deref())
                            );

                            let keyboard = vec![vec![
                                teloxide::types::InlineKeyboardButton::callback(
                                    t_lang("review-add-more", dialogue_lang_code.as_deref()),
                                    "add_more",
                                ),
                                teloxide::types::InlineKeyboardButton::callback(
                                    t_lang("cancel", dialogue_lang_code.as_deref()),
                                    "cancel_empty",
                                ),
                            ]];

                            // Edit the original message
                            match bot.edit_message_text(msg.chat().id, msg.id(), empty_message)
                                .reply_markup(teloxide::types::InlineKeyboardMarkup::new(keyboard))
                                .await {
                                Ok(_) => (),
                                Err(e) => error!(user_id = %q.from.id, error = %e, "Failed to edit message for empty ingredients"),
                            }
                        } else {
                            // Update the message with remaining ingredients
                            let review_message = format!(
                                "📝 **{}**\n\n{}\n\n{}",
                                t_lang("review-title", dialogue_lang_code.as_deref()),
                                t_lang("review-description", dialogue_lang_code.as_deref()),
                                format_ingredients_list(
                                    &ingredients,
                                    dialogue_lang_code.as_deref()
                                )
                            );

                            let keyboard = create_ingredient_review_keyboard(
                                &ingredients,
                                dialogue_lang_code.as_deref(),
                            );

                            // Edit the original message
                            match bot.edit_message_text(
                                msg.chat().id,
                                msg.id(),
                                review_message,
                            )
                            .reply_markup(keyboard)
                            .await {
                                Ok(_) => (),
                                Err(e) => error!(user_id = %q.from.id, error = %e, "Failed to edit message after ingredient deletion"),
                            }
                        }

                        // Update dialogue state with modified ingredients
                        match dialogue
                            .update(RecipeDialogueState::ReviewIngredients {
                                recipe_name: recipe_name.clone(),
                                ingredients: ingredients.clone(),
                                language_code: dialogue_lang_code.clone(),
                                message_id,
                                extracted_text: extracted_text.clone(),
                            })
                            .await {
                            Ok(_) => (),
                            Err(e) => error!(user_id = %q.from.id, error = %e, "Failed to update dialogue state after deletion"),
                        }
                    } else {
                        // Invalid index - ignore silently
                    }
                } else if data == "confirm" {
                    // Handle confirm button - proceed to recipe name input
                    let recipe_name_prompt = format!(
                        "🏷️ **{}**\n\n{}",
                        t_lang("recipe-name-prompt", dialogue_lang_code.as_deref()),
                        t_lang("recipe-name-prompt-hint", dialogue_lang_code.as_deref())
                    );

                    bot.send_message(msg.chat().id, recipe_name_prompt)
                        .await?;

                    // Transition to waiting for recipe name after confirmation
                    dialogue
                        .update(RecipeDialogueState::WaitingForRecipeNameAfterConfirm {
                            ingredients,
                            language_code: dialogue_lang_code,
                            extracted_text,
                        })
                        .await?;
                } else if data == "add_more" {
                    // Handle add more ingredients - reset to start state to allow new image
                    bot.send_message(
                        msg.chat().id,
                        t_lang(
                            "review-add-more-instructions",
                            dialogue_lang_code.as_deref(),
                        ),
                    )
                    .await?;

                    // Reset dialogue to start state
                    dialogue.update(RecipeDialogueState::Start).await?;
                } else if data == "cancel_review" {
                    // Handle cancel button - end dialogue without saving
                    bot.send_message(
                        msg.chat().id,
                        t_lang("review-cancelled", dialogue_lang_code.as_deref()),
                    )
                    .await?;

                    // End the dialogue
                    dialogue.exit().await?;
                }
            }
        }
        _ => {
            // Ignore callbacks for other states
        }
    }

    // Answer the callback query to remove the loading state
    bot.answer_callback_query(q.id).await?;

    Ok(())
}