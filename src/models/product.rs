use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price: i32, // Price in cents/kobo
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProduct {
    pub name: String,
    pub price: i32, // Price in cents/kobo
}

#[derive(Debug, Serialize)]
pub struct ProductResponse {
    pub id: Uuid,
    pub name: String,
    pub price: i32, // Price in cents/kobo
    pub price_formatted: String, // Formatted price (e.g., "$10.99")
}

impl From<Product> for ProductResponse {
    fn from(product: Product) -> Self {
        let price_formatted = format!("₦{:.2}", product.price as f64 / 100.0);
        Self {
            id: product.id,
            name: product.name,
            price: product.price,
            price_formatted,
        }
    }
}