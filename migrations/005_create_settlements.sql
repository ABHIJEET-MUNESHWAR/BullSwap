-- Create settlements table
CREATE TABLE IF NOT EXISTS settlements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    batch_id UUID NOT NULL UNIQUE REFERENCES batches(id),
    solver_id UUID NOT NULL REFERENCES solvers(id),
    objective_value NUMERIC NOT NULL DEFAULT 0,
    surplus_total NUMERIC NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_settlements_batch_id ON settlements(batch_id);

