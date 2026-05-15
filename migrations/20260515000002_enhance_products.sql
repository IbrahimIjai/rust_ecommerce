ALTER TABLE products
    ADD COLUMN description    TEXT,
    ADD COLUMN stock_quantity INT NOT NULL DEFAULT 0,
    ADD COLUMN category       TEXT,
    ADD COLUMN image_url      TEXT,
    ADD COLUMN is_active      BOOLEAN NOT NULL DEFAULT TRUE;

CREATE INDEX IF NOT EXISTS idx_products_category  ON products(category);
CREATE INDEX IF NOT EXISTS idx_products_is_active ON products(is_active);
