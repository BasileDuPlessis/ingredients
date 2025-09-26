//! # Circuit Breaker Module
//!
//! This module implements the circuit breaker pattern for OCR operations.
//! It prevents cascading failures by temporarily stopping requests when
//! OCR operations fail repeatedly.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::ocr_config::RecoveryConfig;

/// Circuit breaker for OCR operations
///
/// Implements circuit breaker pattern to prevent cascading failures in OCR processing.
/// When OCR operations fail repeatedly, the circuit breaker "opens" to stop further
/// attempts and allow the system to recover.
///
/// # State Machine
///
/// - **Closed**: Normal operation, requests pass through
/// - **Open**: Failure threshold exceeded, requests fail fast
/// - **Half-Open**: Testing if service has recovered
///
/// # Configuration
///
/// Uses `RecoveryConfig` for:
/// - `circuit_breaker_threshold`: Failures before opening (default: 5)
/// - `circuit_breaker_reset_secs`: Time before attempting reset (default: 60s)
#[derive(Debug)]
pub struct CircuitBreaker {
    failure_count: Mutex<u32>,
    last_failure_time: Mutex<Option<Instant>>,
    config: RecoveryConfig,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Recovery configuration with circuit breaker settings
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingredients::ocr_config::RecoveryConfig;
    /// use ingredients::circuit_breaker::CircuitBreaker;
    ///
    /// let config = RecoveryConfig::default();
    /// let circuit_breaker = CircuitBreaker::new(config);
    /// ```
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            failure_count: Mutex::new(0),
            last_failure_time: Mutex::new(None),
            config,
        }
    }

    /// Check if circuit breaker is open (blocking requests)
    ///
    /// # Returns
    ///
    /// `true` if circuit is open and should block requests, `false` if closed
    ///
    /// # Behavior
    ///
    /// - Returns `true` when failure count >= threshold and reset time hasn't elapsed
    /// - Automatically resets to closed state after reset timeout
    /// - Thread-safe using internal mutexes
    pub fn is_open(&self) -> bool {
        let failure_count = *self.failure_count.lock().unwrap();
        let last_failure = *self.last_failure_time.lock().unwrap();

        if failure_count >= self.config.circuit_breaker_threshold {
            if let Some(last_time) = last_failure {
                let elapsed = last_time.elapsed();
                if elapsed < Duration::from_secs(self.config.circuit_breaker_reset_secs) {
                    return true; // Circuit is still open
                }
                // Reset circuit breaker
                *self.failure_count.lock().unwrap() = 0;
                *self.last_failure_time.lock().unwrap() = None;
            }
        }
        false
    }

    /// Record a failure to increment the failure counter
    ///
    /// Should be called whenever an OCR operation fails.
    /// Updates failure count and last failure timestamp.
    ///
    /// # Thread Safety
    ///
    /// Uses internal mutex for thread-safe updates.
    pub fn record_failure(&self) {
        *self.failure_count.lock().unwrap() += 1;
        *self.last_failure_time.lock().unwrap() = Some(Instant::now());
    }

    /// Record a success to reset the failure counter
    ///
    /// Should be called whenever an OCR operation succeeds.
    /// Resets failure count and clears last failure timestamp.
    ///
    /// # Thread Safety
    ///
    /// Uses internal mutex for thread-safe updates.
    pub fn record_success(&self) {
        *self.failure_count.lock().unwrap() = 0;
        *self.last_failure_time.lock().unwrap() = None;
    }
}