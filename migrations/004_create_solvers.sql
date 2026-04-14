-- Create solvers table
CREATE TABLE IF NOT EXISTS solvers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    active BOOLEAN NOT NULL DEFAULT TRUE
);

-- Seed default solvers
INSERT INTO solvers (id, name, active) VALUES
    ('b0000000-0000-0000-0000-000000000001', 'naive_solver', TRUE),
    ('b0000000-0000-0000-0000-000000000002', 'cow_finder', TRUE)
ON CONFLICT (name) DO NOTHING;

