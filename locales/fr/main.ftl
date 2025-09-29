# Ingredients Bot - Localisation Française
# Messages principaux de bienvenue et d'aide

welcome-title = Bienvenue sur Ingredients Bot !
welcome-description = Je suis votre assistant OCR qui peut extraire le texte des images. Voici ce que je peux faire :
welcome-features =
    📸 **Envoyez-moi des photos** de listes d'ingrédients, de recettes ou de tout texte à extraire
    📄 **Envoyez-moi des fichiers image** (PNG, JPG, JPEG, BMP, TIFF, TIF)
    🔍 **Je les traiterai avec OCR** et vous renverrai le texte extrait
    💾 **Tout texte extrait est stocké** pour référence future
welcome-commands = Commandes :
welcome-start = /start - Afficher ce message de bienvenue
welcome-help = /help - Obtenir de l'aide et des instructions d'utilisation
welcome-send-image = Envoyez-moi simplement une image et je m'occupe du reste ! 🚀

help-title = 🆘 Aide d'Ingredients Bot
help-description = Comment m'utiliser :
help-step1 = 1. 📸 Envoyer une photo de texte à extraire
help-step2 = 2. 📎 Ou envoyer un fichier image (PNG, JPG, JPEG, BMP, TIFF, TIF)
help-step3 = 3. ⏳ Je le traiterai avec la technologie OCR
help-step4 = 4. 📝 Vous recevrez le texte extrait
help-formats = Formats supportés : PNG, JPG, JPEG, BMP, TIFF, TIF
help-limits = Limite de taille de fichier : 10 Mo pour JPEG, 5 Mo pour les autres formats
help-commands = Commandes :
help-start = /start - Message de bienvenue
help-help = /help - Ce message d'aide
help-tips = Conseils :
help-tip1 = • Utilisez des images claires et bien éclairées
help-tip2 = • Assurez-vous que le texte est lisible et pas trop petit
help-tip3 = • Évitez les images floues ou déformées
help-tip4 = • Langues supportées : Anglais + Français
help-final = Besoin d'aide ? Envoyez-moi simplement une image ! 😊

# Messages d'erreur
error-download-failed = ❌ Échec du téléchargement de l'image. Veuillez réessayer.
error-unsupported-format = ❌ Format d'image non supporté. Veuillez utiliser les formats PNG, JPG, JPEG, BMP, TIFF ou TIF.
error-no-text-found = ⚠️ Aucun texte n'a été trouvé dans l'image. Essayez avec une image plus claire contenant du texte visible.
error-ocr-initialization = ❌ L'initialisation du moteur OCR a échoué. Veuillez réessayer plus tard.
error-ocr-extraction = ❌ Échec de l'extraction du texte de l'image. Essayez avec une image différente.
error-ocr-timeout = ❌ Le traitement OCR a expiré : {$msg}
error-ocr-corruption = ❌ Le moteur OCR a rencontré une erreur interne. Veuillez réessayer.
error-ocr-exhaustion = ❌ Les ressources système sont épuisées. Veuillez réessayer plus tard.
error-validation = ❌ La validation de l'image a échoué : {$msg}
error-image-load = ❌ Le format d'image n'est pas supporté ou l'image est corrompue. Essayez avec une image PNG, JPG ou BMP.

# Messages de succès
success-extraction = ✅ **Texte extrait avec succès !**
success-extracted-text = 📝 **Texte extrait :**
success-photo-downloaded = Photo téléchargée avec succès ! Traitement en cours...
success-document-downloaded = Document image téléchargé avec succès ! Traitement en cours...

# Messages de traitement des ingrédients
ingredients-found = Ingrédients trouvés !
no-ingredients-found = Aucun ingrédient détecté
no-ingredients-suggestion = Je n'ai pas pu trouver de mesures ou d'ingrédients dans le texte. Essayez d'envoyer une image plus claire d'une recette ou d'une liste d'ingrédients.
line = Ligne
unknown-ingredient = Ingrédient inconnu
total-ingredients = Total des ingrédients trouvés
original-text = Texte extrait original
error-processing-failed = Échec du traitement des ingrédients
error-try-again = Veuillez réessayer avec une image différente.

# Messages de traitement
processing-photo = Photo téléchargée avec succès ! Traitement en cours...
processing-document = Document image téléchargé avec succès ! Traitement en cours...

# Types de messages non supportés
unsupported-title = 🤔 Je ne peux traiter que les messages texte et les images.
unsupported-description = Ce que je peux faire :
unsupported-feature1 = 📸 Envoyer des photos de texte à extraire
unsupported-feature2 = 📄 Envoyer des fichiers image (PNG, JPG, JPEG, BMP, TIFF, TIF)
unsupported-feature3 = 💬 Envoyer /start pour voir le message de bienvenue
unsupported-feature4 = ❓ Envoyer /help pour des instructions détaillées
unsupported-final = Essayez d'envoyer une image avec du texte ! 📝

# Réponses texte régulières
text-response = Reçu : {$text}
text-tip = 💡 Conseil : Envoyez-moi une image avec du texte pour l'extraire avec OCR !

# Messages de dialogue pour le nom de recette
recipe-name-prompt = 🏷️ Comment souhaitez-vous nommer cette recette ?
recipe-name-prompt-hint = Veuillez entrer un nom pour votre recette (par ex. "Cookies aux pépites de chocolat", "Lasagnes de Maman")
recipe-name-invalid = ❌ Le nom de recette ne peut pas être vide. Veuillez entrer un nom valide pour votre recette.
recipe-name-too-long = ❌ Le nom de recette est trop long (maximum 255 caractères). Veuillez entrer un nom plus court.
recipe-complete = ✅ Recette "{$recipe_name}" sauvegardée avec succès avec {$ingredient_count} ingrédients !

# Messages de document
document-image = Document image reçu de l'utilisateur {$user_id}
document-non-image = Document non-image reçu de l'utilisateur {$user_id}
document-no-mime = Document sans type MIME reçu de l'utilisateur {$user_id}

# Messages photo
photo-received = Photo reçue de l'utilisateur {$user_id}

# Messages texte
text-received = Message texte reçu de l'utilisateur {$user_id} : {$text}

# Messages non supportés
unsupported-received = Type de message non supporté reçu de l'utilisateur {$user_id}
