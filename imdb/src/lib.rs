#![feature(nll)]

extern crate bincode;
extern crate csv;
extern crate flate2;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate strsim;

mod error;
mod index;
mod title;
mod util;

pub use error::{Error, Result};
pub use index::Imdb;
pub use title::{Title, TitleKind};
