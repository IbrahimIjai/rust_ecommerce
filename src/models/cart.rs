use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CartItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub product_id: Uuid,
    pub quantity: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AddToCart {
    pub product_id: Uuid,
    pub quantity: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCartItem {
    pub quantity: i32,
}

#[derive(Debug, Serialize)]
pub struct CartItemResponse {
    pub id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub product_price: i32,
    pub product_price_formatted: String,
    pub quantity: i32,
    pub subtotal: i32,
    pub subtotal_formatted: String,
}

impl CartItemResponse {
    pub fn new(
        id: Uuid,
        product_id: Uuid,
        product_name: String,
        product_price: i32,
        quantity: i32,
    ) -> Self {
        let subtotal = product_price * quantity;
        let product_price_formatted = format!("₦{:.2}", product_price as f64 / 100.0);
        let subtotal_formatted = format!("₦{:.2}", subtotal as f64 / 100.0);

        Self {
            id,
            product_id,
            product_name,
            product_price,
            product_price_formatted,
            quantity,
            subtotal,
            subtotal_formatted,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CartResponse {
    pub items: Vec<CartItemResponse>,
    pub total_items: i32,
    pub total_amount: i32,
    pub total_amount_formatted: String,
}

impl CartResponse {
    pub fn new(items: Vec<CartItemResponse>) -> Self {
        let total_items = items.iter().map(|item| item.quantity).sum();
        let total_amount = items.iter().map(|item| item.subtotal).sum();
        let total_amount_formatted = format!("${:.2}", total_amount as f64 / 100.0);

        Self {
            items,
            total_items,
            total_amount,
            total_amount_formatted,
        }
    }
}
