#[derive(Debug, Clone, PartialEq)]
pub enum Redirection {
    OutputTruncate(String), // >
    OutputAppend(String),   // >>
    Input(String),          // <
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