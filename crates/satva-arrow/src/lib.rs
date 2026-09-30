//! Columnar batches and vectorized kernels for filter, select, and computed fields.
//!
//! Cloning a [`RecordBatch`] shares column storage. Row conversion stays at the
//! source and sink boundary.

mod array;
mod batch;
mod eval;
mod kernels;

pub use array::ColumnArray;
pub use batch::{Column, DEFAULT_BATCH_SIZE, RecordBatch};
pub use eval::{filter_batch, select_columns, set_column};
