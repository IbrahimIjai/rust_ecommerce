use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

#[derive(Clone)]

#[derive(Debug, Serialize)]
pub struct InitializePaymentRequest {
    pub email: String,
    pub amount: i32, // Amount in kobo (cents)
    pub reference: String,
    pub callback_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InitializePaymentResponse {
    pub status: bool,
    pub message: String,
    pub data: PaymentData,
}

#[derive(Debug, Deserialize)]
pub struct PaymentData {
    pub authorization_url: String,
    pub access_code: String,
    pub reference: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyPaymentResponse {
    pub status: bool,
    pub message: String,
    pub data: VerifyPaymentData,
}

#[derive(Debug, Deserialize)]
pub struct VerifyPaymentData {
    pub id: i32,
    pub domain: String,
    pub status: String,
    pub reference: String,
    pub amount: i32,
    pub paid_at: Option<String>,
    pub created_at: String,
    pub channel: String,
    pub currency: String,
    pub customer: CustomerData,
}

#[derive(Debug, Deserialize)]
pub struct CustomerData {
    pub id: i32,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: String,
    pub customer_code: String,
}

#[derive(Clone)]
pub struct PaystackService {
    client: Client,
    secret_key: String,
    base_url: String,
    mock_mode: bool,
}

impl PaystackService {
    pub fn new() -> Self {
        let secret_key = env::var("PAYSTACK_SECRET_KEY")
            .expect("PAYSTACK_SECRET_KEY must be set");
        let mock_mode_env = env::var("PAYSTACK_MOCK")
            .unwrap_or_else(|_| "false".to_string())
            .to_ascii_lowercase();
        let mock_mode = mock_mode_env == "1"
            || mock_mode_env == "true"
            || mock_mode_env == "yes"
            || secret_key == "sk_test_placeholder";
        
        Self {
            client: Client::new(),
            secret_key,
            base_url: "https://api.paystack.co".to_string(),
            mock_mode,
        }
    }
    
    pub async fn initialize_payment(
        &self,
        email: &str,
        amount: i32,
        reference: &str,
    ) -> Result<InitializePaymentResponse, reqwest::Error> {
        let url = format!("{}/transaction/initialize", self.base_url);
        
        let request_body = InitializePaymentRequest {
            email: email.to_string(),
            amount,
            reference: reference.to_string(),
            callback_url: None,
        };
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;
        
        response.json::<InitializePaymentResponse>().await
    }
    
    pub async fn verify_payment(
        &self,
        reference: &str,
    ) -> Result<VerifyPaymentResponse, reqwest::Error> {
        let url = format!("{}/transaction/verify/{}", self.base_url, reference);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .send()
            .await?;
        
        response.json::<VerifyPaymentResponse>().await
    }
    
    pub fn generate_reference() -> String {
        format!("ECOM_{}_{}", Uuid::new_v4().to_string().replace("-", "")[..8].to_uppercase(), chrono::Utc::now().timestamp())
    }

    pub fn is_mock_mode(&self) -> bool {
        self.mock_mode
    }
}

impl Default for PaystackService {
    fn default() -> Self {
        Self::new()
    }
}
