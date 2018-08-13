extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod error;
mod ffprobe;

pub use error::{Error, Result};
pub use ffprobe::scan;
