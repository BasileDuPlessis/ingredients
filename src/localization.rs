use anyhow::Result;
use fluent_bundle::{FluentBundle, FluentResource};
use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use unic_langid::LanguageIdentifier;

/// Localization manager for the Ingredients Bot
pub struct LocalizationManager {
    bundles: HashMap<String, FluentBundle<FluentResource>>,
}

impl LocalizationManager {
    /// Create a new localization manager
    pub fn new() -> Result<Self> {
        let mut bundles = HashMap::new();

        // Load available locales
        let locales = vec!["en", "fr"];

        for locale_str in locales {
            let locale: LanguageIdentifier = locale_str.parse()?;
            let bundle = Self::create_bundle(&locale)?;
            bundles.insert(locale_str.to_string(), bundle);
        }

        Ok(Self { bundles })
    }

    /// Create a fluent bundle for a specific locale
    fn create_bundle(locale: &LanguageIdentifier) -> Result<FluentBundle<FluentResource>> {
        let mut bundle = FluentBundle::new(vec![locale.clone()]);

        // Load the main resource file - path relative to Cargo.toml
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let resource_path = format!("{}/locales/{}/main.ftl", manifest_dir, locale);
        if let Ok(content) = fs::read_to_string(&resource_path) {
            if let Ok(resource) = FluentResource::try_new(content) {
                let _ = bundle.add_resource(resource);
            }
        }

        Ok(bundle)
    }

    /// Get a localized message in a specific language
    pub fn get_message_in_language(
        &self,
        key: &str,
        language: &str,
        args: Option<&HashMap<&str, &str>>,
    ) -> String {
        let bundle = match self.bundles.get(language) {
            Some(bundle) => bundle,
            None => {
                // Fallback to English if language not found
                match self.bundles.get("en") {
                    Some(bundle) => bundle,
                    None => return format!("Missing translation: {}", key),
                }
            }
        };

        let msg = match bundle.get_message(key) {
            Some(msg) => msg,
            None => return format!("Missing translation: {}", key),
        };

        let pattern = match msg.value() {
            Some(pattern) => pattern,
            None => return format!("Missing value for key: {}", key),
        };

        let mut value = String::new();

        if let Some(args) = args {
            let fluent_args = fluent_bundle::FluentArgs::from_iter(
                args.iter()
                    .map(|(k, v)| (*k, fluent_bundle::FluentValue::from(*v))),
            );

            let _ = bundle.write_pattern(&mut value, pattern, Some(&fluent_args), &mut vec![]);
        } else {
            let _ = bundle.write_pattern(&mut value, pattern, None, &mut vec![]);
        }

        value
    }

    /// Get a localized message with arguments in a specific language
    pub fn get_message_with_args_in_language(
        &self,
        key: &str,
        language: &str,
        args: &[(&str, &str)],
    ) -> String {
        let args_map: HashMap<&str, &str> = args.iter().cloned().collect();
        self.get_message_in_language(key, language, Some(&args_map))
    }

    /// Check if a language is supported
    pub fn is_language_supported(&self, language: &str) -> bool {
        self.bundles.contains_key(language)
    }
}

/// Global localization instance - not thread-safe, use with caution in multi-threaded environments
static mut LOCALIZATION_MANAGER: Option<LocalizationManager> = None;
static LOCALIZATION_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize the global localization manager
pub fn init_localization() -> Result<()> {
    if !LOCALIZATION_INITIALIZED.load(Ordering::SeqCst) {
        unsafe {
            LOCALIZATION_MANAGER = Some(LocalizationManager::new()?);
        }
        LOCALIZATION_INITIALIZED.store(true, Ordering::SeqCst);
    }
    Ok(())
}

/// Get the global localization manager
#[allow(static_mut_refs)]
pub fn get_localization_manager() -> &'static LocalizationManager {
    unsafe {
        LOCALIZATION_MANAGER
            .as_ref()
            .expect("Localization manager not initialized")
    }
}

/// Convenience function to get a localized message in user's language
pub fn t_lang(key: &str, language_code: Option<&str>) -> String {
    let language = detect_language(language_code);
    get_localization_manager().get_message_in_language(key, &language, None)
}

/// Convenience function to get a localized message with arguments in user's language
pub fn t_args_lang(key: &str, args: &[(&str, &str)], language_code: Option<&str>) -> String {
    let language = detect_language(language_code);
    get_localization_manager().get_message_with_args_in_language(key, &language, args)
}

/// Detect the appropriate language based on user's Telegram language code
pub fn detect_language(language_code: Option<&str>) -> String {
    if let Some(code) = language_code {
        // Extract language code (e.g., "fr-FR" -> "fr", "en-US" -> "en")
        let lang = if code.contains('-') {
            code.split('-').next().unwrap_or("en")
        } else {
            code
        };

        // Check if we support this language
        if get_localization_manager().is_language_supported(lang) {
            return lang.to_string();
        }
    }

    // Default to English if language not supported or not provided
    "en".to_string()
}
