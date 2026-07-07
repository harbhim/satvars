use anyhow::{Result, anyhow};

use crate::PipelineStage;
use crate::{pipeline::Pipeline, sink::Sink, source::Source};

pub struct PipelineBuilder {
    source: Option<Box<dyn Source>>,
    sink: Option<Box<dyn Sink>>,
    stages: Vec<Box<dyn PipelineStage>>,
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Self {
            source: None,
            sink: None,
            stages: Vec::new(),
        }
    }

    #[must_use]
    pub fn source<S>(mut self, source: S) -> Self
    where
        S: Source + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }

    #[must_use]
    pub fn sink<S>(mut self, sink: S) -> Self
    where
        S: Sink + 'static,
    {
        self.sink = Some(Box::new(sink));
        self
    }

    #[must_use]
    pub fn stage<S>(mut self, stage: S) -> Self
    where
        S: PipelineStage + 'static,
    {
        self.stages.push(Box::new(stage));
        self
    }

    pub fn build(mut self) -> Result<Pipeline> {
        let source = self
            .source
            .take()
            .ok_or_else(|| anyhow!("Pipeline source is required"))?;

        let mut pipeline = Pipeline::new(source);

        for stage in self.stages {
            pipeline.add_stage(stage);
        }

        if let Some(sink) = self.sink.take() {
            pipeline.set_sink(sink);
        }

        Ok(pipeline)
    }
}
