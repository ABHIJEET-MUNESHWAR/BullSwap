-- Create batches table
CREATE TABLE IF NOT EXISTS batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status TEXT NOT NULL DEFAULT 'collecting',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    solved_at TIMESTAMPTZ,
    settled_at TIMESTAMPTZ,
    order_count BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_batches_status ON batches(status);
CREATE INDEX idx_batches_created_at ON batches(created_at DESC);

