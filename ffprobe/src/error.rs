use std::error;
use std::fmt;
use std::io;
use std::result;

use serde_json;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Json(serde_json::Error),
    SpawnError(String),
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(e) => write!(w, "ProbeError({})", e),
            Error::Json(e) => write!(w, "ProbeError({})", e),
            Error::SpawnError(e) => write!(w, "ProbeError({})", e),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::Io(e) => e.description(),
            Error::Json(e) => e.description(),
            Error::SpawnError(_) => "spawn error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::Io(e) => e.cause(),
            Error::Json(e) => e.cause(),
            Error::SpawnError(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::Json(err)
    }
}
