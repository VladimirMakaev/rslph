#[derive(Debug, Clone, PartialEq)]
pub enum OutputLine {
    Stdout(String),
    Stderr(String),
}
