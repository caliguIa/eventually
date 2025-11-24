use std::fmt;

#[derive(Debug, Clone)]
pub enum CalendarError {
    AccessDenied,
    StoreUnavailable,
}

impl fmt::Display for CalendarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalendarError::AccessDenied => write!(f, "Calendar access denied by user"),
            CalendarError::StoreUnavailable => write!(f, "Calendar event store unavailable"),
        }
    }
}

impl std::error::Error for CalendarError {}
