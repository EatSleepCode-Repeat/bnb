#[derive(Debug, Clone, PartialEq)]
pub enum Redirection {
    OutputTruncate(String),   // >
    OutputAppend(String),     // >>
    Input(String),            // <
    StderrTruncate(String),   // 2>
    StderrAppend(String),     // 2>>
    OutputAndStderr(String),  // &> or >&
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainOp {
    And,      // &&
    Or,       // ||
    Sequence, // ;
}

#[derive(Debug, Clone)]
pub struct Command {
    pub args: Vec<String>,
    pub redirections: Vec<Redirection>,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub commands: Vec<Command>,
    pub run_in_background: bool, // &
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ChainedPipeline {
    pub pipeline: Pipeline,
    pub next_op: Option<ChainOp>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CommandChain {
    pub chains: Vec<ChainedPipeline>,
}