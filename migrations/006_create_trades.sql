-- Create trades table
CREATE TABLE IF NOT EXISTS trades (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    settlement_id UUID NOT NULL REFERENCES settlements(id),
    order_uid UUID NOT NULL REFERENCES orders(uid),
    executed_sell NUMERIC NOT NULL,
    executed_buy NUMERIC NOT NULL,
    surplus NUMERIC NOT NULL DEFAULT 0
);

CREATE INDEX idx_trades_settlement_id ON trades(settlement_id);
CREATE INDEX idx_trades_order_uid ON trades(order_uid);

-- Create clearing prices table
CREATE TABLE IF NOT EXISTS clearing_prices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    settlement_id UUID NOT NULL REFERENCES settlements(id),
    token_id UUID NOT NULL REFERENCES tokens(id),
    price NUMERIC NOT NULL,
    UNIQUE(settlement_id, token_id)
);

CREATE INDEX idx_clearing_prices_settlement_id ON clearing_prices(settlement_id);

