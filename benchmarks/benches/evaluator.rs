#![allow(clippy::question_mark, clippy::let_and_return)]

use criterion::{Criterion, criterion_group, criterion_main};

use satva_expr::{Evaluator, field, lit};
use satva_types::{Record, Value};

fn make_record() -> Record {
    let mut record = Record::new();
    record.insert("id", Value::Int64(42));
    record.insert("active", Value::Boolean(true));
    record.insert("age", Value::Int64(30));
    record.insert("salary", Value::Float64(75_000.50));
    record.insert("bonus", Value::Float64(2_500.0));
    record.insert("rating", Value::Float64(4.5));
    record.insert("department", Value::String("Engineering".to_string()));
    record.insert("name", Value::String("Alice".to_string()));
    record.insert("email", Value::String("alice@example.com".to_string()));
    record.insert("city", Value::String("New York".to_string()));
    record
}

fn bench_evaluate_field_lookup(c: &mut Criterion) {
    let record = make_record();
    let expr = field("salary");

    c.bench_function("evaluate_field_lookup", |b| {
        b.iter(|| Evaluator::evaluate(&expr, &record).unwrap())
    });
}

fn bench_evaluate_simple_comparison(c: &mut Criterion) {
    let record = make_record();
    let expr = field("age").greater_than(lit(18));

    c.bench_function("evaluate_simple_comparison", |b| {
        b.iter(|| Evaluator::evaluate(&expr, &record).unwrap())
    });
}

fn bench_evaluate_arithmetic(c: &mut Criterion) {
    let record = make_record();
    let expr = field("salary").plus(field("bonus"));

    c.bench_function("evaluate_arithmetic", |b| {
        b.iter(|| Evaluator::evaluate(&expr, &record).unwrap())
    });
}

fn bench_evaluate_and_shortcircuit(c: &mut Criterion) {
    let record = make_record();
    // left is false so right (missing field) is never evaluated
    let expr = field("age")
        .less_than(lit(10))
        .and(field("nonexistent").greater_than(lit(0)));

    c.bench_function("evaluate_and_shortcircuit", |b| {
        b.iter(|| Evaluator::evaluate(&expr, &record).unwrap())
    });
}

fn bench_evaluate_complex_filter(c: &mut Criterion) {
    let record = make_record();
    let expr = field("active")
        .equal_to(lit(true))
        .and(
            field("department")
                .equal_to(lit("Engineering"))
                .and(field("salary").greater_than_or_equal_to(lit(70_000.0))),
        );

    c.bench_function("evaluate_complex_filter", |b| {
        b.iter(|| Evaluator::evaluate(&expr, &record).unwrap())
    });
}

fn bench_evaluate_function_chain(c: &mut Criterion) {
    let record = make_record();
    let expr = field("name").upper().trim().length();

    c.bench_function("evaluate_function_chain", |b| {
        b.iter(|| Evaluator::evaluate(&expr, &record).unwrap())
    });
}

fn bench_evaluate_string_concat(c: &mut Criterion) {
    let record = make_record();
    let expr = field("name")
        .upper()
        .plus(lit(" - "))
        .plus(field("department"));

    c.bench_function("evaluate_string_concat", |b| {
        b.iter(|| Evaluator::evaluate(&expr, &record).unwrap())
    });
}

criterion_group!(
    benches,
    bench_evaluate_field_lookup,
    bench_evaluate_simple_comparison,
    bench_evaluate_arithmetic,
    bench_evaluate_and_shortcircuit,
    bench_evaluate_complex_filter,
    bench_evaluate_function_chain,
    bench_evaluate_string_concat,
);

criterion_main!(benches);
