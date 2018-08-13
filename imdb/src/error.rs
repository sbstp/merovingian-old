use std::error;
use std::fmt;
use std::io;
use std::num::ParseIntError;
use std::result;

use bincode;
use csv;
use reqwest;

#[derive(Debug)]
pub enum Error {
    Bincode(bincode::Error),
    Csv(csv::Error),
    Io(io::Error),
    ParseIntError(ParseIntError),
    Reqwest(reqwest::Error),
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Bincode(e) => write!(w, "ImdbError({})", e),
            Error::Csv(e) => write!(w, "ImdbError({})", e),
            Error::Io(e) => write!(w, "ImdbError({})", e),
            Error::ParseIntError(e) => write!(w, "ImdbError({})", e),
            Error::Reqwest(e) => write!(w, "ImdbError({})", e),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::Bincode(e) => e.description(),
            Error::Csv(e) => e.description(),
            Error::Io(e) => e.description(),
            Error::ParseIntError(e) => e.description(),
            Error::Reqwest(e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::Bincode(e) => e.cause(),
            Error::Csv(e) => e.cause(),
            Error::Io(e) => e.cause(),
            Error::ParseIntError(e) => e.cause(),
            Error::Reqwest(e) => e.cause(),
        }
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Error {
        Error::Bincode(err)
    }
}

impl From<csv::Error> for Error {
    fn from(err: csv::Error) -> Error {
        Error::Csv(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Error {
        Error::ParseIntError(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error {
        Error::Reqwest(err)
    }
}
