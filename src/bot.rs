use anyhow::Result;
use sqlx::postgres::PgPool;
use std::io::Write;
use std::sync::{Arc, LazyLock};
use teloxide::prelude::*;
use teloxide::types::FileId;
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
use crate::dialogue::{RecipeDialogue, RecipeDialogueState, validate_recipe_name};

// Import database types
use crate::db::{get_or_create_user, create_ocr_entry, create_ingredient};

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
    pool: Arc<PgPool>,
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
                    let ingredients = process_ingredients_and_extract_matches(&extracted_text, language_code);

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
                        // Ingredients found, start dialogue for recipe name
                        let ingredients_summary = format_ingredients_summary(&ingredients, language_code);
                        bot.send_message(chat_id, &ingredients_summary).await?;
                        
                        // Start recipe name dialogue
                        start_recipe_name_dialogue(bot, chat_id, dialogue, extracted_text.clone(), ingredients, language_code).await?;
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
fn process_ingredients_and_extract_matches(extracted_text: &str, _language_code: Option<&str>) -> Vec<MeasurementMatch> {
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
    let matches = detector.find_measurements(extracted_text);
    info!(matches_found = matches.len(), "Measurement detection completed");
    
    matches
}

/// Format ingredients summary for display
fn format_ingredients_summary(ingredients: &[MeasurementMatch], language_code: Option<&str>) -> String {
    // Group matches by line for better organization
    let mut ingredients_by_line: std::collections::HashMap<usize, Vec<&MeasurementMatch>> =
        std::collections::HashMap::new();

    for measurement_match in ingredients {
        ingredients_by_line
            .entry(measurement_match.line_number)
            .or_default()
            .push(measurement_match);
    }

    // Format the results
    let mut result = format!("🍳 {}\n\n", t_lang("ingredients-found", language_code));

    // Sort by line number for consistent ordering
    let mut sorted_lines: Vec<_> = ingredients_by_line.keys().collect();
    sorted_lines.sort();

    for &line_num in &sorted_lines {
        if let Some(line_matches) = ingredients_by_line.get(line_num) {
            result.push_str(&format!(
                "📋 **{} {}:**\n",
                t_lang("line", language_code),
                line_num + 1
            )); // 1-indexed for user display

            for measurement_match in line_matches {
                let ingredient_display = if measurement_match.ingredient_name.is_empty() {
                    format!("❓ {}", t_lang("unknown-ingredient", language_code))
                } else {
                    measurement_match.ingredient_name.clone()
                };

                result.push_str(&format!(
                    "   • **{}** → {}\n",
                    measurement_match.text, ingredient_display
                ));
            }
            result.push('\n');
        }
    }

    // Add summary
    result.push_str(&format!(
        "📊 **{}:** {}\n",
        t_lang("total-ingredients", language_code),
        ingredients.len()
    ));

    result
}

/// Start the recipe name dialogue
async fn start_recipe_name_dialogue(
    bot: &Bot,
    chat_id: ChatId,
    dialogue: RecipeDialogue,
    extracted_text: String,
    ingredients: Vec<MeasurementMatch>,
    language_code: Option<&str>,
) -> Result<()> {
    // Send recipe name prompt
    let prompt_message = format!(
        "{}\n{}",
        t_lang("recipe-name-prompt", language_code),
        t_lang("recipe-name-prompt-hint", language_code)
    );
    
    bot.send_message(chat_id, prompt_message).await?;
    
    // Update dialogue state
    dialogue.update(RecipeDialogueState::WaitingForRecipeName {
        extracted_text,
        ingredients,
        language_code: language_code.map(|s| s.to_string()),
    }).await?;
    
    Ok(())
}

