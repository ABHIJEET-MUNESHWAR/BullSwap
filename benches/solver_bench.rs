use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rust_decimal::Decimal;
use uuid::Uuid;

// We can't use crate imports directly in a benchmark, but criterion
// benchmarks compiled as separate binaries with access to the lib.

use bullswap::domain::order::{Order, OrderKind, OrderStatus, OrderUid};
use bullswap::solver::competition::SolverCompetition;
use bullswap::solver::cow_finder;
use bullswap::solver::engine::BatchSolver;
use bullswap::solver::naive_solver::NaiveSolver;
use bullswap::solver::optimizer;
use bullswap::solver::surplus;

use chrono::Utc;
use std::sync::Arc;

fn make_order_pair(token_a: Uuid, token_b: Uuid, idx: usize) -> Vec<Order> {
    let sell_amount = Decimal::from(100 + (idx % 50) as i64);
    let buy_amount = Decimal::from(40 + (idx % 30) as i64);

    vec![
        Order {
            uid: OrderUid::new(),
            owner: format!("0xSeller{}", idx),
            sell_token: token_a,
            buy_token: token_b,
            sell_amount,
            buy_amount,
            kind: OrderKind::Sell,
            status: OrderStatus::Open,
            signature: "sig".to_string(),
            batch_id: None,
            valid_to: Utc::now() + chrono::Duration::hours(1),
            created_at: Utc::now(),
        },
        Order {
            uid: OrderUid::new(),
            owner: format!("0xBuyer{}", idx),
            sell_token: token_b,
            buy_token: token_a,
            sell_amount: buy_amount,
            buy_amount: sell_amount,
            kind: OrderKind::Buy,
            status: OrderStatus::Open,
            signature: "sig".to_string(),
            batch_id: None,
            valid_to: Utc::now() + chrono::Duration::hours(1),
            created_at: Utc::now(),
        },
    ]
}

fn generate_orders(count: usize) -> Vec<Order> {
    let token_a = Uuid::new_v4();
    let token_b = Uuid::new_v4();
    let token_c = Uuid::new_v4();

    let mut orders = Vec::with_capacity(count);
    for i in 0..count / 2 {
        // Alternate between two token pairs for variety
        if i % 2 == 0 {
            orders.extend(make_order_pair(token_a, token_b, i));
        } else {
            orders.extend(make_order_pair(token_b, token_c, i));
        }
    }
    orders.truncate(count);
    orders
}

fn bench_cow_finder(c: &mut Criterion) {
    let mut group = c.benchmark_group("cow_finder");

    for size in [10, 100, 500, 1000, 5000] {
        let orders = generate_orders(size);
        group.bench_with_input(
            BenchmarkId::new("find_cows", size),
            &orders,
            |b, orders| {
                b.iter(|| cow_finder::find_cows(black_box(orders)));
            },
        );
    }

    group.finish();
}

fn bench_optimizer(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimizer");

    for size in [10, 100, 500, 1000] {
        let orders = generate_orders(size);
        group.bench_with_input(
            BenchmarkId::new("compute_clearing_prices", size),
            &orders,
            |b, orders| {
                b.iter(|| optimizer::compute_clearing_prices(black_box(orders)));
            },
        );
    }

    group.finish();
}

fn bench_surplus(c: &mut Criterion) {
    let mut group = c.benchmark_group("surplus");

    for size in [10, 100, 500, 1000] {
        let orders = generate_orders(size);
        let prices = optimizer::compute_clearing_prices(&orders);
        let executions = optimizer::optimize_execution(&orders, &prices);

        group.bench_with_input(
            BenchmarkId::new("calculate_total_surplus", size),
            &(&orders, &executions),
            |b, (orders, executions)| {
                b.iter(|| surplus::calculate_total_surplus(black_box(orders), black_box(executions)));
            },
        );
    }

    group.finish();
}

fn bench_naive_solver(c: &mut Criterion) {
    let mut group = c.benchmark_group("naive_solver");

    for size in [10, 100, 500, 1000] {
        let orders = generate_orders(size);
        let solver = NaiveSolver::new(Uuid::new_v4());

        group.bench_with_input(
            BenchmarkId::new("solve", size),
            &orders,
            |b, orders| {
                b.iter(|| solver.solve(black_box(orders), Uuid::new_v4()));
            },
        );
    }

    group.finish();
}

fn bench_solver_competition(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_competition");

    for size in [10, 100, 500] {
        let orders = generate_orders(size);
        let solver1 = Arc::new(NaiveSolver::new(Uuid::new_v4()));
        let solver2 = Arc::new(NaiveSolver::new(Uuid::new_v4()));
        let competition = SolverCompetition::new(vec![solver1, solver2]);

        group.bench_with_input(
            BenchmarkId::new("run_competition", size),
            &orders,
            |b, orders| {
                b.iter(|| competition.run(black_box(orders), Uuid::new_v4()));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_cow_finder,
    bench_optimizer,
    bench_surplus,
    bench_naive_solver,
    bench_solver_competition,
);
criterion_main!(benches);

