use anyhow::Result;
use sqlx::postgres::PgPool;
use std::io::Write;
use std::sync::{Arc, LazyLock};
use teloxide::prelude::*;
use teloxide::types::{FileId, InlineKeyboardButton, InlineKeyboardMarkup};
use tempfile::NamedTempFile;
use tracing::{debug, error, info, warn};

// Import localization
use crate::localization::{t_args_lang, t_lang};

// Import text processing
use crate::text_processing::{MeasurementDetector, MeasurementMatch};

// Import OCR types
use crate::circuit_breaker::CircuitBreaker;
use crate::instance_manager::OcrInstanceManager;
use crate::ocr_config::OcrConfig;
use crate::ocr_errors::OcrError;

// Import dialogue types
use crate::dialogue::{validate_recipe_name, RecipeDialogue, RecipeDialogueState};

// Import database types
use crate::db::{create_ingredient, create_ocr_entry, get_or_create_user};

// Create OCR configuration with default settings
static OCR_CONFIG: LazyLock<OcrConfig> = LazyLock::new(OcrConfig::default);
static OCR_INSTANCE_MANAGER: LazyLock<OcrInstanceManager> =
    LazyLock::new(OcrInstanceManager::default);
static CIRCUIT_BREAKER: LazyLock<CircuitBreaker> =
    LazyLock::new(|| CircuitBreaker::new(OCR_CONFIG.recovery.clone()));

async fn download_file(bot: &Bot, file_id: FileId) -> Result<String> {
    let file = bot.get_file(file_id).await?;
    let file_path = file.path;
    let url = format!(
        "https://api.telegram.org/file/bot{}/{}",
        bot.token(),
        file_path
    );

    let response = reqwest::get(&url).await?;
    let bytes = response.bytes().await?;

    let mut temp_file = NamedTempFile::new()?;
    temp_file.as_file_mut().write_all(&bytes)?;
    let path = temp_file.path().to_string_lossy().to_string();

    // Instead of keeping the file, we return the path and let the caller handle cleanup
    // The NamedTempFile will be dropped here, but the file will remain until explicitly deleted
    std::mem::forget(temp_file);

    Ok(path)
}

