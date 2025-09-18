# Ingredients Bot - English Localization
# Main welcome and help messages

welcome-title = Welcome to Ingredients Bot!
welcome-description = I'm your OCR assistant that can extract text from images. Here's what I can do:
welcome-features =
    📸 **Send me photos** of ingredient lists, recipes, or any text you want to extract
    📄 **Send me image files** (PNG, JPG, JPEG, BMP, TIFF, TIF)
    🔍 **I'll process them with OCR** and send back the extracted text
    🥘 **For ingredient lists**, I'll also parse and structure them automatically
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
help-step5 = 5. 🥘 For ingredient lists, I'll also show structured ingredients
help-formats = Supported formats: PNG, JPG, JPEG, BMP, TIFF, TIF
help-limits = File size limit: 10MB for JPEG, 5MB for other formats
help-ingredient-example = Example: "1 cup sugar" becomes quantity=1, measurement=cup, ingredient=sugar
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
