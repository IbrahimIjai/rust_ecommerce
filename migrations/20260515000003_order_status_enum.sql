CREATE TYPE order_status AS ENUM (
    'pending',
    'payment_initiated',
    'paid',
    'processing',
    'shipped',
    'delivered',
    'cancelled',
    'refunded'
);

ALTER TABLE orders
    ALTER COLUMN status TYPE order_status USING status::order_status;
