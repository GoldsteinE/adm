use std::fmt;

use color_eyre::eyre;

#[derive(Debug)]
pub enum Status {
    Fail(eyre::Report),
    Success,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Fail(err) => write!(f, "{}", err),
            Status::Success => f.write_str("completed"),
        }
    }
}

impl<T> From<Result<T, eyre::Report>> for Status {
    fn from(res: Result<T, eyre::Report>) -> Self {
        match res {
            Ok(_) => Self::Success,
            Err(err) => Self::Fail(err),
        }
    }
}
