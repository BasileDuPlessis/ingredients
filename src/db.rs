use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPool;
use sqlx::Row;
use tracing::{debug, info};

/// Represents a user in the database
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: i64,
    pub telegram_id: i64,
    pub language_code: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents an OCR entry in the database
#[derive(Debug, Clone, PartialEq)]
pub struct OcrEntry {
    pub id: i64,
    pub telegram_id: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// Represents an ingredient in the database
#[derive(Debug, Clone, PartialEq)]
pub struct Ingredient {
    pub id: i64,
    pub user_id: i64,
    pub ocr_entry_id: Option<i64>,
    pub name: String,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub raw_text: String,
    pub recipe_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Initialize the database schema
pub async fn init_database_schema(pool: &PgPool) -> Result<()> {
    info!("Initializing database schema");

    // Create users table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            telegram_id BIGINT UNIQUE NOT NULL,
            language_code VARCHAR(10) DEFAULT 'en',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(pool)
    .await
    .context("Failed to create users table")?;

    // Create OCR entries table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS ocr_entries (
            id SERIAL PRIMARY KEY,
            telegram_id BIGINT NOT NULL,
            content TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english', content)) STORED
        )",
    )
    .execute(pool)
    .await
    .context("Failed to create ocr_entries table")?;

    // Create ingredients table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS ingredients (
            id SERIAL PRIMARY KEY,
            user_id BIGINT NOT NULL REFERENCES users(id),
            ocr_entry_id BIGINT REFERENCES ocr_entries(id),
            name VARCHAR(255) NOT NULL,
            quantity DECIMAL(10,3),
            unit VARCHAR(50),
            raw_text TEXT NOT NULL,
            recipe_name VARCHAR(255),
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (ocr_entry_id) REFERENCES ocr_entries(id)
        )",
    )
    .execute(pool)
    .await
    .context("Failed to create ingredients table")?;

    // Create indexes for performance
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS ocr_entries_content_tsv_idx ON ocr_entries USING GIN (content_tsv)",
    )
    .execute(pool)
    .await
    .context("Failed to create FTS index")?;

    sqlx::query("CREATE INDEX IF NOT EXISTS ingredients_user_id_idx ON ingredients(user_id)")
        .execute(pool)
        .await
        .context("Failed to create ingredients user_id index")?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS ingredients_ocr_entry_id_idx ON ingredients(ocr_entry_id)",
    )
    .execute(pool)
    .await
    .context("Failed to create ingredients ocr_entry_id index")?;

    info!("Database schema initialized successfully");
    Ok(())
}

/// Create a new OCR entry in the database
pub async fn create_ocr_entry(pool: &PgPool, telegram_id: i64, content: &str) -> Result<i64> {
    debug!(telegram_id = %telegram_id, "Creating new OCR entry");

    let row =
        sqlx::query("INSERT INTO ocr_entries (telegram_id, content) VALUES ($1, $2) RETURNING id")
            .bind(telegram_id)
            .bind(content)
            .fetch_one(pool)
            .await
            .context("Failed to insert new OCR entry")?;

    let entry_id: i64 = row.get(0);
    debug!(entry_id = %entry_id, "OCR entry created successfully");

    Ok(entry_id)
}

/// Read an OCR entry from the database by ID
pub async fn read_ocr_entry(pool: &PgPool, entry_id: i64) -> Result<Option<OcrEntry>> {
    debug!(entry_id = %entry_id, "Reading OCR entry");

    let row =
        sqlx::query("SELECT id, telegram_id, content, created_at FROM ocr_entries WHERE id = $1")
            .bind(entry_id)
            .fetch_optional(pool)
            .await
            .context("Failed to read OCR entry")?;

    match row {
        Some(row) => {
            let entry = OcrEntry {
                id: row.get(0),
                telegram_id: row.get(1),
                content: row.get(2),
                created_at: row.get(3),
            };
            debug!(entry_id = %entry_id, "OCR entry found");
            Ok(Some(entry))
        }
        None => {
            debug!(entry_id = %entry_id, "No OCR entry found");
            Ok(None)
        }
    }
}

