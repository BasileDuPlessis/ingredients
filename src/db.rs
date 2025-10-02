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
    pub recipe_name: Option<String>,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Initialize the database schema
pub async fn init_database_schema(pool: &PgPool) -> Result<()> {
    info!("Initializing database schema");

    // Create users table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id BIGSERIAL PRIMARY KEY,
            telegram_id BIGINT UNIQUE NOT NULL,
            language_code VARCHAR(10) DEFAULT 'en',
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(pool)
    .await
    .context("Failed to create users table")?;

    // Create OCR entries table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS ocr_entries (
            id BIGSERIAL PRIMARY KEY,
            telegram_id BIGINT NOT NULL,
            content TEXT NOT NULL,
            recipe_name VARCHAR(255),
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            content_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english', content)) STORED
        )",
    )
    .execute(pool)
    .await
    .context("Failed to create ocr_entries table")?;

    // Create ingredients table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS ingredients (
            id BIGSERIAL PRIMARY KEY,
            user_id BIGINT NOT NULL REFERENCES users(id),
            ocr_entry_id BIGINT REFERENCES ocr_entries(id),
            name VARCHAR(255) NOT NULL,
            quantity DECIMAL(10,3),
            unit VARCHAR(50),
            raw_text TEXT NOT NULL,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
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
                recipe_name: None, // For backward compatibility, existing entries have no recipe name
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
#[allow(clippy::too_many_arguments)]
pub async fn create_ingredient(
    pool: &PgPool,
    user_id: i64,
    ocr_entry_id: Option<i64>,
    name: &str,
    quantity: Option<f64>,
    unit: Option<&str>,
    raw_text: &str,
) -> Result<i64> {
    info!("Creating new ingredient for user_id: {user_id}");

    let row = sqlx::query(
        "INSERT INTO ingredients (user_id, ocr_entry_id, name, quantity, unit, raw_text) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id"
    )
    .bind(user_id)
    .bind(ocr_entry_id)
    .bind(name)
    .bind(quantity)
    .bind(unit)
    .bind(raw_text)
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

    let row = sqlx::query("SELECT id, user_id, ocr_entry_id, name, quantity, unit, raw_text, created_at, updated_at FROM ingredients WHERE id = $1")
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
                created_at: row.get(7),
                updated_at: row.get(8),
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
) -> Result<bool> {
    info!("Updating ingredient with ID: {ingredient_id}");

    let result = sqlx::query("UPDATE ingredients SET name = COALESCE($1, name), quantity = COALESCE($2, quantity), unit = COALESCE($3, unit), raw_text = $4, updated_at = CURRENT_TIMESTAMP WHERE id = $5")
        .bind(name)
        .bind(quantity)
        .bind(unit)
        .bind(raw_text)
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

    let rows = sqlx::query("SELECT id, user_id, ocr_entry_id, name, quantity, unit, raw_text, created_at, updated_at FROM ingredients WHERE user_id = $1 ORDER BY created_at DESC")
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
            created_at: row.get(7),
            updated_at: row.get(8),
        })
        .collect();

    info!(
        "Found {} ingredients for user_id: {user_id}",
        ingredients.len()
    );
    Ok(ingredients)
}

/// Update the recipe name for an OCR entry
pub async fn update_ocr_entry_recipe_name(
    pool: &PgPool,
    entry_id: i64,
    recipe_name: &str,
) -> Result<bool> {
    debug!(entry_id = %entry_id, "Updating OCR entry recipe name");

    let result = sqlx::query("UPDATE ocr_entries SET recipe_name = $1 WHERE id = $2")
        .bind(recipe_name)
        .bind(entry_id)
        .execute(pool)
        .await
        .context("Failed to update OCR entry recipe name")?;

    let rows_affected = result.rows_affected();
    if rows_affected > 0 {
        debug!(entry_id = %entry_id, "OCR entry recipe name updated successfully");
        Ok(true)
    } else {
        info!("No OCR entry found with ID: {entry_id}");
        Ok(false)
    }
}

/// Get OCR entry with recipe name
pub async fn read_ocr_entry_with_recipe(pool: &PgPool, entry_id: i64) -> Result<Option<OcrEntry>> {
    debug!(entry_id = %entry_id, "Reading OCR entry with recipe name");

    let row = sqlx::query("SELECT id, telegram_id, content, recipe_name, created_at FROM ocr_entries WHERE id = $1")
        .bind(entry_id)
        .fetch_optional(pool)
        .await
        .context("Failed to read OCR entry with recipe")?;

    match row {
        Some(row) => {
            let entry = OcrEntry {
                id: row.get(0),
                telegram_id: row.get(1),
                content: row.get(2),
                recipe_name: row.get(3),
                created_at: row.get(4),
            };
            debug!(entry_id = %entry_id, "OCR entry with recipe found");
            Ok(Some(entry))
        }
        None => {
            debug!(entry_id = %entry_id, "No OCR entry found");
            Ok(None)
        }
    }
}

/// Search OCR entries using full-text search
pub async fn search_ocr_entries(
    pool: &PgPool,
    telegram_id: i64,
    query: &str,
) -> Result<Vec<OcrEntry>> {
    info!("Searching OCR entries for telegram_id: {telegram_id} with query: {query}");

    let rows = sqlx::query("SELECT id, telegram_id, content, recipe_name, created_at FROM ocr_entries WHERE telegram_id = $1 AND content_tsv @@ plainto_tsquery('english', $2) ORDER BY created_at DESC")
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
            recipe_name: row.get(3),
            created_at: row.get(4),
        })
        .collect();

    info!("Found {} OCR entries matching query", entries.len());
    Ok(entries)
}