async fn download_and_process_image(
    bot: &Bot,
    file_id: FileId,
    chat_id: ChatId,
    success_message: &str,
    language_code: Option<&str>,
    dialogue: RecipeDialogue,
    _pool: Arc<PgPool>, // Used later in dialogue flow for saving ingredients
) -> Result<String> {
    let temp_path = match download_file(bot, file_id).await {
        Ok(path) => {
            debug!(user_id = %chat_id, temp_path = %path, "Image downloaded successfully");
            path
        }
        Err(e) => {
            error!(user_id = %chat_id, error = %e, "Failed to download image for user");
            bot.send_message(chat_id, t_lang("error-download-failed", language_code))
                .await?;
            return Err(e);
        }
    }; // Ensure cleanup happens even if we return early
    let result = async {
        info!("Image downloaded to: {temp_path}");

        // Send initial success message
        bot.send_message(chat_id, success_message).await?;

        // Validate image format before OCR processing
        if !crate::ocr::is_supported_image_format(&temp_path, &OCR_CONFIG) {
            warn!(user_id = %chat_id, "Unsupported image format rejected");
            bot.send_message(chat_id, t_lang("error-unsupported-format", language_code))
                .await?;
            return Ok(String::new());
        }

        // Extract text from the image using OCR with circuit breaker protection
        match crate::ocr::extract_text_from_image(
            &temp_path,
            &OCR_CONFIG,
            &OCR_INSTANCE_MANAGER,
            &CIRCUIT_BREAKER,
        )
        .await
        {
            Ok(extracted_text) => {
                if extracted_text.is_empty() {
                    warn!(user_id = %chat_id, "OCR extraction returned empty text");
                    bot.send_message(chat_id, t_lang("error-no-text-found", language_code))
                        .await?;
                    Ok(String::new())
                } else {
                    info!(
                        user_id = %chat_id,
                        chars_extracted = extracted_text.len(),
                        "OCR extraction completed successfully"
                    );

                    // Process the extracted text to find ingredients with measurements
                    let ingredients =
                        process_ingredients_and_extract_matches(&extracted_text, language_code);

                    if ingredients.is_empty() {
                        // No ingredients found, send message directly without dialogue
                        let no_ingredients_msg = format!(
                            "📝 {}\n\n{}\n\n```\n{}\n```",
                            t_lang("no-ingredients-found", language_code),
                            t_lang("no-ingredients-suggestion", language_code),
                            extracted_text
                        );
                        bot.send_message(chat_id, &no_ingredients_msg).await?;
                    } else {
                        // Ingredients found, go directly to review interface
                        info!(user_id = %chat_id, ingredients_count = ingredients.len(), "Sending ingredients review interface");
                        let review_message = format!(
                            "📝 **{}**\n\n{}\n\n{}",
                            t_lang("review-title", language_code),
                            t_lang("review-description", language_code),
                            format_ingredients_list(&ingredients, language_code)
                        );

                        let keyboard = create_ingredient_review_keyboard(&ingredients, language_code);

                        let sent_message = bot.send_message(chat_id, review_message)
                            .reply_markup(keyboard)
                            .await?;

                        // Update dialogue state to review ingredients with default recipe name
                        dialogue
                            .update(RecipeDialogueState::ReviewIngredients {
                                recipe_name: "Recipe".to_string(), // Default recipe name
                                ingredients,
                                language_code: language_code.map(|s| s.to_string()),
                                message_id: Some(sent_message.id.0 as i32),
                            })
                            .await?;
                        
                        info!(user_id = %chat_id, "Ingredients review interface sent successfully");
                    }

                    Ok(extracted_text)
                }
            }
            Err(e) => {
                error!(
                    user_id = %chat_id,
                    error = %e,
                    "OCR processing failed for user"
                );

                // Provide more specific error messages based on the error type
                let error_message = match &e {
                    OcrError::Validation(msg) => {
                        t_args_lang("error-validation", &[("msg", msg)], language_code)
                    }
                    OcrError::ImageLoad(_) => t_lang("error-image-load", language_code),
                    OcrError::Initialization(_) => {
                        t_lang("error-ocr-initialization", language_code)
                    }
                    OcrError::Extraction(_) => t_lang("error-ocr-extraction", language_code),
                    OcrError::Timeout(msg) => {
                        t_args_lang("error-ocr-timeout", &[("msg", msg)], language_code)
                    }
                    OcrError::_InstanceCorruption(_) => {
                        t_lang("error-ocr-corruption", language_code)
                    }
                    OcrError::_ResourceExhaustion(_) => {
                        t_lang("error-ocr-exhaustion", language_code)
                    }
                };

                bot.send_message(chat_id, &error_message).await?;
                Err(anyhow::anyhow!("OCR processing failed: {:?}", e))
            }
        }
    }
    .await;

    // Always clean up the temporary file
    if let Err(cleanup_err) = std::fs::remove_file(&temp_path) {
        error!(temp_path = %temp_path, error = %cleanup_err, "Failed to clean up temporary file");
    } else {
        debug!(temp_path = %temp_path, "Temporary file cleaned up successfully");
    }

    result
}

/// Process extracted text and return measurement matches
fn process_ingredients_and_extract_matches(
    extracted_text: &str,
    _language_code: Option<&str>,
) -> Vec<MeasurementMatch> {
    debug!(
        text_length = extracted_text.len(),
        "Processing extracted text for ingredients"
    );

    // Create measurement detector with default configuration
    let detector = match MeasurementDetector::new() {
        Ok(detector) => detector,
        Err(e) => {
            error!(error = %e, "Failed to create measurement detector - ingredient extraction disabled");
            return Vec::new();
        }
    };

    // Find all measurements in the text
    let matches = detector.extract_ingredient_measurements(extracted_text);
    info!(
        matches_found = matches.len(),
        "Measurement detection completed"
    );

    matches
}


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


