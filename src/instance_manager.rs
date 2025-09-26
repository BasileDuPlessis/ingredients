//! # OCR Instance Manager Module
//!
//! This module provides thread-safe OCR instance management for reusing Tesseract instances.
//! Reusing instances significantly improves performance by avoiding initialization overhead.

use leptess::LepTess;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::ocr_config::OcrConfig;

/// Thread-safe OCR instance manager for reusing Tesseract instances
///
/// Manages a pool of Tesseract OCR instances keyed by language configuration.
/// Reusing instances significantly improves performance by avoiding the overhead
/// of creating new Tesseract instances for each OCR operation.
///
/// # Performance Benefits
///
/// - Eliminates Tesseract initialization overhead (~100-500ms per instance)
/// - Reduces memory allocations for repeated OCR operations
/// - Thread-safe with Arc<Mutex<>> for concurrent access
///
/// # Instance Lifecycle
///
/// - Instances are created on first request for a language combination
/// - Instances are reused for subsequent requests with same language config
/// - Instances persist until explicitly removed or manager is dropped
///
/// # Thread Safety
///
/// Uses `Mutex<HashMap<>>` internally for thread-safe instance management.
/// Multiple threads can safely request instances concurrently.
///
/// # Memory Management
///
/// - Each language combination maintains one instance
/// - Memory usage scales with number of unique language combinations
/// - Consider memory limits for applications with many language combinations
pub struct OcrInstanceManager {
    instances: Mutex<HashMap<String, Arc<Mutex<LepTess>>>>,
}

impl OcrInstanceManager {
    /// Create a new OCR instance manager
    ///
    /// Initializes an empty instance pool. Instances will be created
    /// on-demand when first requested via `get_instance()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingredients::instance_manager::OcrInstanceManager;
    ///
    /// let manager = OcrInstanceManager::new();
    /// // Manager is ready to provide OCR instances
    /// ```
    pub fn new() -> Self {
        Self {
            instances: Mutex::new(HashMap::new()),
        }
    }

    /// Get or create an OCR instance for the given configuration
    ///
    /// Returns an existing instance if one exists for the language configuration,
    /// otherwise creates a new instance and stores it for future reuse.
    ///
    /// # Arguments
    ///
    /// * `config` - OCR configuration containing language settings and other options
    ///
    /// # Returns
    ///
    /// Returns `Result<Arc<Mutex<LepTess>>, anyhow::Error>` containing the OCR instance
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use ingredients::instance_manager::OcrInstanceManager;
    /// use ingredients::ocr_config::OcrConfig;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = OcrInstanceManager::new();
    /// let config = OcrConfig::default();
    ///
    /// let instance = manager.get_instance(&config)?;
    /// // Use the instance for OCR processing
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if Tesseract instance creation fails (e.g., invalid language codes)
    ///
    /// # Performance
    ///
    /// - First call for a language: ~100-500ms (Tesseract initialization)
    /// - Subsequent calls: ~1ms (instance lookup and Arc clone)
    pub fn get_instance(&self, config: &OcrConfig) -> anyhow::Result<Arc<Mutex<LepTess>>> {
        let key = config.languages.clone();

        // Try to get existing instance
        {
            let instances = self.instances.lock().unwrap();
            if let Some(instance) = instances.get(&key) {
                return Ok(Arc::clone(instance));
            }
        }

        // Create new instance if none exists
        log::info!("Creating new OCR instance for languages: {key}");
        let tess = LepTess::new(None, &key)
            .map_err(|e| anyhow::anyhow!("Failed to initialize Tesseract OCR instance: {}", e))?;

        let instance = Arc::new(Mutex::new(tess));

        // Store the instance
        {
            let mut instances = self.instances.lock().unwrap();
            instances.insert(key, Arc::clone(&instance));
        }

        Ok(instance)
    }

    /// Remove an instance (useful for cleanup or when configuration changes)
    pub fn _remove_instance(&self, languages: &str) {
        let mut instances = self.instances.lock().unwrap();
        if instances.remove(languages).is_some() {
            log::info!("Removed OCR instance for languages: {languages}");
        }
    }

    /// Clear all instances (useful for memory cleanup)
    pub fn _clear_all_instances(&self) {
        let mut instances = self.instances.lock().unwrap();
        let count = instances.len();
        instances.clear();
        if count > 0 {
            log::info!("Cleared {count} OCR instances");
        }
    }

    /// Get the number of cached instances
    pub fn _instance_count(&self) -> usize {
        let instances = self.instances.lock().unwrap();
        instances.len()
    }
}

impl Default for OcrInstanceManager {
    fn default() -> Self {
        Self::new()
    }
}