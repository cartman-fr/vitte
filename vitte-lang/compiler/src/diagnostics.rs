#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity { Error, Warning, Info }

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl Diagnostic {
    pub fn error(msg: impl Into<String>) -> Self {
        Self { severity: Severity::Error, message: msg.into(), file: None, line: None, column: None }
    }
}