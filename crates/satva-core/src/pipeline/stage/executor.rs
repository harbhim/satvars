use crate::{PipelineStage, Record, StageContext, StageResult};

pub struct StageExecutionResult {
    pub stage: &'static str,
    pub result: StageResult,
}

pub struct StageExecutor<'a> {
    stages: &'a [Box<dyn PipelineStage>],
}

impl<'a> StageExecutor<'a> {
    pub fn new(stages: &'a [Box<dyn PipelineStage>]) -> Self {
        Self { stages }
    }

    pub fn execute(&self, record: &mut Record, context: &StageContext) -> StageExecutionResult {
        for stage in self.stages {
            match stage.execute(record, context) {
                StageResult::Continue => {}
                result => {
                    return StageExecutionResult {
                        stage: stage.name(),
                        result,
                    };
                }
            }
        }

        StageExecutionResult {
            stage: "",
            result: StageResult::Continue,
        }
    }
}