/// Handle recipe name input during dialogue
#[allow(clippy::too_many_arguments)]
async fn handle_recipe_name_input(
    bot: &Bot,
    msg: &Message,
    dialogue: RecipeDialogue,
    _pool: Arc<PgPool>,
    recipe_name_input: &str,
    _extracted_text: String,
    ingredients: Vec<MeasurementMatch>,
    language_code: Option<&str>,
) -> Result<()> {
    // Validate recipe name
    match validate_recipe_name(recipe_name_input) {
        Ok(validated_name) => {
            // Recipe name is valid, transition to ingredient review state
            let review_message = format!(
                "📝 **{}**\n\n{}\n\n{}",
                t_lang("review-title", language_code),
                t_lang("review-description", language_code),
                format_ingredients_list(&ingredients, language_code)
            );

            let keyboard = create_ingredient_review_keyboard(&ingredients, language_code);

            let sent_message = bot.send_message(msg.chat.id, review_message)
                .reply_markup(keyboard)
                .await?;

            // Update dialogue state to review ingredients
            dialogue
                .update(RecipeDialogueState::ReviewIngredients {
                    recipe_name: validated_name,
                    ingredients,
                    language_code: language_code.map(|s| s.to_string()),
                    message_id: Some(sent_message.id.0 as i32),
                })
                .await?;
        }
        Err("empty") => {
            bot.send_message(msg.chat.id, t_lang("recipe-name-invalid", language_code))
                .await?;
            // Keep dialogue active, user can try again
        }
        Err("too_long") => {
            bot.send_message(msg.chat.id, t_lang("recipe-name-too-long", language_code))
                .await?;
            // Keep dialogue active, user can try again
        }
        Err(_) => {
            bot.send_message(msg.chat.id, t_lang("recipe-name-invalid", language_code))
                .await?;
            // Keep dialogue active, user can try again
        }
    }

    Ok(())
}

/// Handle recipe name input after ingredient confirmation during dialogue
#[allow(clippy::too_many_arguments)]
async fn handle_recipe_name_after_confirm_input(
    bot: &Bot,
    msg: &Message,
    dialogue: RecipeDialogue,
    pool: Arc<PgPool>,
    recipe_name_input: &str,
    ingredients: Vec<MeasurementMatch>,
    language_code: Option<&str>,
) -> Result<()> {
    let input = recipe_name_input.trim().to_lowercase();

    // Check for cancellation commands
    if matches!(input.as_str(), "cancel" | "stop" | "back") {
        // User cancelled, end dialogue without saving
        bot.send_message(msg.chat.id, t_lang("review-cancelled", language_code))
            .await?;
        dialogue.exit().await?;
        return Ok(());
    }

    // Validate recipe name
    match validate_recipe_name(recipe_name_input) {
        Ok(validated_name) => {
            // Recipe name is valid, save ingredients to database
            if let Err(e) = save_ingredients_to_database(
                &pool,
                msg.chat.id.0,
                "", // extracted_text not needed for saving
                &ingredients,
                &validated_name,
                language_code,
            )
            .await
            {
                error!(error = %e, "Failed to save ingredients to database");
                bot.send_message(
                    msg.chat.id,
                    t_lang("error-processing-failed", language_code),
                )
                .await?;
            } else {
                // Success! Send confirmation message
                let success_message = t_args_lang(
                    "recipe-complete",
                    &[
                        ("recipe_name", &validated_name),
                        ("ingredient_count", &ingredients.len().to_string()),
                    ],
                    language_code,
                );
                bot.send_message(msg.chat.id, success_message).await?;
            }

            // End the dialogue
            dialogue.exit().await?;
        }
        Err("empty") => {
            bot.send_message(msg.chat.id, t_lang("recipe-name-invalid", language_code))
                .await?;
            // Keep dialogue active, user can try again
        }
        Err("too_long") => {
            bot.send_message(msg.chat.id, t_lang("recipe-name-too-long", language_code))
                .await?;
            // Keep dialogue active, user can try again
        }
        Err(_) => {
            bot.send_message(msg.chat.id, t_lang("recipe-name-invalid", language_code))
                .await?;
            // Keep dialogue active, user can try again
        }
    }

    Ok(())
}

