//! Message Handler module for processing incoming Telegram messages

use anyhow::Result;
use sqlx::postgres::PgPool;
use std::io::Write;
use std::sync::Arc;
use teloxide::prelude::*;
use tempfile::NamedTempFile;
use tracing::{debug, error, info, warn};

// Import localization
use crate::localization::t_lang;

// Import text processing
use crate::text_processing::{MeasurementDetector, MeasurementMatch};

// Import OCR types
use crate::circuit_breaker::CircuitBreaker;
use crate::instance_manager::OcrInstanceManager;
use crate::ocr_config::OcrConfig;
use crate::ocr_errors::OcrError;

// Import dialogue types
use crate::dialogue::{RecipeDialogue, RecipeDialogueState};

// Import dialogue manager functions
use super::dialogue_manager::{
    handle_ingredient_edit_input, handle_ingredient_review_input, handle_recipe_name_after_confirm_input,
    handle_recipe_name_input,
};

// Import UI builder functions
use super::ui_builder::{format_ingredients_list, create_ingredient_review_keyboard};

// Create OCR configuration with default settings
static OCR_CONFIG: std::sync::LazyLock<OcrConfig> = std::sync::LazyLock::new(OcrConfig::default);
static OCR_INSTANCE_MANAGER: std::sync::LazyLock<OcrInstanceManager> =
    std::sync::LazyLock::new(OcrInstanceManager::default);
static CIRCUIT_BREAKER: std::sync::LazyLock<CircuitBreaker> =
    std::sync::LazyLock::new(|| CircuitBreaker::new(OCR_CONFIG.recovery.clone()));

pub async fn download_file(bot: &Bot, file_id: teloxide::types::FileId) -> Result<String> {
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

pub async fn download_and_process_image(
    bot: &Bot,
    file_id: teloxide::types::FileId,
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
                                extracted_text: extracted_text.clone(),
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
                        t_lang("error-validation", language_code).replace("{}", msg)
                    }
                    OcrError::ImageLoad(_) => t_lang("error-image-load", language_code),
                    OcrError::Initialization(_) => {
                        t_lang("error-ocr-initialization", language_code)
                    }
                    OcrError::Extraction(_) => t_lang("error-ocr-extraction", language_code),
                    OcrError::Timeout(msg) => {
                        t_lang("error-ocr-timeout", language_code).replace("{}", msg)
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
pub fn process_ingredients_and_extract_matches(
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
                extracted_text,
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
                    extracted_text,
                )
                .await;
            }
            Some(RecipeDialogueState::ReviewIngredients {
                recipe_name,
                ingredients,
                language_code: dialogue_lang_code,
                message_id: _,
                extracted_text,
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
                    extracted_text,
                )
                .await;
            }
            Some(RecipeDialogueState::EditingIngredient {
                recipe_name,
                ingredients,
                editing_index,
                language_code: dialogue_lang_code,
                message_id,
                extracted_text,
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
                    extracted_text,
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
                    t_lang("text-response", language_code),
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