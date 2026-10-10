use crate::executor::pipeline::{execute_chain, execute_pipeline};
use crate::parser::ast::{CommandChain, Pipeline};
use crate::safety::SafetyEngine;

#[allow(dead_code)]
pub fn run_chain(chain: &CommandChain) -> Result<i32, String> {
    // Check all pipelines in the chain before execution
    for chained in &chain.chains {
        if !verify_pipeline_safety(&chained.pipeline) {
            return Ok(130); // Return SIGINT / user cancellation exit code
        }
    }

    execute_chain(chain)
}

#[allow(dead_code)]
pub fn run_pipeline(pipeline: &Pipeline) -> Result<i32, String> {
    if !verify_pipeline_safety(pipeline) {
        return Ok(130);
    }

    execute_pipeline(pipeline)
}

/// Helper to inspect all commands in a pipeline for destructive actions
fn verify_pipeline_safety(pipeline: &Pipeline) -> bool {
    for command in &pipeline.commands {
        if !SafetyEngine::intercept(&command.args) {
            return false; // User aborted the prompt
        }
    }
    true
}