/// Handle ingredient edit input during dialogue
#[allow(clippy::too_many_arguments)]
async fn handle_ingredient_edit_input(
    bot: &Bot,
    msg: &Message,
    dialogue: RecipeDialogue,
    edit_input: &str,
    recipe_name: String,
    mut ingredients: Vec<MeasurementMatch>,
    editing_index: usize,
    language_code: Option<&str>,
    message_id: Option<i32>,
) -> Result<()> {
    let input = edit_input.trim().to_lowercase();

    // Check for cancellation commands
    if matches!(input.as_str(), "cancel" | "stop" | "back") {
        // User cancelled editing, return to review state without changes
        let review_message = format!(
            "📝 **{}**\n\n{}\n\n{}",
            t_lang("review-title", language_code),
            t_lang("review-description", language_code),
            format_ingredients_list(&ingredients, language_code)
        );

        let keyboard = create_ingredient_review_keyboard(&ingredients, language_code);

        // If we have a message_id, edit the existing message; otherwise send a new one
        if let Some(msg_id) = message_id {
            bot.edit_message_text(msg.chat.id, teloxide::types::MessageId(msg_id), review_message)
                .reply_markup(keyboard)
                .await?;
        } else {
            bot.send_message(msg.chat.id, review_message)
                .reply_markup(keyboard)
                .await?;
        }

        // Update dialogue state to review ingredients
        dialogue
            .update(RecipeDialogueState::ReviewIngredients {
                recipe_name,
                ingredients,
                language_code: language_code.map(|s| s.to_string()),
                message_id,
            })
            .await?;

        return Ok(());
    }

    // Parse the user input to create a new ingredient
    match parse_ingredient_from_text(edit_input) {
        Ok(new_ingredient) => {
            // Update the ingredient at the editing index
            if editing_index < ingredients.len() {
                ingredients[editing_index] = new_ingredient;

                // Return to review state with updated ingredients
                let review_message = format!(
                    "📝 **{}**\n\n{}\n\n{}",
                    t_lang("review-title", language_code),
                    t_lang("review-description", language_code),
                    format_ingredients_list(&ingredients, language_code)
                );

                let keyboard = create_ingredient_review_keyboard(&ingredients, language_code);

                // If we have a message_id, edit the existing message; otherwise send a new one
                if let Some(msg_id) = message_id {
                    bot.edit_message_text(msg.chat.id, teloxide::types::MessageId(msg_id), review_message)
                        .reply_markup(keyboard)
                        .await?;
                } else {
                    bot.send_message(msg.chat.id, review_message)
                        .reply_markup(keyboard)
                        .await?;
                }

                // Update dialogue state to review ingredients
                dialogue
                    .update(RecipeDialogueState::ReviewIngredients {
                        recipe_name,
                        ingredients,
                        language_code: language_code.map(|s| s.to_string()),
                        message_id,
                    })
                    .await?;
            } else {
                // Invalid index, return to review state
                bot.send_message(msg.chat.id, t_lang("error-invalid-edit", language_code))
                    .await?;
                dialogue
                    .update(RecipeDialogueState::ReviewIngredients {
                        recipe_name,
                        ingredients,
                        language_code: language_code.map(|s| s.to_string()),
                        message_id,
                    })
                    .await?;
            }
        }
        Err(error_msg) => {
            // Invalid input, ask user to try again
            let error_message = format!(
                "{}\n\n{}",
                t_lang(error_msg, language_code),
                t_lang("edit-try-again", language_code)
            );
            bot.send_message(msg.chat.id, error_message).await?;
            // Stay in editing state for user to try again
        }
    }

    Ok(())
}

