use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, sqlx::Type, ToSchema)]
#[sqlx(type_name = "order_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Pending,
    PaymentInitiated,
    Paid,
    Processing,
    Shipped,
    Delivered,
    Cancelled,
    Refunded,
}

impl OrderStatus {
    pub fn can_transition_to(&self, next: &OrderStatus) -> bool {
        matches!(
            (self, next),
            (OrderStatus::Pending, OrderStatus::PaymentInitiated)
            | (OrderStatus::PaymentInitiated, OrderStatus::Paid)
            | (OrderStatus::Paid, OrderStatus::Processing)
            | (OrderStatus::Processing, OrderStatus::Shipped)
            | (OrderStatus::Shipped, OrderStatus::Delivered)
            | (OrderStatus::Pending, OrderStatus::Cancelled)
            | (OrderStatus::PaymentInitiated, OrderStatus::Cancelled)
            | (OrderStatus::Paid, OrderStatus::Refunded)
        )
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub total_amount: i32,
    pub status: OrderStatus,
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

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
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
            status: serde_json::to_value(&order.status)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_else(|| format!("{:?}", order.status).to_lowercase()),
            payment_reference: order.payment_reference,
            created_at: order.created_at,
            items,
        }
    }
}