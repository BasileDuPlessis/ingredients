use fluent_bundle::{FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;
use std::sync::Arc;
use std::collections::HashMap;
use std::fs;
use anyhow::Result;

/// Localization manager for the Ingredients Bot
pub struct LocalizationManager {
    bundles: HashMap<String, Arc<FluentBundle<FluentResource>>>,
}

impl LocalizationManager {
    /// Create a new localization manager
    pub fn new() -> Result<Self> {
        let mut bundles = HashMap::new();

        // Load English bundle
        let en_locale: LanguageIdentifier = "en".parse()?;
        let bundle = Self::create_bundle(&en_locale)?;
        bundles.insert("en".to_string(), Arc::new(bundle));

        Ok(Self { bundles })
    }

    /// Create a fluent bundle for a specific locale
    fn create_bundle(locale: &LanguageIdentifier) -> Result<FluentBundle<FluentResource>> {
        let mut bundle = FluentBundle::new(vec![locale.clone()]);

        // Load the main resource file
        let resource_path = format!("./locales/{}/main.ftl", locale);
        if let Ok(content) = fs::read_to_string(&resource_path) {
            if let Ok(resource) = FluentResource::try_new(content) {
                let _ = bundle.add_resource(resource);
            }
        }

        Ok(bundle)
    }

    /// Get a localized message
    pub fn get_message(&self, key: &str, args: Option<&HashMap<&str, &str>>) -> String {
        let bundle = self.bundles.get("en").unwrap();

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
                args.iter().map(|(k, v)| (*k, fluent_bundle::FluentValue::from(*v)))
            );

            let _ = bundle.write_pattern(&mut value, pattern, Some(&fluent_args), &mut vec![]);
        } else {
            let _ = bundle.write_pattern(&mut value, pattern, None, &mut vec![]);
        }

        value
    }

    /// Get a localized message with simple string arguments
    pub fn get_message_with_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        let args_map: HashMap<&str, &str> = args.iter().cloned().collect();
        self.get_message(key, Some(&args_map))
    }
}

/// Global localization instance
static mut LOCALIZATION_MANAGER: Option<LocalizationManager> = None;

/// Initialize the global localization manager
pub fn init_localization() -> Result<()> {
    let manager = LocalizationManager::new()?;
    unsafe {
        LOCALIZATION_MANAGER = Some(manager);
    }
    Ok(())
}

/// Get the global localization manager
pub fn get_localization_manager() -> &'static LocalizationManager {
    unsafe {
        LOCALIZATION_MANAGER.as_ref().expect("Localization manager not initialized")
    }
}

/// Convenience function to get a localized message
pub fn t(key: &str) -> String {
    get_localization_manager().get_message(key, None)
}

/// Convenience function to get a localized message with arguments
pub fn t_args(key: &str, args: &[(&str, &str)]) -> String {
    get_localization_manager().get_message_with_args(key, args)
}
