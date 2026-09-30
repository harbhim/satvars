use satva_arrow::{Column, RecordBatch, filter_batch, select_columns, set_column};
use satva_expr::{field, lit};
use satva_types::{Record, Value};

fn batch() -> RecordBatch {
    RecordBatch::try_new(vec![
        Column::int64("id", vec![1, 2, 3]),
        Column::boolean("active", vec![true, false, true]),
        Column::float64("salary", vec![80_000.0, 40_000.0, 90_000.0]),
        Column::utf8(
            "name",
            vec!["ada".to_string(), "ben".to_string(), "cy".to_string()],
        ),
        Column::utf8(
            "department",
            vec![
                "Engineering".to_string(),
                "Sales".to_string(),
                "Engineering".to_string(),
            ],
        ),
    ])
    .unwrap()
}

#[test]
fn round_trips_records_and_promotes_mixed_numbers() {
    let mut first = Record::new();
    first.insert("id", Value::Int64(1));
    first.insert("name", Value::Null);
    let mut second = Record::new();
    second.insert("id", Value::Float64(2.5));
    second.insert("name", Value::string("ada"));
    second.insert("active", Value::Boolean(true));

    let batch = RecordBatch::from_records(&[first, second]).unwrap();
    let records = batch.to_records();

    assert_eq!(records[0].get("id"), Some(&Value::Float64(1.0)));
    assert_eq!(records[0].get("name"), Some(&Value::Null));
    assert_eq!(records[0].get("active"), Some(&Value::Null));
    assert_eq!(records[1].get("name"), Some(&Value::string("ada")));
    assert_eq!(records[1].get("active"), Some(&Value::Boolean(true)));
}

#[test]
fn rejects_mixed_string_and_number_columns() {
    let mut first = Record::new();
    first.insert("id", Value::Int64(1));
    let mut second = Record::new();
    second.insert("id", Value::string("nope"));

    let error = RecordBatch::from_records(&[first, second]).unwrap_err();
    assert!(error.to_string().contains("mixed types"));
}

#[test]
fn filters_compares_and_short_circuits() {
    let batch = batch();
    let expr = field("active").equal_to(lit(true)).and(
        field("department")
            .equal_to(lit("Engineering"))
            .and(field("salary").greater_than_or_equal_to(lit(50_000.0))),
    );
    let (filtered, skipped) = filter_batch(&batch, &expr).unwrap();
    assert_eq!(skipped, 1);
    assert_eq!(filtered.to_records()[0].get("id"), Some(&Value::Int64(1)));
    assert_eq!(filtered.num_rows(), 2);

    let skipped_missing = lit(false).and(field("missing").equal_to(lit(1)));
    assert!(filter_batch(&batch, &skipped_missing).is_ok());
    assert!(filter_batch(&batch, &lit(true).and(field("missing").equal_to(lit(1)))).is_err());
}

#[test]
fn sets_and_selects_columns() {
    let batch = batch();
    let with_bonus = set_column(&batch, "bonus", &field("salary").times(lit(0.15))).unwrap();
    let with_name = set_column(
        &with_bonus,
        "display_name",
        &field("name")
            .upper()
            .plus(lit(" - "))
            .plus(field("department")),
    )
    .unwrap();
    let selected =
        select_columns(&with_name, &["name", "bonus", "display_name", "missing"]).unwrap();

    assert_eq!(selected.column("display_name").unwrap().data().len(), 3);
    let records = selected.to_records();
    assert!(records[0].get("salary").is_none());
    assert_eq!(records[0].get("bonus"), Some(&Value::Float64(12_000.0)));
    assert_eq!(
        records[0].get("display_name"),
        Some(&Value::string("ADA - Engineering"))
    );
    assert_eq!(records[1].get("bonus"), Some(&Value::Float64(6_000.0)));
}

#[test]
fn null_ordered_compare_is_false_and_overflow_fails() {
    let mut first = Record::new();
    first.insert("n", Value::Null);
    let mut second = Record::new();
    second.insert("n", Value::Int64(5));
    let batch = RecordBatch::from_records(&[first, second]).unwrap();
    let (filtered, skipped) = filter_batch(&batch, &field("n").greater_than(lit(1))).unwrap();
    assert_eq!(skipped, 1);
    assert_eq!(filtered.to_records()[0].get("n"), Some(&Value::Int64(5)));

    let overflow = set_column(&filtered, "n", &field("n").plus(lit(i64::MAX)));
    assert!(overflow.unwrap_err().to_string().contains("overflow"));
}

#[test]
fn split_shares_full_batches_without_changing_rows() {
    let batch = batch();
    let parts = batch.split(2);
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].num_rows(), 2);
    assert_eq!(parts[1].num_rows(), 1);
    assert_eq!(parts[0].to_records()[1].get("id"), Some(&Value::Int64(2)));
}
