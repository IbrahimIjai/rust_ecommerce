use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub total_amount: i32, // Total amount in cents/kobo
    pub status: String, // "pending", "paid", "shipped", "delivered", "cancelled"
    pub payment_reference: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct OrderItem {
    pub id: Uuid,
    pub order_id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub product_price: i32,
    pub quantity: i32,
    pub subtotal: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrder {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub total_amount: i32,
    pub total_amount_formatted: String,
    pub status: String,
    pub payment_reference: Option<String>,
    pub created_at: DateTime<Utc>,
    pub items: Vec<OrderItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct OrderItemResponse {
    pub product_id: Uuid,
    pub product_name: String,
    pub product_price: i32,
    pub product_price_formatted: String,
    pub quantity: i32,
    pub subtotal: i32,
    pub subtotal_formatted: String,
}

impl OrderItemResponse {
    pub fn new(
        product_id: Uuid,
        product_name: String,
        product_price: i32,
        quantity: i32,
    ) -> Self {
        let subtotal = product_price * quantity;
        let product_price_formatted = format!("₦{:.2}", product_price as f64 / 100.0);
        let subtotal_formatted = format!("₦{:.2}", subtotal as f64 / 100.0);
        
        Self {
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

impl OrderResponse {
    pub fn new(
        order: Order,
        items: Vec<OrderItemResponse>,
    ) -> Self {
        let total_amount_formatted = format!("₦{:.2}", order.total_amount as f64 / 100.0);
        
        Self {
            id: order.id,
            user_id: order.user_id,
            total_amount: order.total_amount,
            total_amount_formatted,
            status: order.status,
            payment_reference: order.payment_reference,
            created_at: order.created_at,
            items,
        }
    }
}