/// Update an existing OCR entry in the database
pub async fn update_ocr_entry(pool: &PgPool, entry_id: i64, new_content: &str) -> Result<bool> {
    debug!(entry_id = %entry_id, "Updating OCR entry");

    let result = sqlx::query("UPDATE ocr_entries SET content = $1 WHERE id = $2")
        .bind(new_content)
        .bind(entry_id)
        .execute(pool)
        .await
        .context("Failed to update OCR entry")?;

    let rows_affected = result.rows_affected();
    if rows_affected > 0 {
        debug!(entry_id = %entry_id, "OCR entry updated successfully");
        Ok(true)
    } else {
        info!("No OCR entry found with ID: {entry_id}");
        Ok(false)
    }
}

/// Delete an OCR entry from the database
pub async fn delete_ocr_entry(pool: &PgPool, entry_id: i64) -> Result<bool> {
    debug!(entry_id = %entry_id, "Deleting OCR entry");

    let result = sqlx::query("DELETE FROM ocr_entries WHERE id = $1")
        .bind(entry_id)
        .execute(pool)
        .await
        .context("Failed to delete OCR entry")?;

    let rows_affected = result.rows_affected();
    if rows_affected > 0 {
        debug!(entry_id = %entry_id, "OCR entry deleted successfully");
        Ok(true)
    } else {
        info!("No OCR entry found with ID: {entry_id}");
        Ok(false)
    }
}

/// Get or create a user by Telegram ID
pub async fn get_or_create_user(
    pool: &PgPool,
    telegram_id: i64,
    language_code: Option<&str>,
) -> Result<User> {
    debug!(telegram_id = %telegram_id, "Getting or creating user");

    // Try to get existing user
    if let Some(user) = get_user_by_telegram_id(pool, telegram_id).await? {
        return Ok(user);
    }

    // Create new user
    let language_code = language_code.unwrap_or("en");
    let row = sqlx::query(
        "INSERT INTO users (telegram_id, language_code) VALUES ($1, $2) RETURNING id, telegram_id, language_code, created_at, updated_at"
    )
    .bind(telegram_id)
    .bind(language_code)
    .fetch_one(pool)
    .await
    .context("Failed to create new user")?;

    let user = User {
        id: row.get(0),
        telegram_id: row.get(1),
        language_code: row.get(2),
        created_at: row.get(3),
        updated_at: row.get(4),
    };

    debug!(user_id = %user.id, "User created successfully");
    Ok(user)
}

