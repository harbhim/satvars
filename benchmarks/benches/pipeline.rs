use criterion::{Criterion, criterion_group, criterion_main};

use satva_core::{
    FilterStage, Pipeline, PipelineOptions, SelectFieldsStage, SetFieldStage, Source,
};
use satva_expr::{field, lit};
use satva_types::{Record, Value};

fn make_records(count: usize) -> Vec<Record> {
    (0..count)
        .map(|i| {
            let mut record = Record::new();
            record.insert("id", Value::Int64(i64::try_from(i).unwrap()));
            record.insert("active", Value::Boolean(i % 2 == 0));
            record.insert("age", Value::Int64(i64::try_from(20 + (i % 40)).unwrap()));
            record.insert(
                "department",
                Value::String(["Engineering", "Marketing", "HR", "Sales"][i % 4].to_string()),
            );
            record.insert("salary", Value::Float64(30_000.0 + (i as f64 * 500.0)));
            record.insert("name", Value::String(format!("Employee {i}")));
            record.insert("email", Value::String(format!("emp{i}@example.com")));
            record
        })
        .collect()
}

struct VecSource {
    records: Vec<Record>,
}

impl Source for VecSource {
    fn read(&self) -> anyhow::Result<Box<dyn Iterator<Item = anyhow::Result<Record>>>> {
        let iter = self.records.clone().into_iter().map(Ok::<_, anyhow::Error>);
        Ok(Box::new(iter))
    }
}

fn bench_pipeline_simple_filter_1000(c: &mut Criterion) {
    let records = make_records(1000);
    let expr = field("active").equal_to(lit(true));

    c.bench_function("pipeline_simple_filter_1000", |b| {
        b.iter(|| {
            let source = VecSource {
                records: records.clone(),
            };
            let mut pipeline = Pipeline::new(Box::new(source));
            pipeline.add_stage(Box::new(FilterStage::new(expr.clone())));

            pipeline.run(PipelineOptions::new()).unwrap();
        });
    });
}

fn bench_pipeline_complex_filter_1000(c: &mut Criterion) {
    let records = make_records(1000);
    let expr = field("active").equal_to(lit(true)).and(
        field("department")
            .equal_to(lit("Engineering"))
            .and(field("salary").greater_than_or_equal_to(lit(50_000.0))),
    );

    c.bench_function("pipeline_complex_filter_1000", |b| {
        b.iter(|| {
            let source = VecSource {
                records: records.clone(),
            };
            let mut pipeline = Pipeline::new(Box::new(source));
            pipeline.add_stage(Box::new(FilterStage::new(expr.clone())));

            pipeline.run(PipelineOptions::new()).unwrap();
        });
    });
}

fn bench_pipeline_full_transform_1000(c: &mut Criterion) {
    let records = make_records(1000);
    let filter_expr = field("active")
        .equal_to(lit(true))
        .and(field("salary").greater_than_or_equal_to(lit(50_000.0)));
    let bonus_expr = field("salary").times(lit(0.15));
    let display_expr = field("name")
        .upper()
        .plus(lit(" - "))
        .plus(field("department"));

    c.bench_function("pipeline_full_transform_1000", |b| {
        b.iter(|| {
            let source = VecSource {
                records: records.clone(),
            };
            let mut pipeline = Pipeline::new(Box::new(source));
            pipeline.add_stage(Box::new(FilterStage::new(filter_expr.clone())));
            pipeline.add_stage(Box::new(SetFieldStage::new("bonus", bonus_expr.clone())));
            pipeline.add_stage(Box::new(SetFieldStage::new(
                "display_name",
                display_expr.clone(),
            )));
            pipeline.add_stage(Box::new(SelectFieldsStage::new(vec![
                "id".to_string(),
                "name".to_string(),
                "department".to_string(),
                "salary".to_string(),
                "bonus".to_string(),
                "display_name".to_string(),
            ])));

            pipeline.run(PipelineOptions::new()).unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_pipeline_simple_filter_1000,
    bench_pipeline_complex_filter_1000,
    bench_pipeline_full_transform_1000,
);

criterion_main!(benches);
