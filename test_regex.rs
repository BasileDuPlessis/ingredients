extern crate regex;

use regex::Regex;

const DEFAULT_PATTERN: &str = r#"(?i)(\d*\.?\d+|\d+/\d+|[½⅓⅔¼¾⅕⅖⅗⅘⅙⅚⅛⅜⅝⅞⅟])(?:\s*(?:cup(?:s)?|teaspoon(?:s)?|tsp(?:\.?)|tablespoon(?:s)?|tbsp(?:\.?)|pint(?:s)?|quart(?:s)?|gallon(?:s)?|oz|ounce(?:s)?|lb(?:\.?)|pound(?:s)?|mg|gram(?:me)?s?|kilogram(?:me)?s?|kg|g|liter(?:s)?|litre(?:s)?|millilitre(?:s)?|ml|cm3|mm3|cm²|mm²|cl|dl|l|slice(?:s)?|can(?:s)?|bottle(?:s)?|stick(?:s)?|packet(?:s)?|pkg|bag(?:s)?|dash(?:es)?|pinch(?:es)?|drop(?:s)?|cube(?:s)?|piece(?:s)?|handful(?:s)?|bar(?:s)?|sheet(?:s)?|serving(?:s)?|portion(?:s)?|tasse(?:s)?|cuil(?:\.?)?(?:\s+à\s+(?:café|soupe))?|cuillère(?:s)?(?:\s+à\s+(?:café|soupe))?|poignée(?:s)?|sachet(?:s)?|paquet(?:s)?|boîte(?:s)?|conserve(?:s)?|tranche(?:s)?|morceau(?:x)?|gousse(?:s)?|brin(?:s)?|feuille(?:s)?|bouquet(?:s)?)|\s+\w+)"#;

fn main() {
    let regex = Regex::new(DEFAULT_PATTERN).unwrap();
    let text = "2 cuillères à soupe de sucre";
    
    println!("Testing regex on: {}", text);
    for capture in regex.find_iter(text) {
        println!("Match: '{}'", capture.as_str());
    }
}