/// Get a user by Telegram ID
pub async fn get_user_by_telegram_id(pool: &PgPool, telegram_id: i64) -> Result<Option<User>> {
    debug!(telegram_id = %telegram_id, "Getting user by telegram_id");

    let row = sqlx::query("SELECT id, telegram_id, language_code, created_at, updated_at FROM users WHERE telegram_id = $1")
        .bind(telegram_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get user by telegram_id")?;

    match row {
        Some(row) => {
            let user = User {
                id: row.get(0),
                telegram_id: row.get(1),
                language_code: row.get(2),
                created_at: row.get(3),
                updated_at: row.get(4),
            };
            info!("User found with ID: {}", user.id);
            Ok(Some(user))
        }
        None => {
            info!("No user found with telegram_id: {telegram_id}");
            Ok(None)
        }
    }
}

/// Get a user by internal ID
pub async fn get_user_by_id(pool: &PgPool, user_id: i64) -> Result<Option<User>> {
    info!("Getting user by ID: {user_id}");

    let row = sqlx::query(
        "SELECT id, telegram_id, language_code, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get user by ID")?;

    match row {
        Some(row) => {
            let user = User {
                id: row.get(0),
                telegram_id: row.get(1),
                language_code: row.get(2),
                created_at: row.get(3),
                updated_at: row.get(4),
            };
            info!("User found with ID: {}", user.id);
            Ok(Some(user))
        }
        None => {
            info!("No user found with ID: {user_id}");
            Ok(None)
        }
    }
}

/// Create a new ingredient in the database
pub async fn create_ingredient(
    pool: &PgPool,
    user_id: i64,
    ocr_entry_id: Option<i64>,
    name: &str,
    quantity: Option<f64>,
    unit: Option<&str>,
    raw_text: &str,
    recipe_name: Option<&str>,
) -> Result<i64> {
    info!("Creating new ingredient for user_id: {user_id}");

    let row = sqlx::query(
        "INSERT INTO ingredients (user_id, ocr_entry_id, name, quantity, unit, raw_text, recipe_name) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id"
    )
    .bind(user_id)
    .bind(ocr_entry_id)
    .bind(name)
    .bind(quantity)
    .bind(unit)
    .bind(raw_text)
    .bind(recipe_name)
    .fetch_one(pool)
    .await
    .context("Failed to insert new ingredient")?;

    let ingredient_id: i64 = row.get(0);
    info!("Ingredient created with ID: {ingredient_id}");

    Ok(ingredient_id)
}

/// Read an ingredient from the database by ID
pub async fn read_ingredient(pool: &PgPool, ingredient_id: i64) -> Result<Option<Ingredient>> {
    info!("Reading ingredient with ID: {ingredient_id}");

    let row = sqlx::query("SELECT id, user_id, ocr_entry_id, name, quantity, unit, raw_text, recipe_name, created_at, updated_at FROM ingredients WHERE id = $1")
        .bind(ingredient_id)
        .fetch_optional(pool)
        .await
        .context("Failed to read ingredient")?;

    match row {
        Some(row) => {
            let ingredient = Ingredient {
                id: row.get(0),
                user_id: row.get(1),
                ocr_entry_id: row.get(2),
                name: row.get(3),
                quantity: row.get(4),
                unit: row.get(5),
                raw_text: row.get(6),
                recipe_name: row.get(7),
                created_at: row.get(8),
                updated_at: row.get(9),
            };
            info!("Ingredient found with ID: {ingredient_id}");
            Ok(Some(ingredient))
        }
        None => {
            info!("No ingredient found with ID: {ingredient_id}");
            Ok(None)
        }
    }
}

/// Update an existing ingredient in the database
pub async fn update_ingredient(
    pool: &PgPool,
    ingredient_id: i64,
    name: Option<&str>,
    quantity: Option<f64>,
    unit: Option<&str>,
    raw_text: &str,
    recipe_name: Option<&str>,
) -> Result<bool> {
    info!("Updating ingredient with ID: {ingredient_id}");

    let result = sqlx::query("UPDATE ingredients SET name = COALESCE($1, name), quantity = COALESCE($2, quantity), unit = COALESCE($3, unit), raw_text = $4, recipe_name = COALESCE($5, recipe_name), updated_at = CURRENT_TIMESTAMP WHERE id = $6")
        .bind(name)
        .bind(quantity)
        .bind(unit)
        .bind(raw_text)
        .bind(recipe_name)
        .bind(ingredient_id)
        .execute(pool)
        .await
        .context("Failed to update ingredient")?;

    let rows_affected = result.rows_affected();
    if rows_affected > 0 {
        info!("Ingredient updated successfully with ID: {ingredient_id}");
        Ok(true)
    } else {
        info!("No ingredient found with ID: {ingredient_id}");
        Ok(false)
    }
}

/// Delete an ingredient from the database
pub async fn delete_ingredient(pool: &PgPool, ingredient_id: i64) -> Result<bool> {
    info!("Deleting ingredient with ID: {ingredient_id}");

    let result = sqlx::query("DELETE FROM ingredients WHERE id = $1")
        .bind(ingredient_id)
        .execute(pool)
        .await
        .context("Failed to delete ingredient")?;

    let rows_affected = result.rows_affected();
    if rows_affected > 0 {
        info!("Ingredient deleted successfully with ID: {ingredient_id}");
        Ok(true)
    } else {
        info!("No ingredient found with ID: {ingredient_id}");
        Ok(false)
    }
}

/// List all ingredients for a user
pub async fn list_ingredients_by_user(pool: &PgPool, user_id: i64) -> Result<Vec<Ingredient>> {
    info!("Listing ingredients for user_id: {user_id}");

    let rows = sqlx::query("SELECT id, user_id, ocr_entry_id, name, quantity, unit, raw_text, recipe_name, created_at, updated_at FROM ingredients WHERE user_id = $1 ORDER BY created_at DESC")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .context("Failed to list ingredients by user")?;

    let ingredients: Vec<Ingredient> = rows
        .into_iter()
        .map(|row| Ingredient {
            id: row.get(0),
            user_id: row.get(1),
            ocr_entry_id: row.get(2),
            name: row.get(3),
            quantity: row.get(4),
            unit: row.get(5),
            raw_text: row.get(6),
            recipe_name: row.get(7),
            created_at: row.get(8),
            updated_at: row.get(9),
        })
        .collect();

    info!(
        "Found {} ingredients for user_id: {user_id}",
        ingredients.len()
    );
    Ok(ingredients)
}

/// Search OCR entries using full-text search
pub async fn search_ocr_entries(
    pool: &PgPool,
    telegram_id: i64,
    query: &str,
) -> Result<Vec<OcrEntry>> {
    info!("Searching OCR entries for telegram_id: {telegram_id} with query: {query}");

    let rows = sqlx::query("SELECT id, telegram_id, content, created_at FROM ocr_entries WHERE telegram_id = $1 AND content_tsv @@ plainto_tsquery('english', $2) ORDER BY created_at DESC")
        .bind(telegram_id)
        .bind(query)
        .fetch_all(pool)
        .await
        .context("Failed to search OCR entries")?;

    let entries: Vec<OcrEntry> = rows
        .into_iter()
        .map(|row| OcrEntry {
            id: row.get(0),
            telegram_id: row.get(1),
            content: row.get(2),
            created_at: row.get(3),
        })
        .collect();

    info!("Found {} OCR entries matching query", entries.len());
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    async fn setup_test_db() -> Result<PgPool> {
        // Skip tests if no DATABASE_URL is provided
        let database_url = match env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => {
                eprintln!("Skipping database tests: DATABASE_URL not set");
                return Err(anyhow::anyhow!("Test database not configured"));
            }
        };

        let pool = PgPool::connect(&database_url)
            .await
            .context("Failed to connect to test database")?;

        // Clean up any existing test data
        sqlx::query("DROP TABLE IF EXISTS ingredients CASCADE")
            .execute(&pool)
            .await?;
        sqlx::query("DROP TABLE IF EXISTS ocr_entries CASCADE")
            .execute(&pool)
            .await?;
        sqlx::query("DROP TABLE IF EXISTS users CASCADE")
            .execute(&pool)
            .await?;

        // Initialize schema
        init_database_schema(&pool).await?;

        Ok(pool)
    }

    // Helper macro to skip tests when database is not available
    macro_rules! skip_if_no_db {
        ($test_fn:expr) => {
            match setup_test_db().await {
                Ok(pool) => $test_fn(&pool).await,
                Err(_) => {
                    eprintln!("Skipping test: Database not available");
                    Ok(())
                }
            }
        };
    }

    #[tokio::test]
    async fn test_user_operations() -> Result<()> {
        skip_if_no_db!(test_user_operations_impl)
    }

    async fn test_user_operations_impl(pool: &PgPool) -> Result<()> {
        let user = get_or_create_user(pool, 12345, Some("fr")).await?;
        assert_eq!(user.telegram_id, 12345);
        assert_eq!(user.language_code, "fr");

        // Test getting existing user
        let user2 = get_or_create_user(pool, 12345, Some("en")).await?;
        assert_eq!(user2.id, user.id); // Should return same user
        assert_eq!(user2.language_code, "fr"); // Should keep original language

        // Test get_user_by_telegram_id
        let found_user = get_user_by_telegram_id(pool, 12345).await?;
        assert_eq!(found_user, Some(user.clone()));

        // Test get_user_by_id
        let found_user_by_id = get_user_by_id(pool, user.id).await?;
        assert_eq!(found_user_by_id, Some(user));

        Ok(())
    }

    #[tokio::test]
    async fn test_ocr_entry_operations() -> Result<()> {
        skip_if_no_db!(test_ocr_entry_operations_impl)
    }

    async fn test_ocr_entry_operations_impl(pool: &PgPool) -> Result<()> {
        let entry_id = create_ocr_entry(pool, 12345, "Test OCR content").await?;
        assert!(entry_id > 0);

        // Read OCR entry
        let entry = read_ocr_entry(pool, entry_id).await?;
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.telegram_id, 12345);
        assert_eq!(entry.content, "Test OCR content");

        // Update OCR entry
        let updated = update_ocr_entry(pool, entry_id, "Updated content").await?;
        assert!(updated);

        let updated_entry = read_ocr_entry(pool, entry_id).await?;
        assert_eq!(updated_entry.unwrap().content, "Updated content");

        // Delete OCR entry
        let deleted = delete_ocr_entry(pool, entry_id).await?;
        assert!(deleted);

        let not_found = read_ocr_entry(pool, entry_id).await?;
        assert!(not_found.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_ingredient_operations() -> Result<()> {
        skip_if_no_db!(test_ingredient_operations_impl)
    }

    async fn test_ingredient_operations_impl(pool: &PgPool) -> Result<()> {
        let user = get_or_create_user(pool, 12345, None).await?;

        // Create OCR entry
        let ocr_entry_id = create_ocr_entry(pool, 12345, "flour 2 cups").await?;

        // Create ingredient
        let ingredient_id = create_ingredient(
            pool,
            user.id,
            Some(ocr_entry_id),
            "flour",
            Some(2.0),
            Some("cups"),
            "flour 2 cups",
            Some("Test Recipe"),
        )
        .await?;
        assert!(ingredient_id > 0);

        // Read ingredient
        let ingredient = read_ingredient(pool, ingredient_id).await?;
        assert!(ingredient.is_some());
        let ingredient = ingredient.unwrap();
        assert_eq!(ingredient.user_id, user.id);
        assert_eq!(ingredient.ocr_entry_id, Some(ocr_entry_id));
        assert_eq!(ingredient.name, "flour");
        assert_eq!(ingredient.quantity, Some(2.0));
        assert_eq!(ingredient.unit, Some("cups".to_string()));

        // Update ingredient
        let updated = update_ingredient(
            pool,
            ingredient_id,
            Some("bread flour"),
            Some(3.0),
            Some("cups"),
            "bread flour 3 cups",
            Some("Updated Test Recipe"),
        )
        .await?;
        assert!(updated);

        let updated_ingredient = read_ingredient(pool, ingredient_id).await?;
        assert_eq!(updated_ingredient.unwrap().name, "bread flour");

        // List ingredients by user
        let ingredients = list_ingredients_by_user(pool, user.id).await?;
        assert_eq!(ingredients.len(), 1);
        assert_eq!(ingredients[0].name, "bread flour");

        // Delete ingredient
        let deleted = delete_ingredient(pool, ingredient_id).await?;
        assert!(deleted);

        let not_found = read_ingredient(pool, ingredient_id).await?;
        assert!(not_found.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_full_text_search() -> Result<()> {
        skip_if_no_db!(test_full_text_search_impl)
    }

    async fn test_full_text_search_impl(pool: &PgPool) -> Result<()> {
        create_ocr_entry(pool, 12345, "flour 2 cups sugar 1 cup").await?;
        create_ocr_entry(pool, 12345, "butter 100 grams milk 250 ml").await?;
        create_ocr_entry(pool, 67890, "chocolate 200 grams").await?;

        // Search for entries containing "flour"
        let results = search_ocr_entries(pool, 12345, "flour").await?;
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("flour"));

        // Search for entries containing "grams"
        let results = search_ocr_entries(pool, 12345, "grams").await?;
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("butter"));

        // Search for non-existent term
        let results = search_ocr_entries(pool, 12345, "nonexistent").await?;
        assert_eq!(results.len(), 0);

        Ok(())
    }
}
