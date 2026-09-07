use std::fmt;

#[derive(Debug)]
pub enum JoyError {
    Compile(String),
    Parse(String),
    Execute(String),
    Io(String),
}

impl fmt::Display for JoyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JoyError::Compile(msg) => write!(f, "compile error: {}", msg),
            JoyError::Parse(msg) => write!(f, "parse error: {}", msg),
            JoyError::Execute(msg) => write!(f, "execution error: {}", msg),
            JoyError::Io(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for JoyError {}

impl From<std::io::Error> for JoyError {
    fn from(e: std::io::Error) -> Self {
        JoyError::Io(e.to_string())
    }
}
