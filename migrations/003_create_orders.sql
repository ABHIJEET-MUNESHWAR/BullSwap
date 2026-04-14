-- Create orders table
CREATE TABLE IF NOT EXISTS orders (
    uid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner TEXT NOT NULL,
    sell_token UUID NOT NULL REFERENCES tokens(id),
    buy_token UUID NOT NULL REFERENCES tokens(id),
    sell_amount NUMERIC NOT NULL,
    buy_amount NUMERIC NOT NULL,
    kind TEXT NOT NULL DEFAULT 'sell',
    status TEXT NOT NULL DEFAULT 'open',
    signature TEXT NOT NULL,
    batch_id UUID REFERENCES batches(id),
    valid_to TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_sell_amount_positive CHECK (sell_amount > 0),
    CONSTRAINT chk_buy_amount_positive CHECK (buy_amount > 0),
    CONSTRAINT chk_different_tokens CHECK (sell_token != buy_token)
);

CREATE INDEX idx_orders_status ON orders(status);
CREATE INDEX idx_orders_owner ON orders(owner);
CREATE INDEX idx_orders_batch_id ON orders(batch_id);
CREATE INDEX idx_orders_valid_to ON orders(valid_to);
CREATE INDEX idx_orders_sell_buy_token ON orders(sell_token, buy_token);

