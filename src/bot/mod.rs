//! Bot module for handling Telegram interactions
//!
//! This module is split into several submodules for better organization:
//! - `message_handler`: Handles incoming text, photo, and document messages
//! - `callback_handler`: Handles inline keyboard callback queries
//! - `ui_builder`: Creates keyboards and formats messages
//! - `dialogue_manager`: Manages dialogue state transitions and validation

pub mod callback_handler;
pub mod dialogue_manager;
pub mod message_handler;
pub mod ui_builder;

// Re-export main handler functions for use in main.rs
pub use message_handler::message_handler;
pub use callback_handler::callback_handler;

// Re-export utility functions that might be used elsewhere
pub use ui_builder::{format_ingredients_list, create_ingredient_review_keyboard};
pub use message_handler::{download_file, download_and_process_image, process_ingredients_and_extract_matches};
pub use dialogue_manager::{save_ingredients_to_database, parse_ingredient_from_text};