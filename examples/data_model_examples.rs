//! # Ingredient Data Model Examples
//!
//! This file provides comprehensive examples of how to use the ingredient
//! and quantity extraction data model.

use ingredients::ingredient_model::{Ingredient, IngredientList, Quantity, QuantityType, Unit};
use ingredients::ingredient_parser::parse_ingredient_list;

fn main() {
    println!("🧑‍🍳 Ingredient Data Model Examples\n");

    // Example 1: Creating ingredients manually
    println!("📝 Example 1: Manual Ingredient Creation");
    let flour = Ingredient::new("all-purpose flour")
        .with_quantity(Quantity::exact(2.0, Unit::Cups))
        .with_modifier("sifted")
        .with_confidence(1.0);
    
    let sugar = Ingredient::new("sugar")
        .with_quantity(Quantity::fraction(Some(1), 1, 2, Unit::Cups));
    
    let eggs = Ingredient::new("eggs")
        .with_quantity(Quantity::range(2.0, 3.0, Unit::Pieces))
        .with_modifier("large");
    
    let salt = Ingredient::new("salt")
        .with_quantity(Quantity::ambiguous("to taste", Unit::Pinches));

    println!("  • {}", flour);
    println!("  • {}", sugar);
    println!("  • {}", eggs);
    println!("  • {}", salt);
    println!();

    // Example 2: Parsing ingredient text
    println!("📝 Example 2: Parsing Ingredient Text");
    let recipe_text = r#"
2 cups all-purpose flour
1/2 cup granulated sugar
3 large eggs
1-2 tablespoons milk
1 teaspoon vanilla extract
salt to taste
butter for greasing
"#;

    let parsed = parse_ingredient_list(recipe_text.trim());
    println!("Original text:\n{}", parsed.original_text);
    println!("\nParsed ingredients:");
    for (i, ingredient) in parsed.ingredients.iter().enumerate() {
        println!("  {}. {}", i + 1, ingredient);
    }
    
    if !parsed.unparsed_lines.is_empty() {
        println!("\nCould not parse:");
        for line in &parsed.unparsed_lines {
            println!("  ? {}", line);
        }
    }
    
    println!("\nParsing confidence: {:.1}%", parsed.overall_confidence * 100.0);
    println!("Success rate: {:.1}%", parsed.success_rate() * 100.0);
    println!();

    // Example 3: Different quantity types
    println!("📝 Example 3: Quantity Type Examples");
    
    // Exact quantities
    let qty1 = Quantity::exact(2.5, Unit::Cups);
    println!("  Exact: {} (estimated: {:?})", qty1, qty1.estimated_value());
    
    // Fractions
    let qty2 = Quantity::fraction(Some(2), 3, 4, Unit::Tablespoons);
    println!("  Fraction: {} (estimated: {:?})", qty2, qty2.estimated_value());
    
    // Ranges
    let qty3 = Quantity::range(1.0, 2.0, Unit::Teaspoons);
    println!("  Range: {} (estimated: {:?})", qty3, qty3.estimated_value());
    
    // Ambiguous
    let qty4 = Quantity::ambiguous("a pinch", Unit::Unknown("".to_string()));
    println!("  Ambiguous: {} (estimated: {:?})", qty4, qty4.estimated_value());
    println!();

    // Example 4: Unit categorization
    println!("📝 Example 4: Unit Categories");
    let units = vec![
        Unit::Cups, Unit::Tablespoons, Unit::Pounds, Unit::Grams, 
        Unit::Pieces, Unit::Cloves, Unit::Unknown("bottles".to_string())
    ];
    
    for unit in units {
        let category = if unit.is_volume() {
            "Volume"
        } else if unit.is_weight() {
            "Weight"
        } else if unit.is_count() {
            "Count"
        } else {
            "Other"
        };
        println!("  {}: {} ({})", unit.display_name(), category, 
                 if matches!(unit, Unit::Unknown(_)) { "unknown" } else { "known" });
    }
    println!();

    // Example 5: Edge cases and multi-language
    println!("📝 Example 5: Edge Cases and Multi-Language");
    
    let edge_cases = vec![
        "250 g farine",                    // French
        "2 cas huile d'olive",            // French abbreviation
        "1⁄2 teaspoon vanilla",           // Unicode fraction
        "2-3 medium onions (diced)",      // Range with modifier
        "salt and pepper to taste",       // Ambiguous compound
        "eggs",                           // No quantity
        "1.5 lbs ground beef",            // Decimal
        "a handful of fresh herbs",       // Ambiguous text
    ];
    
    for case in edge_cases {
        let parsed = parse_ingredient_list(case);
        if let Some(ingredient) = parsed.ingredients.first() {
            println!("  '{}' → {}", case, ingredient);
        } else {
            println!("  '{}' → [unparsed]", case);
        }
    }
    println!();

    // Example 6: Confidence scoring
    println!("📝 Example 6: Confidence Scoring");
    
    let test_cases = vec![
        "2 cups flour",                    // Perfect parsing
        "some salt",                       // Ambiguous quantity
        "flour",                           // No quantity
        "mysterious ingredient xyz",       // Unclear
    ];
    
    for case in test_cases {
        let parsed = parse_ingredient_list(case);
        println!("  '{}' → confidence: {:.1}%", case, parsed.overall_confidence * 100.0);
    }
    
    println!("\n✅ All examples completed!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples_compile() {
        // This test ensures all the examples compile and run without panicking
        main();
    }

    #[test]
    fn test_quantity_estimation() {
        let qty = Quantity::fraction(Some(2), 1, 4, Unit::Cups);
        assert_eq!(qty.estimated_value(), Some(2.25));
        
        let range_qty = Quantity::range(1.0, 3.0, Unit::Tablespoons);
        assert_eq!(range_qty.estimated_value(), Some(2.0));
        
        let ambiguous_qty = Quantity::ambiguous("some", Unit::Unknown("".to_string()));
        assert_eq!(ambiguous_qty.estimated_value(), None);
    }

    #[test]
    fn test_ingredient_display() {
        let ingredient = Ingredient::new("onions")
            .with_quantity(Quantity::range(2.0, 3.0, Unit::Pieces))
            .with_modifier("diced");
        
        let display = format!("{}", ingredient);
        assert!(display.contains("onions"));
        assert!(display.contains("2-3"));
        assert!(display.contains("diced"));
    }
}