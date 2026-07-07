use satva_expr::{Evaluator, Expression};
use satva_types::{Record, Value};

use crate::{PipelineStage, StageContext, StageError, StageResult};

pub struct FilterStage {
    expression: Expression,
    reason: Option<String>,
}

impl FilterStage {
    pub fn new(expression: Expression) -> Self {
        Self {
            expression,
            reason: None,
        }
    }

    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    fn skip_reason(&self) -> String {
        self.reason
            .clone()
            .unwrap_or_else(|| "Record did not satisfy filter".to_string())
    }
}

impl PipelineStage for FilterStage {
    fn name(&self) -> &'static str {
        "Filter"
    }

    fn execute(&self, record: &mut Record, _ctx: &StageContext) -> StageResult {
        match Evaluator::evaluate(&self.expression, record) {
            Ok(Value::Boolean(true)) => StageResult::continue_(),

            Ok(Value::Boolean(false)) => StageResult::skip(self.skip_reason()),

            Ok(value) => StageResult::fail(StageError::execution(
                self.name(),
                format!("Filter expression returned '{value}', expected Boolean"),
            )),

            Err(err) => StageResult::fail(StageError::execution(self.name(), err.to_string())),
        }
    }
}