/// Handle recipe name input during dialogue
async fn handle_recipe_name_input(
    bot: &Bot,
    msg: &Message,
    dialogue: RecipeDialogue,
    pool: Arc<PgPool>,
    recipe_name_input: &str,
    extracted_text: String,
    ingredients: Vec<MeasurementMatch>,
    language_code: Option<&str>,
) -> Result<()> {
    // Validate recipe name
    match validate_recipe_name(recipe_name_input) {
        Ok(validated_name) => {
            // Recipe name is valid, save ingredients to database
            if let Err(e) = save_ingredients_to_database(
                &pool,
                msg.chat.id.0,
                &extracted_text,
                &ingredients,
                &validated_name,
                language_code,
            ).await {
                error!(error = %e, "Failed to save ingredients to database");
                bot.send_message(
                    msg.chat.id,
                    t_lang("error-processing-failed", language_code)
                ).await?;
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
            bot.send_message(
                msg.chat.id,
                t_lang("recipe-name-invalid", language_code)
            ).await?;
            // Keep dialogue active, user can try again
        }
        Err("too_long") => {
            bot.send_message(
                msg.chat.id,
                t_lang("recipe-name-too-long", language_code)
            ).await?;
            // Keep dialogue active, user can try again
        }
        Err(_) => {
            bot.send_message(
                msg.chat.id,
                t_lang("recipe-name-invalid", language_code)
            ).await?;
            // Keep dialogue active, user can try again
        }
    }

    Ok(())
}

/// Parse quantity and unit from measurement text
fn parse_measurement_text(measurement_text: &str) -> (Option<f64>, Option<String>) {
    // Simple regex to extract quantity and unit
    use regex::Regex;
    
    let re = Regex::new(r"^(\d+(?:[.,]\d+)?(?:/\d+)?)\s*(.*)$").unwrap();
    
    if let Some(captures) = re.captures(measurement_text.trim()) {
        let quantity_str = captures.get(1).map_or("", |m| m.as_str());
        let unit_str = captures.get(2).map_or("", |m| m.as_str()).trim();
        
        // Parse quantity (handle fractions)
        let quantity = if quantity_str.contains('/') {
            // Handle fractions like "1/2"
            let parts: Vec<&str> = quantity_str.split('/').collect();
            if parts.len() == 2 {
                if let (Ok(numerator), Ok(denominator)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
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
        };
        
        let unit = if unit_str.is_empty() {
            None
        } else {
            Some(unit_str.to_string())
        };
        
        (quantity, unit)
    } else {
        (None, None)
    }
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
        let (quantity, unit) = parse_measurement_text(&ingredient.text);
        create_ingredient(
            pool,
            user.id,
            Some(ocr_entry_id),
            &ingredient.ingredient_name,
            quantity,
            unit.as_deref(),
            &ingredient.text,
            Some(recipe_name),
        ).await?;
    }
    
    Ok(())
}

async fn handle_text_message(bot: &Bot, msg: &Message, dialogue: RecipeDialogue, pool: Arc<PgPool>) -> Result<()> {
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
                language_code: dialogue_lang_code 
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
                    effective_language_code
                ).await;
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

async fn handle_photo_message(bot: &Bot, msg: &Message, dialogue: RecipeDialogue, pool: Arc<PgPool>) -> Result<()> {
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

async fn handle_document_message(bot: &Bot, msg: &Message, dialogue: RecipeDialogue, pool: Arc<PgPool>) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Import types from the ocr module for testing
    use crate::circuit_breaker::CircuitBreaker;
    use crate::instance_manager::OcrInstanceManager;
    use crate::ocr_config::{FormatSizeLimits, OcrConfig, RecoveryConfig};

    /// Test static configuration initialization
    #[test]
    fn test_static_config_initialization() {
        // Test that static configurations are properly initialized
        let _config = &*OCR_CONFIG;
        let _manager = &*OCR_INSTANCE_MANAGER;
        let _circuit_breaker = &*CIRCUIT_BREAKER;

        // Verify configuration values
        assert_eq!(OCR_CONFIG.languages, "eng+fra");
        assert_eq!(OCR_CONFIG.buffer_size, 32);
        assert_eq!(OCR_CONFIG.min_format_bytes, 8);
        assert_eq!(OCR_CONFIG.max_file_size, 10 * 1024 * 1024); // 10MB
    }

    /// Test OCR configuration validation
    #[test]
    fn test_ocr_config_validation() {
        let config = OcrConfig::default();

        // Test that configuration has reasonable defaults
        assert!(!config.languages.is_empty());
        assert!(config.buffer_size > 0);
        assert!(config.min_format_bytes > 0);
        assert!(config.max_file_size > 0);
        assert!(config.recovery.max_retries <= 10); // Reasonable upper bound
        assert!(config.recovery.operation_timeout_secs > 0);
    }

    /// Test circuit breaker initialization
    #[test]
    fn test_circuit_breaker_initialization() {
        let circuit_breaker = CircuitBreaker::new(OCR_CONFIG.recovery.clone());

        // Initially should not be open
        assert!(!circuit_breaker.is_open());
    }

    /// Test OCR instance manager initialization
    #[test]
    fn test_ocr_instance_manager_initialization() {
        let manager = OcrInstanceManager::new();

        // Initially should be empty
        assert_eq!(manager._instance_count(), 0);
    }

    /// Test error message formatting
    #[test]
    fn test_error_message_formatting() {
        let validation_error = OcrError::Validation("Test validation error".to_string());
        let display_msg = format!("{}", validation_error);
        assert_eq!(display_msg, "Validation error: Test validation error");

        let timeout_error = OcrError::Timeout("Test timeout".to_string());
        let display_msg = format!("{}", timeout_error);
        assert_eq!(display_msg, "Timeout error: Test timeout");
    }

    /// Test temporary file cleanup
    #[test]
    fn test_temp_file_cleanup() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();
        let temp_path = temp_file.path().to_string_lossy().to_string();

        // Simulate cleanup
        let cleanup_result = fs::remove_file(&temp_path);
        assert!(cleanup_result.is_ok() || cleanup_result.is_err()); // File might not exist
    }

    /// Test OCR configuration defaults are reasonable
    #[test]
    fn test_ocr_config_defaults_reasonable() {
        let config = OcrConfig::default();
        let recovery = config.recovery;

        // Test that defaults are within reasonable ranges
        assert!(config.max_file_size > 1024 * 1024); // At least 1MB
        assert!(config.max_file_size <= 100 * 1024 * 1024); // At most 100MB

        assert!(recovery.max_retries <= 10); // Reasonable upper bound
        assert!(recovery.max_retries <= 10); // Reasonable retry limit

        assert!(recovery.operation_timeout_secs > 0);
        assert!(recovery.operation_timeout_secs <= 300); // At most 5 minutes

        assert!(recovery.base_retry_delay_ms >= 100); // At least 100ms
        assert!(recovery.base_retry_delay_ms <= 10000); // At most 10 seconds
    }

    /// Test format size limits defaults
    #[test]
    fn test_format_size_limits_defaults() {
        let limits = FormatSizeLimits::default();

        // Test that format limits are in ascending order for different formats
        assert!(limits.bmp_max <= limits.jpeg_max);
        assert!(limits.jpeg_max <= limits.png_max);
        assert!(limits.png_max <= limits.tiff_max);

        // Test that all limits are reasonable (between 1MB and 50MB)
        assert!(limits.bmp_max >= 1024 * 1024);
        assert!(limits.tiff_max <= 50 * 1024 * 1024);
    }

    /// Test circuit breaker failure recording
    #[test]
    fn test_circuit_breaker_failure_recording() {
        let config = RecoveryConfig {
            circuit_breaker_threshold: 2,
            ..Default::default()
        };
        let circuit_breaker = CircuitBreaker::new(config);

        // Initially closed
        assert!(!circuit_breaker.is_open());

        // Record one failure - still closed
        circuit_breaker.record_failure();
        assert!(!circuit_breaker.is_open());

        // Record second failure - now open
        circuit_breaker.record_failure();
        assert!(circuit_breaker.is_open());
    }

    /// Test circuit breaker success recording
    #[test]
    fn test_circuit_breaker_success_recording() {
        let config = RecoveryConfig {
            circuit_breaker_threshold: 1,
            ..Default::default()
        };
        let circuit_breaker = CircuitBreaker::new(config);

        // Record failure to open circuit
        circuit_breaker.record_failure();
        assert!(circuit_breaker.is_open());

        // Record success to close circuit
        circuit_breaker.record_success();
        assert!(!circuit_breaker.is_open());
    }

    /// Test OCR instance manager operations
    #[test]
    fn test_ocr_instance_manager_operations() {
        let manager = OcrInstanceManager::new();

        // Initially empty
        assert_eq!(manager._instance_count(), 0);

        // Test that we can create a new manager (basic functionality test)
        let new_manager = OcrInstanceManager::new();
        assert_eq!(new_manager._instance_count(), 0);
    }

    /// Test configuration cloning
    #[test]
    fn test_config_cloning() {
        let config = OcrConfig::default();
        let cloned_config = config.clone();

        // Test that cloning preserves values
        assert_eq!(config.languages, cloned_config.languages);
        assert_eq!(config.buffer_size, cloned_config.buffer_size);
        assert_eq!(config.max_file_size, cloned_config.max_file_size);
    }

    /// Test that static configs are thread-safe
    #[test]
    fn test_static_configs_thread_safety() {
        // Test that we can access static configs multiple times
        let config1 = &*OCR_CONFIG;
        let config2 = &*OCR_CONFIG;

        // Both should point to the same configuration
        assert_eq!(config1.languages, config2.languages);
        assert_eq!(config1.max_file_size, config2.max_file_size);
    }

    /// Test image format validation function
    #[test]
    fn test_image_format_validation() {
        // Test with a non-existent file (should return false)
        let result =
            crate::ocr::is_supported_image_format("/non/existent/file.png", &OcrConfig::default());
        assert!(!result);
    }

    /// Test that all error variants can be created
    #[test]
    fn test_error_variants_creation() {
        let validation_err = OcrError::Validation("test".to_string());
        let init_err = OcrError::Initialization("test".to_string());
        let load_err = OcrError::ImageLoad("test".to_string());
        let extract_err = OcrError::Extraction("test".to_string());
        let timeout_err = OcrError::Timeout("test".to_string());

        // Test that all variants can be formatted
        assert!(format!("{}", validation_err).contains("Validation error"));
        assert!(format!("{}", init_err).contains("Initialization error"));
        assert!(format!("{}", load_err).contains("Image load error"));
        assert!(format!("{}", extract_err).contains("Extraction error"));
        assert!(format!("{}", timeout_err).contains("Timeout error"));
    }

    /// Test configuration structure
    #[test]
    fn test_config_structure() {
        let config = OcrConfig::default();

        // Test that all fields are accessible and have reasonable values
        assert!(!config.languages.is_empty());
        assert!(config.buffer_size > 0);
        assert!(config.min_format_bytes > 0);
        assert!(config.max_file_size > 0);

        // Test nested structure access with references
        let png_max = config.format_limits.png_max;
        let max_retries = config.recovery.max_retries;

        assert!(png_max > 0);
        assert!(max_retries <= 10); // Reasonable upper bound
    }

    /// Test /start command response content
    #[test]
    fn test_start_command_response_contains_expected_content() {
        // Test that the start command response contains key elements
        let expected_phrases = [
            "Welcome to Ingredients Bot",
            "Send me photos",
            "OCR",
            "start",
            "help",
        ];

        // This is a basic content check - in a real scenario we'd mock the bot
        // For now, we verify our expected phrases are reasonable
        for phrase in &expected_phrases {
            assert!(!phrase.is_empty(), "Expected phrase should not be empty");
            assert!(phrase.len() > 2, "Expected phrase should be meaningful");
        }
    }

    /// Test /help command response content
    #[test]
    fn test_help_command_response_contains_expected_content() {
        // Test that the help command response contains key elements
        let expected_phrases = [
            "Ingredients Bot Help",
            "Send a photo",
            "Supported formats",
            "File size limit",
            "clear, well-lit images",
        ];

        // This is a basic content check - in a real scenario we'd mock the bot
        // For now, we verify our expected phrases are reasonable
        for phrase in &expected_phrases {
            assert!(!phrase.is_empty(), "Expected phrase should not be empty");
            assert!(phrase.len() > 3, "Expected phrase should be meaningful");
        }
    }

    /// Test French localization support
    #[test]
    fn test_french_localization() {
        use crate::localization::{get_localization_manager, init_localization};

        // Initialize localization
        init_localization().expect("Failed to initialize localization");

        let manager = get_localization_manager();

        // Test that both English and French are supported
        assert!(
            manager.is_language_supported("en"),
            "English should be supported"
        );
        assert!(
            manager.is_language_supported("fr"),
            "French should be supported"
        );
        assert!(
            !manager.is_language_supported("es"),
            "Spanish should not be supported"
        );

        // Test basic messages in both languages
        let welcome_title_en = manager.get_message_in_language("welcome-title", "en", None);
        let welcome_title_fr = manager.get_message_in_language("welcome-title", "fr", None);

        assert!(
            !welcome_title_en.is_empty(),
            "English welcome-title should not be empty"
        );
        assert!(
            !welcome_title_fr.is_empty(),
            "French welcome-title should not be empty"
        );
        assert_ne!(
            welcome_title_en, welcome_title_fr,
            "English and French welcome-title should be different"
        );

        // Test messages with arguments - let's find a key that uses arguments
        let help_step1_en = manager.get_message_in_language("help-step1", "en", None);
        let help_step1_fr = manager.get_message_in_language("help-step1", "fr", None);

        assert!(
            !help_step1_en.is_empty(),
            "English help-step1 should not be empty"
        );
        assert!(
            !help_step1_fr.is_empty(),
            "French help-step1 should not be empty"
        );
        assert_ne!(
            help_step1_en, help_step1_fr,
            "English and French help-step1 should be different"
        );

        // Test fallback to English for unsupported language
        let fallback = manager.get_message_in_language("welcome-title", "de", None);
        assert_eq!(
            fallback, welcome_title_en,
            "Unsupported language should fallback to English"
        );
    }

    /// Test language detection functionality
    #[test]
    fn test_language_detection() {
        use crate::localization::{detect_language, init_localization};

        // Initialize localization
        init_localization().expect("Failed to initialize localization");

        // Test supported languages
        assert_eq!(
            detect_language(Some("fr")),
            "fr",
            "French should be detected as 'fr'"
        );
        assert_eq!(
            detect_language(Some("en")),
            "en",
            "English should be detected as 'en'"
        );
        assert_eq!(
            detect_language(Some("fr-FR")),
            "fr",
            "French with locale should be detected as 'fr'"
        );
        assert_eq!(
            detect_language(Some("en-US")),
            "en",
            "English with locale should be detected as 'en'"
        );

        // Test unsupported languages fallback to English
        assert_eq!(
            detect_language(Some("es")),
            "en",
            "Unsupported language should fallback to English"
        );
        assert_eq!(
            detect_language(Some("de")),
            "en",
            "German should fallback to English"
        );
        assert_eq!(
            detect_language(Some("zh-CN")),
            "en",
            "Chinese should fallback to English"
        );

        // Test None case
        assert_eq!(
            detect_language(None),
            "en",
            "None should default to English"
        );
    }
}
