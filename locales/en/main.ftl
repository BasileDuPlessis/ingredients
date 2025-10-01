# Ingredients Bot - English Localization
# Main welcome and help messages

welcome-title = Welcome to Ingredients Bot!
welcome-description = I'm your OCR assistant that can extract text from images. Here's what I can do:
welcome-features =
    📸 **Send me photos** of ingredient lists, recipes, or any text you want to extract
    📄 **Send me image files** (PNG, JPG, JPEG, BMP, TIFF, TIF)
    🔍 **I'll process them with OCR** and send back the extracted text
    💾 **All extracted text is stored** for future reference
welcome-commands = Commands:
welcome-start = /start - Show this welcome message
welcome-help = /help - Get help and usage instructions
welcome-send-image = Just send me an image and I'll do the rest! 🚀

help-title = 🆘 Ingredients Bot Help
help-description = How to use me:
help-step1 = 1. 📸 Send a photo of text you want to extract
help-step2 = 2. 📎 Or send an image file (PNG, JPG, JPEG, BMP, TIFF, TIF)
help-step3 = 3. ⏳ I'll process it with OCR technology
help-step4 = 4. 📝 You'll receive the extracted text
help-formats = Supported formats: PNG, JPG, JPEG, BMP, TIFF, TIF
help-limits = File size limit: 10MB for JPEG, 5MB for other formats
help-commands = Commands:
help-start = /start - Welcome message
help-help = /help - This help message
help-tips = Tips:
help-tip1 = • Use clear, well-lit images
help-tip2 = • Ensure text is readable and not too small
help-tip3 = • Avoid blurry or distorted images
help-tip4 = • Supported languages: English + French
help-final = Need help? Just send me an image! 😊

# Error messages
error-download-failed = ❌ Failed to download the image. Please try again.
error-unsupported-format = ❌ Unsupported image format. Please use PNG, JPG, JPEG, BMP, TIFF, or TIF formats.
error-no-text-found = ⚠️ No text was found in the image. Please try a clearer image with visible text.
error-ocr-initialization = ❌ OCR engine initialization failed. Please try again later.
error-ocr-extraction = ❌ Failed to extract text from the image. Please try again with a different image.
error-ocr-timeout = ❌ OCR processing timed out: {$msg}
error-ocr-corruption = ❌ OCR engine encountered an internal error. Please try again.
error-ocr-exhaustion = ❌ System resources are exhausted. Please try again later.
error-validation = ❌ Image validation failed: {$msg}
error-image-load = ❌ The image format is not supported or the image is corrupted. Please try with a PNG, JPG, or BMP image.

# Success messages
success-extraction = ✅ **Text extracted successfully!**
success-extracted-text = 📝 **Extracted Text:**
success-photo-downloaded = Photo downloaded successfully! Processing...
success-document-downloaded = Image document downloaded successfully! Processing...

# Ingredient processing messages
ingredients-found = Ingredients Found!
no-ingredients-found = No Ingredients Detected
no-ingredients-suggestion = I couldn't find any measurements or ingredients in the text. Try sending a clearer image of a recipe or ingredient list.
line = Line
unknown-ingredient = Unknown ingredient
total-ingredients = Total ingredients found
original-text = Original extracted text
error-processing-failed = Failed to process ingredients
error-try-again = Please try again with a different image.

# Processing messages
processing-photo = Photo downloaded successfully! Processing...
processing-document = Image document downloaded successfully! Processing...

# Unsupported message types
unsupported-title = 🤔 I can only process text messages and images.
unsupported-description = What I can do:
unsupported-feature1 = 📸 Send photos of text you want to extract
unsupported-feature2 = 📄 Send image files (PNG, JPG, JPEG, BMP, TIFF, TIF)
unsupported-feature3 = 💬 Send /start to see the welcome message
unsupported-feature4 = ❓ Send /help for detailed instructions
unsupported-final = Try sending me an image with text! 📝

# Regular text responses
text-response = Received: {$text}
text-tip = 💡 Tip: Send me an image with text to extract it using OCR!

# Recipe name dialogue messages
recipe-name-prompt = 🏷️ What would you like to call this recipe?
recipe-name-prompt-hint = Please enter a name for your recipe (e.g., "Chocolate Chip Cookies", "Mom's Lasagna")
recipe-name-invalid = ❌ Recipe name cannot be empty. Please enter a valid name for your recipe.
recipe-name-too-long = ❌ Recipe name is too long (maximum 255 characters). Please enter a shorter name.
recipe-complete = ✅ Recipe "{$recipe_name}" saved successfully with {$ingredient_count} ingredients!

# Ingredient review messages
review-title = Review Your Ingredients
review-description = Please review the extracted ingredients below. Use the buttons to edit or delete items, then confirm when ready.
review-confirm = Confirm and Save
review-cancelled = ❌ Ingredient review cancelled. No ingredients were saved.
review-no-ingredients = No ingredients remaining
review-no-ingredients-help = All ingredients have been deleted. You can add more ingredients by sending another image, or cancel this recipe.
review-add-more = Add More Ingredients
review-add-more-instructions = Send another image with ingredients to add them to this recipe.
cancel = Cancel
edit-ingredient-prompt = Enter the corrected ingredient text
current-ingredient = Current ingredient
edit-empty = Ingredient text cannot be empty.
edit-invalid-format = Invalid ingredient format. Please enter something like "2 cups flour" or "3 eggs".
edit-try-again = Please try again with a valid ingredient format.
edit-too-long = Ingredient text is too long (maximum 200 characters). Please enter a shorter description.
edit-no-ingredient-name = Please specify an ingredient name (e.g., "2 cups flour" not just "2 cups").
edit-ingredient-name-too-long = Ingredient name is too long (maximum 100 characters). Please use a shorter name.
edit-invalid-quantity = Invalid quantity. Please use a positive number (e.g., "2.5 cups flour").
error-invalid-edit = Invalid ingredient index for editing.
review-help = Please reply with "confirm" to save these ingredients, or "cancel" to discard them.

# Document messages
document-image = Received image document from user {$user_id}
document-non-image = Received non-image document from user {$user_id}
document-no-mime = Received document without MIME type from user {$user_id}

# Photo messages
photo-received = Received photo from user {$user_id}

# Text messages
text-received = Received text message from user {$user_id}: {$text}

# Unsupported messages
unsupported-received = Received unsupported message type from user {$user_id}