/// Parse ingredient text input and create a MeasurementMatch
pub fn parse_ingredient_from_text(input: &str) -> Result<MeasurementMatch, &'static str> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err("edit-empty");
    }

    // Check for maximum length to prevent abuse
    if trimmed.len() > 200 {
        return Err("edit-too-long");
    }

    // Try to extract measurement using the detector
    let detector = match MeasurementDetector::new() {
        Ok(detector) => detector,
        Err(_) => return Err("error-processing-failed"),
    };

    // Create a temporary text with the input to extract measurements
    let temp_text = format!("temp: {}", trimmed);
    let matches = detector.extract_ingredient_measurements(&temp_text);

    if let Some(mut measurement_match) = matches.into_iter().next() {
        // Found a measurement, validate the ingredient name
        let ingredient_name = measurement_match.ingredient_name.trim();

        // Check ingredient name length (before post-processing truncation)
        // Re-extract the raw ingredient name to check its length
        let temp_text = format!("temp: {}", trimmed);
        let measurement_end = measurement_match.end_pos;
        let raw_ingredient_name = temp_text[measurement_end..].trim();
        
        if raw_ingredient_name.is_empty() {
            return Err("edit-no-ingredient-name");
        }

        if raw_ingredient_name.len() > 100 {
            return Err("edit-ingredient-name-too-long");
        }

        if ingredient_name.is_empty() {
            return Err("edit-no-ingredient-name");
        }

        if ingredient_name.len() > 100 {
            return Err("edit-ingredient-name-too-long");
        }

        // Check for negative quantity by looking at the original text
        let temp_text = format!("temp: {}", trimmed);
        let quantity_start = measurement_match.start_pos;
        let mut actual_quantity = measurement_match.quantity.clone();
        
        // Check if there's a minus sign before the quantity
        if quantity_start > 0 && temp_text.as_bytes()[quantity_start - 1] == b'-' {
            // Check if the minus sign is not part of another word (should be preceded by space or start)
            let before_minus = if quantity_start > 1 { temp_text.as_bytes()[quantity_start - 2] } else { b' ' };
            if before_minus == b' ' || quantity_start == 1 {
                actual_quantity = format!("-{}", actual_quantity);
            }
        }
        
        measurement_match.quantity = actual_quantity;

        // Validate quantity is reasonable (not zero or negative)
        if let Some(qty) = parse_quantity(&measurement_match.quantity) {
            if qty <= 0.0 || qty > 10000.0 {
                return Err("edit-invalid-quantity");
            }
        }

        // Clean up the ingredient name
        measurement_match.ingredient_name = ingredient_name.to_string();
        Ok(measurement_match)
    } else {
        // No measurement found, try to extract a simple quantity pattern
        let quantity_pattern = regex::Regex::new(r"^(-?\d+(?:\.\d+)?(?:\s*\d+/\d+)?)").unwrap();
        if let Some(captures) = quantity_pattern.captures(trimmed) {
            if let Some(quantity_match) = captures.get(1) {
                let quantity = quantity_match.as_str().trim().to_string();
                let remaining = trimmed[quantity_match.end()..].trim().to_string();

                // Validate quantity
                if let Some(qty) = parse_quantity(&quantity) {
                    if qty <= 0.0 || qty > 10000.0 {
                        return Err("edit-invalid-quantity");
                    }
                }

                let ingredient_name = if remaining.is_empty() {
                    return Err("edit-no-ingredient-name");
                } else if remaining.len() > 100 {
                    return Err("edit-ingredient-name-too-long");
                } else {
                    remaining
                };

                Ok(MeasurementMatch {
                    quantity,
                    measurement: None,
                    ingredient_name,
                    line_number: 0,
                    start_pos: 0,
                    end_pos: trimmed.len(),
                })
            } else {
                Err("edit-invalid-format")
            }
        } else {
            // No quantity found, treat the whole input as ingredient name
            if trimmed.len() > 100 {
                return Err("edit-ingredient-name-too-long");
            }

            Ok(MeasurementMatch {
                quantity: "1".to_string(), // Default quantity
                measurement: None,
                ingredient_name: trimmed.to_string(),
                line_number: 0,
                start_pos: 0,
                end_pos: trimmed.len(),
            })
        }
    }
}

