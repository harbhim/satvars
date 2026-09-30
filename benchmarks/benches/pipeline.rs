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

fn employee_batches(count: usize, batch_size: usize) -> Vec<satva_arrow::RecordBatch> {
    let departments = ["Engineering", "Marketing", "HR", "Sales"];
    let mut batches = Vec::new();
    let mut start = 0;
    while start < count {
        let end = (start + batch_size).min(count);
        let len = end - start;
        let mut id = Vec::with_capacity(len);
        let mut active = Vec::with_capacity(len);
        let mut age = Vec::with_capacity(len);
        let mut department = Vec::with_capacity(len);
        let mut salary = Vec::with_capacity(len);
        let mut name = Vec::with_capacity(len);
        let mut email = Vec::with_capacity(len);
        for offset in 0..len {
            let index = start + offset;
            id.push(i64::try_from(index).unwrap());
            active.push(index % 2 == 0);
            age.push(i64::try_from(20 + (index % 40)).unwrap());
            department.push(departments[index % 4].to_string());
            salary.push(30_000.0 + (index as f64 * 500.0));
            name.push(format!("Employee {index}"));
            email.push(format!("emp{index}@example.com"));
        }
        batches.push(
            satva_arrow::RecordBatch::try_new(vec![
                satva_arrow::Column::int64("id", id),
                satva_arrow::Column::boolean("active", active),
                satva_arrow::Column::int64("age", age),
                satva_arrow::Column::utf8("department", department),
                satva_arrow::Column::float64("salary", salary),
                satva_arrow::Column::utf8("name", name),
                satva_arrow::Column::utf8("email", email),
            ])
            .unwrap(),
        );
        start = end;
    }
    batches
}

fn bench_batch_1m(c: &mut Criterion) {
    use std::time::Duration;

    use satva_core::{
        BatchFilter, BatchPipeline, MemoryBatchSource, PipelineOptions, SelectFieldsBatch,
        SetFieldBatch,
    };
    use satva_execution::ParallelBatchPipeline;

    let batches = employee_batches(1_000_000, satva_arrow::DEFAULT_BATCH_SIZE);
    let simple = field("active").equal_to(lit(true));
    let filter_expr = field("active")
        .equal_to(lit(true))
        .and(field("salary").greater_than_or_equal_to(lit(50_000.0)));
    let bonus_expr = field("salary").times(lit(0.15));
    let display_expr = field("name")
        .upper()
        .plus(lit(" - "))
        .plus(field("department"));
    let selected = [
        "id",
        "name",
        "department",
        "salary",
        "bonus",
        "display_name",
    ];

    let mut group = c.benchmark_group("batch_1m");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("filter_sequential", |b| {
        b.iter(|| {
            let mut pipeline =
                BatchPipeline::new(Box::new(MemoryBatchSource::new(batches.clone())));
            pipeline.add_stage(BatchFilter::new(simple.clone()));
            std::hint::black_box(
                pipeline
                    .run(PipelineOptions::without_logs())
                    .unwrap()
                    .summary
                    .succeeded,
            );
        });
    });

    group.bench_function("filter_parallel", |b| {
        b.iter(|| {
            let mut pipeline =
                ParallelBatchPipeline::new(Box::new(MemoryBatchSource::new(batches.clone())));
            pipeline.add_stage(BatchFilter::new(simple.clone()));
            std::hint::black_box(
                pipeline
                    .run(PipelineOptions::without_logs())
                    .unwrap()
                    .summary
                    .succeeded,
            );
        });
    });

    group.bench_function("transform_sequential", |b| {
        b.iter(|| {
            let mut pipeline =
                BatchPipeline::new(Box::new(MemoryBatchSource::new(batches.clone())));
            pipeline.add_stage(BatchFilter::new(filter_expr.clone()));
            pipeline.add_stage(SetFieldBatch::new("bonus", bonus_expr.clone()));
            pipeline.add_stage(SetFieldBatch::new("display_name", display_expr.clone()));
            pipeline.add_stage(SelectFieldsBatch::new(selected));
            std::hint::black_box(
                pipeline
                    .run(PipelineOptions::without_logs())
                    .unwrap()
                    .summary
                    .succeeded,
            );
        });
    });

    group.bench_function("transform_parallel", |b| {
        b.iter(|| {
            let mut pipeline =
                ParallelBatchPipeline::new(Box::new(MemoryBatchSource::new(batches.clone())));
            pipeline.add_stage(BatchFilter::new(filter_expr.clone()));
            pipeline.add_stage(SetFieldBatch::new("bonus", bonus_expr.clone()));
            pipeline.add_stage(SetFieldBatch::new("display_name", display_expr.clone()));
            pipeline.add_stage(SelectFieldsBatch::new(selected));
            std::hint::black_box(
                pipeline
                    .run(PipelineOptions::without_logs())
                    .unwrap()
                    .summary
                    .succeeded,
            );
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_pipeline_simple_filter_1000,
    bench_pipeline_complex_filter_1000,
    bench_pipeline_full_transform_1000,
    bench_batch_1m,
);

criterion_main!(benches);
