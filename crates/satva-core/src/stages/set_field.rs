use satva_expr::{Evaluator, Expression};
use satva_types::Record;

use crate::{PipelineStage, StageContext, StageError, StageResult};

pub struct SetFieldStage {
    field: String,
    expression: Expression,
}

impl SetFieldStage {
    pub fn new(field: impl Into<String>, expression: Expression) -> Self {
        Self {
            field: field.into(),
            expression,
        }
    }
}

impl PipelineStage for SetFieldStage {
    fn name(&self) -> &'static str {
        "SetField"
    }

    fn execute(&self, record: &mut Record, _ctx: &StageContext) -> StageResult {
        match Evaluator::evaluate(&self.expression, record) {
            Ok(value) => {
                record.insert(&self.field, value);
                StageResult::continue_()
            }
            Err(err) => StageResult::fail(StageError::execution(self.name(), err.to_string())),
        }
    }
}
