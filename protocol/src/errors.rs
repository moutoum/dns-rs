use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug)]
pub enum Error {
    OutOfRange {
        expected: usize,
        max: usize,
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match *self {
            Error::OutOfRange { expected, max } => write!(f, "out of range error: expected {} but the limit is {}", expected, max),
        }
    }
}

impl std::error::Error for Error {}