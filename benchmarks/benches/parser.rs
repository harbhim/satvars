use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

use satva_parser::parse_expression;

fn bench_parse_field(c: &mut Criterion) {
    c.bench_function("parse_field", |b| {
        b.iter(|| parse_expression("salary").unwrap())
    });
}

fn bench_parse_simple_comparison(c: &mut Criterion) {
    c.bench_function("parse_simple_comparison", |b| {
        b.iter(|| parse_expression("age > 25").unwrap())
    });
}

fn bench_parse_complex_filter(c: &mut Criterion) {
    c.bench_function("parse_complex_filter", |b| {
        b.iter(|| {
            parse_expression(
                "is_not_null(salary) && is_not_null(active) \
                 && active == true && department == \"Engineering\" \
                 && salary >= 70000",
            )
            .unwrap()
        })
    });
}

fn bench_parse_nested_arithmetic(c: &mut Criterion) {
    c.bench_function("parse_nested_arithmetic", |b| {
        b.iter(|| parse_expression("(price - discount) * 1.1 + tax").unwrap())
    });
}

fn bench_parse_function_call(c: &mut Criterion) {
    c.bench_function("parse_function_call", |b| {
        b.iter(|| {
            parse_expression(
                "upper(trim(first_name)) + \" \" + upper(trim(last_name))",
            )
            .unwrap()
        })
    });
}

fn bench_parse_multiple_strings(c: &mut Criterion) {
    let expressions = vec![
        "active == true",
        "age >= 18",
        "department == \"Engineering\" && salary > 50000",
        "is_not_null(email)",
        "upper(first_name)",
        "coalesce(middle_name, last_name, \"N/A\")",
        "salary * 0.15",
        "(a + b) * (c - d) / e",
        "length(trim(name)) > 0",
        "cast_int(salary) + cast_int(bonus)",
    ];

    c.bench_function("parse_batch_10_expressions", |b| {
        b.iter_batched(
            || expressions.clone(),
            |exprs| {
                for expr in &exprs {
                    let _ = parse_expression(expr).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    benches,
    bench_parse_field,
    bench_parse_simple_comparison,
    bench_parse_complex_filter,
    bench_parse_nested_arithmetic,
    bench_parse_function_call,
    bench_parse_multiple_strings,
);

criterion_main!(benches);