/// Parse quantity string to f64 (handles fractions and decimals)
fn parse_quantity(quantity_str: &str) -> Option<f64> {
    if quantity_str.contains('/') {
        // Handle fractions like "1/2"
        let parts: Vec<&str> = quantity_str.split('/').collect();
        if parts.len() == 2 {
            if let (Ok(numerator), Ok(denominator)) =
                (parts[0].parse::<f64>(), parts[1].parse::<f64>())
            {
                if denominator != 0.0 {
                    Some(numerator / denominator)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        // Handle regular numbers, replace comma with dot for European format
        quantity_str.replace(',', ".").parse::<f64>().ok()
    }
}

/// Handle ingredient review input during dialogue
#[allow(clippy::too_many_arguments)]
async fn handle_ingredient_review_input(
    bot: &Bot,
    msg: &Message,
    dialogue: RecipeDialogue,
    _pool: Arc<PgPool>,
    review_input: &str,
    recipe_name: String,
    ingredients: Vec<MeasurementMatch>,
    language_code: Option<&str>,
) -> Result<()> {
    let input = review_input.trim().to_lowercase();

    match input.as_str() {
        "confirm" | "ok" | "yes" | "save" => {
            // User confirmed, save ingredients to database
            if let Err(e) = save_ingredients_to_database(
                &_pool,
                msg.chat.id.0,
                "", // extracted_text not needed for saving
                &ingredients,
                &recipe_name,
                language_code,
            )
            .await
            {
                error!(error = %e, "Failed to save ingredients to database");
                bot.send_message(
                    msg.chat.id,
                    t_lang("error-processing-failed", language_code),
                )
                .await?;
            } else {
                // Success! Send confirmation message
                let success_message = t_args_lang(
                    "recipe-complete",
                    &[
                        ("recipe_name", &recipe_name),
                        ("ingredient_count", &ingredients.len().to_string()),
                    ],
                    language_code,
                );
                bot.send_message(msg.chat.id, success_message).await?;
            }

            // End the dialogue
            dialogue.exit().await?;
        }
        "cancel" | "stop" => {
            // User cancelled, end dialogue without saving
            bot.send_message(msg.chat.id, t_lang("review-cancelled", language_code))
                .await?;
            dialogue.exit().await?;
        }
        _ => {
            // Unknown command, show help
            let help_message = format!(
                "{}\n\n{}",
                t_lang("review-help", language_code),
                format_ingredients_list(&ingredients, language_code)
            );
            bot.send_message(msg.chat.id, help_message).await?;
            // Keep dialogue active
        }
    }

    Ok(())
}

/// Save ingredients to database
async fn save_ingredients_to_database(
    pool: &PgPool,
    telegram_id: i64,
    extracted_text: &str,
    ingredients: &[MeasurementMatch],
    recipe_name: &str,
    language_code: Option<&str>,
) -> Result<()> {
    // Get or create user
    let user = get_or_create_user(pool, telegram_id, language_code).await?;

    // Create OCR entry
    let ocr_entry_id = create_ocr_entry(pool, telegram_id, extracted_text).await?;

    // Save each ingredient
    for ingredient in ingredients {
        // Parse quantity from string (handle fractions)
        let quantity = parse_quantity(&ingredient.quantity);
        let unit = ingredient.measurement.as_deref();

        // Create raw text by combining quantity and measurement
        let raw_text = if let Some(ref unit) = ingredient.measurement {
            format!("{} {}", ingredient.quantity, unit)
        } else {
            ingredient.quantity.clone()
        };

        create_ingredient(
            pool,
            user.id,
            Some(ocr_entry_id),
            &ingredient.ingredient_name,
            quantity,
            unit,
            &raw_text,
            Some(recipe_name),
        )
        .await?;
    }

    Ok(())
}

async fn handle_text_message(
    bot: &Bot,
    msg: &Message,
    dialogue: RecipeDialogue,
    pool: Arc<PgPool>,
) -> Result<()> {
    if let Some(text) = msg.text() {
        debug!(user_id = %msg.chat.id, message_length = text.len(), "Received text message from user");

        // Extract user's language code from Telegram
        let language_code = msg
            .from
            .as_ref()
            .and_then(|user| user.language_code.as_ref())
            .map(|s| s.as_str());

        // Check dialogue state first
        let dialogue_state = dialogue.get().await?;
        match dialogue_state {
            Some(RecipeDialogueState::WaitingForRecipeName {
                extracted_text,
                ingredients,
                language_code: dialogue_lang_code,
            }) => {
                // Use dialogue language code if available, otherwise fall back to message language
                let effective_language_code = dialogue_lang_code.as_deref().or(language_code);

                // Handle recipe name input
                return handle_recipe_name_input(
                    bot,
                    msg,
                    dialogue,
                    pool,
                    text,
                    extracted_text,
                    ingredients,
                    effective_language_code,
                )
                .await;
            }
            Some(RecipeDialogueState::WaitingForRecipeNameAfterConfirm {
                ingredients,
                language_code: dialogue_lang_code,
            }) => {
                // Use dialogue language code if available, otherwise fall back to message language
                let effective_language_code = dialogue_lang_code.as_deref().or(language_code);

                // Handle recipe name input after ingredient confirmation
                return handle_recipe_name_after_confirm_input(
                    bot,
                    msg,
                    dialogue,
                    pool,
                    text,
                    ingredients,
                    effective_language_code,
                )
                .await;
            }
            Some(RecipeDialogueState::ReviewIngredients {
                recipe_name,
                ingredients,
                language_code: dialogue_lang_code,
                message_id: _,
            }) => {
                // Use dialogue language code if available, otherwise fall back to message language
                let effective_language_code = dialogue_lang_code.as_deref().or(language_code);

                // Handle ingredient review commands
                return handle_ingredient_review_input(
                    bot,
                    msg,
                    dialogue,
                    pool,
                    text,
                    recipe_name,
                    ingredients,
                    effective_language_code,
                )
                .await;
            }
            Some(RecipeDialogueState::EditingIngredient {
                recipe_name,
                ingredients,
                editing_index,
                language_code: dialogue_lang_code,
                message_id,
            }) => {
                // Use dialogue language code if available, otherwise fall back to message language
                let effective_language_code = dialogue_lang_code.as_deref().or(language_code);

                // Handle ingredient edit input
                return handle_ingredient_edit_input(
                    bot,
                    msg,
                    dialogue,
                    text,
                    recipe_name,
                    ingredients,
                    editing_index,
                    effective_language_code,
                    message_id,
                )
                .await;
            }
            Some(RecipeDialogueState::Start) | None => {
                // Continue with normal command handling
            }
        }

        // Handle /start command
        if text == "/start" {
            let welcome_message = format!(
                "👋 **{}**\n\n{}\n\n{}\n\n{}\n{}\n{}\n\n{}",
                t_lang("welcome-title", language_code),
                t_lang("welcome-description", language_code),
                t_lang("welcome-features", language_code),
                t_lang("welcome-commands", language_code),
                t_lang("welcome-start", language_code),
                t_lang("welcome-help", language_code),
                t_lang("welcome-send-image", language_code)
            );
            bot.send_message(msg.chat.id, welcome_message).await?;
        }
        // Handle /help command
        else if text == "/help" {
            let help_message = vec![
                t_lang("help-title", language_code),
                t_lang("help-description", language_code),
                t_lang("help-step1", language_code),
                t_lang("help-step2", language_code),
                t_lang("help-step3", language_code),
                t_lang("help-step4", language_code),
                t_lang("help-formats", language_code),
                t_lang("help-commands", language_code),
                t_lang("help-start", language_code),
                t_lang("help-tips", language_code),
                t_lang("help-tip1", language_code),
                t_lang("help-tip2", language_code),
                t_lang("help-tip3", language_code),
                t_lang("help-tip4", language_code),
                t_lang("help-final", language_code),
            ]
            .join("\n\n");
            bot.send_message(msg.chat.id, help_message).await?;
        }
        // Handle regular text messages
        else {
            bot.send_message(
                msg.chat.id,
                format!(
                    "{} {}",
                    t_args_lang("text-response", &[("text", text)], language_code),
                    t_lang("text-tip", language_code)
                ),
            )
            .await?;
        }
    }
    Ok(())
}

async fn handle_photo_message(
    bot: &Bot,
    msg: &Message,
    dialogue: RecipeDialogue,
    pool: Arc<PgPool>,
) -> Result<()> {
    // Extract user's language code from Telegram
    let language_code = msg
        .from
        .as_ref()
        .and_then(|user| user.language_code.as_ref())
        .map(|s| s.as_str());

    debug!(user_id = %msg.chat.id, "Received photo message from user");

    if let Some(photos) = msg.photo() {
        if let Some(largest_photo) = photos.last() {
            let _temp_path = download_and_process_image(
                bot,
                largest_photo.file.id.clone(),
                msg.chat.id,
                &t_lang("processing-photo", language_code),
                language_code,
                dialogue,
                pool,
            )
            .await;
        }
    }
    Ok(())
}

async fn handle_document_message(
    bot: &Bot,
    msg: &Message,
    dialogue: RecipeDialogue,
    pool: Arc<PgPool>,
) -> Result<()> {
    // Extract user's language code from Telegram
    let language_code = msg
        .from
        .as_ref()
        .and_then(|user| user.language_code.as_ref())
        .map(|s| s.as_str());

    if let Some(doc) = msg.document() {
        if let Some(mime_type) = &doc.mime_type {
            if mime_type.to_string().starts_with("image/") {
                debug!(user_id = %msg.chat.id, mime_type = %mime_type, "Received image document from user");
                let _temp_path = download_and_process_image(
                    bot,
                    doc.file.id.clone(),
                    msg.chat.id,
                    &t_lang("processing-document", language_code),
                    language_code,
                    dialogue,
                    pool,
                )
                .await;
            } else {
                debug!(user_id = %msg.chat.id, mime_type = %mime_type, "Received non-image document from user");
                bot.send_message(
                    msg.chat.id,
                    t_lang("error-unsupported-format", language_code),
                )
                .await?;
            }
        } else {
            debug!(user_id = %msg.chat.id, "Received document without mime type from user");
            bot.send_message(msg.chat.id, t_lang("error-no-mime-type", language_code))
                .await?;
        }
    }
    Ok(())
}

async fn handle_unsupported_message(bot: &Bot, msg: &Message) -> Result<()> {
    // Extract user's language code from Telegram
    let language_code = msg
        .from
        .as_ref()
        .and_then(|user| user.language_code.as_ref())
        .map(|s| s.as_str());

    debug!(user_id = %msg.chat.id, "Received unsupported message type from user");

    let help_message = format!(
        "{}\n\n{}\n{}\n{}\n{}\n{}\n\n{}",
        t_lang("unsupported-title", language_code),
        t_lang("unsupported-description", language_code),
        t_lang("unsupported-feature1", language_code),
        t_lang("unsupported-feature2", language_code),
        t_lang("unsupported-feature3", language_code),
        t_lang("unsupported-feature4", language_code),
        t_lang("unsupported-final", language_code)
    );
    bot.send_message(msg.chat.id, help_message).await?;
    Ok(())
}

pub async fn message_handler(
    bot: Bot,
    msg: Message,
    pool: Arc<PgPool>,
    dialogue: RecipeDialogue,
) -> Result<()> {
    if msg.text().is_some() {
        handle_text_message(&bot, &msg, dialogue, pool).await?;
    } else if msg.photo().is_some() {
        handle_photo_message(&bot, &msg, dialogue, pool).await?;
    } else if msg.document().is_some() {
        handle_document_message(&bot, &msg, dialogue, pool).await?;
    } else {
        handle_unsupported_message(&bot, &msg).await?;
    }

    Ok(())
}

/// Handle callback queries from inline keyboards
pub async fn callback_handler(
    bot: Bot,
    q: CallbackQuery,
    _pool: Arc<PgPool>,
    dialogue: RecipeDialogue,
) -> Result<()> {
    debug!(user_id = %q.from.id, "Received callback query from user");

    // Check dialogue state
    let dialogue_state = dialogue.get().await?;
    match dialogue_state {
        Some(RecipeDialogueState::ReviewIngredients {
            recipe_name,
            mut ingredients,
            language_code: dialogue_lang_code,
            message_id,
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
                        bot.send_message(ChatId::from(q.from.id), edit_prompt)
                            .await?;

                        // Transition to editing state
                        dialogue
                            .update(RecipeDialogueState::EditingIngredient {
                                recipe_name: recipe_name.clone(),
                                ingredients: ingredients.clone(),
                                editing_index: index,
                                language_code: dialogue_lang_code.clone(),
                                message_id,
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

                            let keyboard = InlineKeyboardMarkup::new(vec![
                                vec![
                                    InlineKeyboardButton::callback(
                                        t_lang("review-add-more", dialogue_lang_code.as_deref()),
                                        "add_more"
                                    ),
                                    InlineKeyboardButton::callback(
                                        t_lang("cancel", dialogue_lang_code.as_deref()),
                                        "cancel_empty"
                                    ),
                                ]
                            ]);

                            // Edit the original message
                            bot.edit_message_text(ChatId::from(q.from.id), msg.id(), empty_message)
                                .reply_markup(keyboard)
                                .await?;
                        } else {
                            // Update the message with remaining ingredients
                            let review_message = format!(
                                "📝 **{}**\n\n{}\n\n{}",
                                t_lang("review-title", dialogue_lang_code.as_deref()),
                                t_lang("review-description", dialogue_lang_code.as_deref()),
                                format_ingredients_list(&ingredients, dialogue_lang_code.as_deref())
                            );

                            let keyboard = create_ingredient_review_keyboard(
                                &ingredients,
                                dialogue_lang_code.as_deref(),
                            );

                            // Edit the original message
                            bot.edit_message_text(ChatId::from(q.from.id), msg.id(), review_message)
                                .reply_markup(keyboard)
                                .await?;
                        }

                        // Update dialogue state with modified ingredients
                        dialogue
                            .update(RecipeDialogueState::ReviewIngredients {
                                recipe_name: recipe_name.clone(),
                                ingredients: ingredients.clone(),
                                language_code: dialogue_lang_code.clone(),
                                message_id,
                            })
                            .await?;
                    }
                } else if data == "confirm" {
                    // Handle confirm button - proceed to recipe name input
                    let recipe_name_prompt = format!(
                        "🏷️ **{}**\n\n{}",
                        t_lang("recipe-name-prompt", dialogue_lang_code.as_deref()),
                        t_lang("recipe-name-prompt-hint", dialogue_lang_code.as_deref())
                    );

                    bot.send_message(ChatId::from(q.from.id), recipe_name_prompt)
                        .await?;

                    // Transition to waiting for recipe name after confirmation
                    dialogue
                        .update(RecipeDialogueState::WaitingForRecipeNameAfterConfirm {
                            ingredients,
                            language_code: dialogue_lang_code,
                        })
                        .await?;
                } else if data == "add_more" {
                    // Handle add more ingredients - reset to start state to allow new image
                    bot.send_message(
                        ChatId::from(q.from.id),
                        t_lang("review-add-more-instructions", dialogue_lang_code.as_deref()),
                    )
                    .await?;

                    // Reset dialogue to start state
                    dialogue.update(RecipeDialogueState::Start).await?;
                } else if data == "cancel_review" {
                    // Handle cancel button - end dialogue without saving
                    bot.send_message(
                        ChatId::from(q.from.id),
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
