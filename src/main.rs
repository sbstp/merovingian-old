#![feature(nll)]

extern crate csv;
extern crate failure;
extern crate flate2;
extern crate strsim;

use failure::Error;

use std::io::prelude::*;

mod imdb;

fn foo() -> Result<(), Error> {
    let imdb = imdb::Imdb::create_index("title.basics.tsv.gz")?;
    println!("done");

    loop {
        let mut line = String::new();
        print!("> ");
        std::io::stdout().flush();
        std::io::stdin().read_line(&mut line)?;
        println!("{} -> {:?}", line, imdb.lookup(line.trim(), None));
    }

    Ok(())
}

fn main() {
    if let Err(e) = foo() {
        println!("{}", e);
        println!("{}", e.backtrace());
    }
}
