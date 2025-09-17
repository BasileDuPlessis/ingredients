//! # Integration Example for Ingredient Parsing
//!
//! This module demonstrates how to integrate the structured ingredient parsing
//! with the existing OCR pipeline and database storage.

use crate::ingredient_model::IngredientList;
use crate::ingredient_parser::parse_ingredient_list;
use crate::db::{create_ingredient_entry, read_ingredient_entry, get_parsed_ingredients};
use anyhow::Result;
use log::info;
use rusqlite::Connection;

/// Process OCR text through the full ingredient parsing pipeline
pub fn process_ocr_text_with_structured_parsing(
    conn: &Connection,
    telegram_id: i64,
    ocr_text: &str,
) -> Result<IngredientList> {
    info!("Processing OCR text with structured parsing for user {}", telegram_id);
    
    // Parse the OCR text into structured ingredients
    let ingredient_list = parse_ingredient_list(ocr_text);
    
    info!(
        "Parsed {} ingredients with {:.1}% confidence, {} unparsed lines",
        ingredient_list.parsed_count(),
        ingredient_list.overall_confidence * 100.0,
        ingredient_list.unparsed_count()
    );
    
    // Store the structured data in the database
    let entry_id = create_ingredient_entry(conn, telegram_id, &ingredient_list)?;
    
    info!("Stored structured ingredient data with entry ID: {}", entry_id);
    
    Ok(ingredient_list)
}

/// Retrieve and format structured ingredients for display
pub fn format_parsed_ingredients_for_display(
    conn: &Connection,
    entry_id: i64,
) -> Result<String> {
    let entry = read_ingredient_entry(conn, entry_id)?
        .ok_or_else(|| anyhow::anyhow!("Entry not found"))?;
    
    let parsed = get_parsed_ingredients(&entry)?;
    
    let mut output = String::new();
    output.push_str(&format!("📝 **Parsed Ingredients** (Confidence: {:.1}%)\n\n", 
                             parsed.overall_confidence * 100.0));
    
    for (i, ingredient) in parsed.ingredients.iter().enumerate() {
        output.push_str(&format!("{}. {}\n", i + 1, ingredient));
    }
    
    if !parsed.unparsed_lines.is_empty() {
        output.push_str("\n❓ **Could not parse:**\n");
        for line in &parsed.unparsed_lines {
            output.push_str(&format!("• {}\n", line));
        }
    }
    
    Ok(output)
}

/// Generate a summary of ingredient quantities by type
pub fn generate_ingredient_summary(ingredient_list: &IngredientList) -> String {
    let mut volume_items = Vec::new();
    let mut weight_items = Vec::new();
    let mut count_items = Vec::new();
    let mut ambiguous_items = Vec::new();
    
    for ingredient in &ingredient_list.ingredients {
        if let Some(quantity) = &ingredient.quantity {
            if quantity.is_ambiguous() {
                ambiguous_items.push(&ingredient.name);
            } else if quantity.unit.is_volume() {
                volume_items.push(&ingredient.name);
            } else if quantity.unit.is_weight() {
                weight_items.push(&ingredient.name);
            } else if quantity.unit.is_count() {
                count_items.push(&ingredient.name);
            }
        }
    }
    
    let mut summary = String::new();
    summary.push_str("📊 **Ingredient Summary**\n\n");
    
    if !volume_items.is_empty() {
        summary.push_str(&format!("🥤 **Volume ingredients:** {}\n", volume_items.join(", ")));
    }
    if !weight_items.is_empty() {
        summary.push_str(&format!("⚖️ **Weight ingredients:** {}\n", weight_items.join(", ")));
    }
    if !count_items.is_empty() {
        summary.push_str(&format!("🔢 **Count ingredients:** {}\n", count_items.join(", ")));
    }
    if !ambiguous_items.is_empty() {
        summary.push_str(&format!("❓ **To taste/optional:** {}\n", ambiguous_items.join(", ")));
    }
    
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_database_schema;
    use tempfile::NamedTempFile;

    fn setup_test_db() -> Result<(Connection, NamedTempFile)> {
        let temp_file = NamedTempFile::new()?;
        let conn = Connection::open(temp_file.path())?;
        init_database_schema(&conn)?;
        Ok((conn, temp_file))
    }

    #[test]
    fn test_full_integration_pipeline() -> Result<()> {
        let (conn, _temp_file) = setup_test_db()?;
        
        let telegram_id = 12345;
        let ocr_text = "2 cups all-purpose flour\n1/2 cup sugar\n3 large eggs\nsalt to taste";
        
        // Process through the full pipeline
        let ingredient_list = process_ocr_text_with_structured_parsing(&conn, telegram_id, ocr_text)?;
        
        // Verify parsing results
        assert_eq!(ingredient_list.parsed_count(), 4);
        assert!(ingredient_list.overall_confidence > 0.7);
        
        // Test display formatting
        let display_text = format_parsed_ingredients_for_display(&conn, 1)?;
        assert!(display_text.contains("all-purpose flour"));
        assert!(display_text.contains("Confidence:"));
        
        // Test summary generation
        let summary = generate_ingredient_summary(&ingredient_list);
        assert!(summary.contains("Volume ingredients"));
        assert!(summary.contains("To taste"));
        
        Ok(())
    }

    #[test]
    fn test_ingredient_summary_categorization() -> Result<()> {
        use crate::ingredient_model::{Ingredient, Quantity, Unit};
        
        let mut ingredient_list = IngredientList::new("test ingredients".to_string());
        
        // Add different types of ingredients
        ingredient_list.add_ingredient(
            Ingredient::new("flour").with_quantity(Quantity::exact(2.0, Unit::Cups))
        );
        ingredient_list.add_ingredient(
            Ingredient::new("butter").with_quantity(Quantity::exact(0.5, Unit::Pounds))
        );
        ingredient_list.add_ingredient(
            Ingredient::new("eggs").with_quantity(Quantity::exact(3.0, Unit::Pieces))
        );
        ingredient_list.add_ingredient(
            Ingredient::new("salt").with_quantity(Quantity::ambiguous("to taste", Unit::Unknown("".to_string())))
        );
        
        let summary = generate_ingredient_summary(&ingredient_list);
        
        assert!(summary.contains("Volume ingredients: flour"));
        assert!(summary.contains("Weight ingredients: butter"));
        assert!(summary.contains("Count ingredients: eggs"));
        assert!(summary.contains("To taste/optional: salt"));
        
        Ok(())
    }
}