use crate::executor::pipeline::{execute_chain, execute_pipeline};
use crate::parser::ast::{CommandChain, Pipeline};

#[allow(dead_code)]
pub fn run_chain(chain: &CommandChain) -> Result<i32, String> {
    execute_chain(chain)
}

#[allow(dead_code)]
pub fn run_pipeline(pipeline: &Pipeline) -> Result<i32, String> {
    execute_pipeline(pipeline)
}
