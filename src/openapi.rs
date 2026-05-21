use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rust E-Commerce API",
        version = "0.1.0",
        description = "Production-ready e-commerce REST API built with Axum and PostgreSQL. \
                       Handles auth, product catalog, cart management, order lifecycle, \
                       and Paystack payment processing.",
        contact(name = "API Support"),
        license(name = "MIT"),
    ),
    paths(
        crate::handlers::health::health_check,
        crate::handlers::auth::signup,
        crate::handlers::auth::login,
        crate::handlers::auth::refresh_token,
        crate::handlers::auth::me,
        crate::handlers::auth::forgot_password,
        crate::handlers::auth::reset_password,
        crate::handlers::user::get_users,
        crate::handlers::user::create_user,
        crate::handlers::user::get_user,
        crate::handlers::user::delete_user,
        crate::handlers::product::get_products,
        crate::handlers::product::get_product,
        crate::handlers::product::create_product,
        crate::handlers::product::update_product,
        crate::handlers::product::delete_product,
        crate::handlers::cart::get_cart,
        crate::handlers::cart::add_to_cart,
        crate::handlers::cart::update_cart_item,
        crate::handlers::cart::remove_from_cart,
        crate::handlers::order::create_order,
        crate::handlers::order::get_orders,
        crate::handlers::order::get_user_orders,
        crate::handlers::order::get_order,
        crate::handlers::payment::initialize_payment,
        crate::handlers::payment::verify_payment,
        crate::handlers::webhook::paystack_webhook,
    ),
    components(
        schemas(
            crate::models::CreateUser,
            crate::models::SignupRequest,
            crate::models::LoginRequest,
            crate::models::RefreshRequest,
            crate::models::ForgotPasswordRequest,
            crate::models::ResetPasswordRequest,
            crate::models::AuthResponse,
            crate::models::UserResponse,
            crate::auth::Role,
            crate::models::CreateProduct,
            crate::models::UpdateProduct,
            crate::models::ProductResponse,
            crate::models::AddToCart,
            crate::models::UpdateCartItem,
            crate::models::CartItemResponse,
            crate::models::CartResponse,
            crate::models::OrderStatus,
            crate::models::OrderResponse,
            crate::models::OrderItemResponse,
            crate::handlers::payment::InitializePaymentRequest,
            crate::handlers::payment::VerifyPaymentRequest,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health",   description = "Service readiness and database health"),
        (name = "Auth",     description = "Registration, login, JWT refresh, password reset"),
        (name = "Users",    description = "User profile and admin user management"),
        (name = "Products", description = "Public catalog + admin CRUD"),
        (name = "Cart",     description = "Cart read, add, update, and remove operations"),
        (name = "Orders",   description = "Cart → order conversion, order history"),
        (name = "Payment",  description = "Paystack initialize, verify, and webhook handling"),
    )
)]
pub struct ApiDoc;
