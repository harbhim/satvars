use anyhow::Result;
use satva_arrow::{RecordBatch, filter_batch, select_columns, set_column};
use satva_expr::Expression;

use super::{AppliedBatch, BatchStage};

/// Vectorized filter. Rows that evaluate to false are skipped.
pub struct BatchFilter {
    expression: Expression,
}

impl BatchFilter {
    pub fn new(expression: Expression) -> Self {
        Self { expression }
    }
}

impl BatchStage for BatchFilter {
    fn apply(&self, batch: RecordBatch) -> Result<AppliedBatch> {
        let input_rows = batch.num_rows();
        let (batch, skipped) = filter_batch(&batch, &self.expression)?;
        debug_assert_eq!(input_rows - batch.num_rows(), skipped);
        Ok(AppliedBatch { batch, skipped })
    }
}

/// Vectorized projection. Field order follows the input batch.
pub struct SelectFieldsBatch {
    fields: Vec<String>,
}

impl SelectFieldsBatch {
    pub fn new<I, S>(fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            fields: fields.into_iter().map(Into::into).collect(),
        }
    }
}

impl BatchStage for SelectFieldsBatch {
    fn apply(&self, batch: RecordBatch) -> Result<AppliedBatch> {
        let names: Vec<&str> = self.fields.iter().map(String::as_str).collect();
        let batch = select_columns(&batch, &names)?;
        Ok(AppliedBatch { batch, skipped: 0 })
    }
}

/// Vectorized computed field.
pub struct SetFieldBatch {
    field: String,
    expression: Expression,
}

impl SetFieldBatch {
    pub fn new(field: impl Into<String>, expression: Expression) -> Self {
        Self {
            field: field.into(),
            expression,
        }
    }
}

impl BatchStage for SetFieldBatch {
    fn apply(&self, batch: RecordBatch) -> Result<AppliedBatch> {
        let batch = set_column(&batch, &self.field, &self.expression)?;
        Ok(AppliedBatch { batch, skipped: 0 })
    }
}
