use std::fmt;

#[derive(Debug, Clone)]
pub enum CalendarError {
    AccessDenied,
}

impl fmt::Display for CalendarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalendarError::AccessDenied => write!(f, "Calendar access denied by user"),
        }
    }
}

impl std::error::Error for CalendarError {}
