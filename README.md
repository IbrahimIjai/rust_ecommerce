# Rust E-Commerce API

A production-ready e-commerce REST API built entirely in Rust. Handles auth, product catalog, cart management, order lifecycle, and payment processing via Paystack — with a full integration test suite.

---

## Motivation

Most e-commerce backends are built in Node or Python. This project exists to prove that Rust is a practical choice for backend APIs — not just systems programming. The goals were:

- Zero runtime crashes from null pointer / type errors (the type system enforces correctness at compile time)
- Single binary deployment — no runtime, no interpreter, no virtual environment
- Real-world features: JWT auth, payment webhooks, idempotency, role-based access control, stock management without oversell
- A codebase that can be handed to a frontend team and just works

---

## Demo

> 📹 **Demo video coming soon** — will walk through signup → add to cart → checkout → Paystack payment → webhook confirmation

---

## Tech Stack

| Layer | Choice | Why |
|---|---|---|
| Framework | [Axum 0.7](https://github.com/tokio-rs/axum) | Tower-native, ergonomic extractors, async-first |
| Database | PostgreSQL via [SQLx 0.8](https://github.com/launchbadge/sqlx) | Compile-time checked queries, async, no ORM overhead |
| Auth | [jsonwebtoken](https://crates.io/crates/jsonwebtoken) + [bcrypt](https://crates.io/crates/bcrypt) | HS256 JWTs, bcrypt password hashing at cost 12 |
| Payment | [Paystack](https://paystack.com) | Initialize → verify flow + HMAC-SHA512 webhook verification |
| Validation | [validator](https://crates.io/crates/validator) | Derive-based struct validation with custom error responses |
| Middleware | [tower-http](https://crates.io/crates/tower-http) | Tracing, CORS, request IDs |
| Testing | [sqlx::test](https://docs.rs/sqlx/latest/sqlx/attr.test.html) + [tower](https://crates.io/crates/tower) | Each test gets an isolated DB; router tested without a live server |
| Runtime | [Tokio](https://tokio.rs) | Multi-threaded async runtime |

---

## Folder Structure

```
rust_ecommerce/
├── src/
│   ├── main.rs                  # Binary entry point — boots server, runs migrations
│   ├── lib.rs                   # Library root — exports build_app() for tests
│   │
│   ├── auth/
│   │   ├── claims.rs            # JWT Claims, Role enum, token generation helpers
│   │   ├── keys.rs              # JwtKeys (encoding + decoding key pair)
│   │   └── mod.rs
│   │
│   ├── config.rs                # Reads all env vars once at startup → Arc<Config>
│   ├── error.rs                 # Centralized AppError enum → JSON error responses
│   │
│   ├── extractors/
│   │   ├── validated_json.rs    # ValidatedJson<T>: JSON + validator::Validate in one extractor
│   │   └── pagination.rs        # PaginationParams + PaginatedResponse<T>
│   │
│   ├── handlers/
│   │   ├── auth.rs              # signup, login, refresh, forgot/reset password, me
│   │   ├── cart.rs              # add, view, update, remove cart items
│   │   ├── health.rs            # health check with DB latency
│   │   ├── order.rs             # create order, list orders (admin/user)
│   │   ├── payment.rs           # initialize + verify payment (Paystack)
│   │   ├── product.rs           # CRUD products (admin), public listing
│   │   ├── user.rs              # list/get/delete users
│   │   └── webhook.rs           # Paystack webhook — HMAC verify + idempotency
│   │
│   ├── models/
│   │   ├── cart.rs              # CartItem, CartResponse
│   │   ├── order.rs             # Order, OrderStatus enum, OrderResponse
│   │   ├── product.rs           # Product, CreateProduct, UpdateProduct, filters
│   │   └── user.rs              # User, SignupRequest, LoginRequest, AuthResponse
│   │
│   ├── routes/
│   │   ├── auth.rs              # /api/auth/*
│   │   ├── cart.rs              # /api/cart/*
│   │   ├── order.rs             # /api/orders/*
│   │   ├── payment.rs           # /api/payment/*
│   │   ├── product.rs           # /api/products/*
│   │   ├── user.rs              # /api/users/*
│   │   └── mod.rs               # Assembles all route groups
│   │
│   └── services/
│       ├── database.rs          # PgPool setup, migration runner
│       ├── payment.rs           # PaystackService — HTTP client wrapper
│       └── mod.rs               # AppState + FromRef impls for sub-state extraction
│
├── migrations/                  # SQLx migrations (run automatically on startup)
│   ├── *_init_schema.sql        # Base tables: users, products, cart_items, orders
│   ├── *_add_auth_to_users.sql  # password_hash, role (ENUM), is_active, reset tokens
│   ├── *_enhance_products.sql   # description, stock_quantity, category, image_url
│   ├── *_order_status_enum.sql  # order_status PostgreSQL ENUM type
│   └── *_webhook_events.sql     # Idempotency table for Paystack webhooks
│
├── tests/
│   ├── helpers/mod.rs           # test_app(), create_test_user(), request() helper
│   ├── auth_test.rs             # 9 tests: signup, login, JWT protection
│   ├── product_test.rs          # 5 tests: CRUD, role enforcement, soft delete
│   ├── order_test.rs            # 5 tests: cart→order, stock decrement, access control
│   └── payment_test.rs          # 5 tests: mock flow, HMAC webhook, idempotency
│
├── docker-compose.yml           # PostgreSQL 16 for local development
├── .env.example                 # All required env vars with descriptions
└── Cargo.toml
```

---

## API Overview

### Auth — `/api/auth`
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/signup` | — | Register, returns access + refresh tokens |
| POST | `/login` | — | Login, returns access + refresh tokens |
| POST | `/refresh` | — | Exchange refresh token for new access token |
| POST | `/forgot-password` | — | Sends reset link (always 200 — no email enumeration) |
| POST | `/reset-password` | — | Set new password with reset token |
| GET | `/me` | JWT | Current user profile |

### Products — `/api/products`
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/` | — | List active products (filter by category, price, search) |
| GET | `/:id` | — | Single product |
| POST | `/` | Admin | Create product |
| PUT | `/:id` | Admin | Update product (partial) |
| DELETE | `/:id` | Admin | Soft delete (sets `is_active = false`) |

### Cart — `/api/cart`
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/:user_id` | JWT (owner) | View cart |
| POST | `/:user_id` | JWT (owner) | Add item |
| PUT | `/:user_id/:product_id` | JWT (owner) | Update quantity |
| DELETE | `/:user_id/:product_id` | JWT (owner) | Remove item |

### Orders — `/api/orders`
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/` | JWT | Create order from cart, decrements stock atomically |
| GET | `/` | Admin | All orders |
| GET | `/user/:user_id` | JWT (owner/admin) | Orders for a user |
| GET | `/:id` | JWT (owner/admin) | Single order |

### Payment — `/api/payment`
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/initialize` | JWT | Initialize Paystack payment, returns checkout URL |
| POST | `/verify` | JWT | Verify payment and mark order paid |
| POST | `/webhook` | — (Paystack) | HMAC-SHA512 verified, idempotent event processing |

---

## Running Locally

**Prerequisites:** Docker, Rust (1.75+)

```bash
# 1. Clone and enter the project
git clone <repo-url>
cd rust_ecommerce

# 2. Copy env and fill in your values
cp .env.example .env

# 3. Start PostgreSQL
docker compose up -d

# 4. Run the server (migrations run automatically)
cargo run
```

The server starts at `http://localhost:3000`. The root endpoint (`GET /`) and `GET /api/health` confirm it's live.

---

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `DATABASE_URL` | Yes | — | `postgres://user:pass@host/db` |
| `JWT_SIGNING_KEY` | Yes | — | Secret key for HS256 JWTs (min 32 chars) |
| `PAYSTACK_SECRET_KEY` | Yes | — | From your Paystack dashboard |
| `PAYSTACK_MOCK` | No | `false` | Set `true` to skip real Paystack calls in dev |
| `ALLOWED_ORIGINS` | No | permissive | Comma-separated CORS origins |
| `HOST` | No | `127.0.0.1` | Bind address |
| `PORT` | No | `3000` | Bind port |
| `RUST_LOG` | No | `rust_ecommerce=debug` | Log level filter |

---

## Testing

The test suite uses `#[sqlx::test]` — each test gets its own isolated PostgreSQL database with all migrations applied, torn down after the test completes. No test state bleeds into another.

```bash
# Start the database (if not already running)
docker compose up -d

# Run all tests
cargo test

# Run a specific test file
cargo test --test auth_test
cargo test --test product_test
cargo test --test order_test
cargo test --test payment_test

# See println! output
cargo test -- --nocapture
```

**What's covered:**
- Auth: signup validation, duplicate email, login, JWT enforcement, wrong password
- Products: admin-only create/delete, public listing, soft delete visibility
- Orders: cart → order flow, atomic stock decrement, oversell prevention, role access
- Payment: mock payment flow, Paystack webhook HMAC signature, idempotency

---

## Key Design Decisions

- **No oversell** — Stock decrement runs inside the order transaction: `UPDATE products SET stock_quantity = stock_quantity - $qty WHERE id = $id AND stock_quantity >= $qty`. Zero rows affected = rollback with 400.
- **JWT extractors** — `Claims` and `AdminClaims` implement `FromRequestParts`, so adding `claims: Claims` to any handler signature automatically protects it — no middleware wiring needed.
- **Webhook idempotency** — Paystack can deliver webhooks more than once. Each event is inserted into `webhook_events` with `ON CONFLICT (reference, event_type) DO NOTHING`; duplicate deliveries return 200 immediately.
- **No email enumeration** — `/forgot-password` always returns 200 regardless of whether the email exists.
- **Soft deletes** — Products are never hard-deleted; `is_active = false` hides them from the public catalog while preserving order history.

---

## Outro

This project started as a quick Rust experiment and turned into something I'm genuinely proud to put on a portfolio. It covers the full surface area of a real production API: auth with refresh tokens, payment provider integration with webhook verification, role-based access, transactional stock management, and a test suite that runs against a real database.

If you're exploring Rust for backend development, I hope this gives you a practical starting point. The type system really does catch an entire class of bugs before they reach production — and `cargo test` on a project like this is a genuinely different experience from most dynamic language stacks.

**Built with** Rust · Axum · PostgreSQL · Paystack · Docker
