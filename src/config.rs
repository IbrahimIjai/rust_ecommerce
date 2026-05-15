use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: String,
    pub jwt_signing_key: String,
    pub paystack_secret_key: String,
    pub paystack_mock: bool,
    pub allowed_origins: String,
    pub rust_log: String,
}

impl Config {
    pub fn from_env() -> Self {
        let paystack_secret_key = env::var("PAYSTACK_SECRET_KEY")
            .expect("PAYSTACK_SECRET_KEY must be set");

        let mock_env = env::var("PAYSTACK_MOCK")
            .unwrap_or_else(|_| "false".to_string())
            .to_ascii_lowercase();
        let paystack_mock = matches!(mock_env.as_str(), "1" | "true" | "yes")
            || paystack_secret_key == "sk_test_placeholder";

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT").unwrap_or_else(|_| "3000".to_string()),
            jwt_signing_key: env::var("JWT_SIGNING_KEY")
                .expect("JWT_SIGNING_KEY must be set"),
            paystack_secret_key,
            paystack_mock,
            allowed_origins: env::var("ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            rust_log: env::var("RUST_LOG")
                .unwrap_or_else(|_| "rust_ecommerce=debug,tower_http=debug".to_string()),
        }
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
