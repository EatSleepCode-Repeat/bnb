use crate::executor::pipeline::execute_pipeline;
use crate::parser::ast::Pipeline;

pub fn run_pipeline(pipeline: &Pipeline) -> Result<i32, String> {
    execute_pipeline(pipeline)
}
