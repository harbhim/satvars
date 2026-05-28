use anyhow::Result;

use crate::pipeline_stage::PipelineStage;
use crate::source::Source;
use crate::stage_result::StageResult;

pub struct Pipeline {
    source: Box<dyn Source>,

    stages: Vec<Box<dyn PipelineStage>>,
}

impl Pipeline {
    pub fn new(source: Box<dyn Source>) -> Self {
        Self {
            source,
            stages: Vec::new(),
        }
    }

    pub fn add_stage(&mut self, stage: Box<dyn PipelineStage>) {
        self.stages.push(stage);
    }

    pub fn run(&self) -> Result<()> {
        let records = self.source.read()?;

        for record in records {
            let mut current_record = Some(record);
            for stage in &self.stages {
                let record = current_record.take().expect("record should exist");

                match stage.execute(record) {
                    StageResult::Continue(record) => {
                        current_record = Some(record);
                    }

                    StageResult::Skip { record, reason } => {
                        println!("\nSKIPPED RECORD:");

                        println!("{:#?}", record);

                        println!("\nSKIP REASON:");

                        println!("{:#?}", reason);

                        current_record = None;

                        break;
                    }

                    StageResult::Error(error) => {
                        println!("\nPIPELINE ERROR:");

                        println!("{:#?}", error);

                        return Ok(());
                    }
                }
            }

            if let Some(record) = current_record {
                println!("\nFINAL RECORD:");

                println!("{:#?}", record);
            }
        }

        Ok(())
    }
}
