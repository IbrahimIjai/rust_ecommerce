use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub price: i32,
    pub stock_quantity: i32,
    pub category: Option<String>,
    pub image_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProduct {
    pub name: String,
    pub description: Option<String>,
    pub price: i32,
    pub stock_quantity: Option<i32>,
    pub category: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProduct {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<i32>,
    pub stock_quantity: Option<i32>,
    pub category: Option<String>,
    pub image_url: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ProductFilterParams {
    pub category: Option<String>,
    pub min_price: Option<i32>,
    pub max_price: Option<i32>,
    pub search: Option<String>,
    pub in_stock: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ProductResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub price: i32,
    pub price_formatted: String,
    pub stock_quantity: i32,
    pub category: Option<String>,
    pub image_url: Option<String>,
    pub is_active: bool,
}

impl From<Product> for ProductResponse {
    fn from(p: Product) -> Self {
        let price_formatted = format!("₦{:.2}", p.price as f64 / 100.0);
        Self {
            id: p.id,
            name: p.name,
            description: p.description,
            price: p.price,
            price_formatted,
            stock_quantity: p.stock_quantity,
            category: p.category,
            image_url: p.image_url,
            is_active: p.is_active,
        }
    }
}
