#![feature(nll)]

extern crate bincode;
extern crate csv;
extern crate failure;
extern crate flate2;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate maplit;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate strsim;
extern crate walkdir;
extern crate yansi;

mod imdb;
mod input;
mod parse;
mod scan;

use std::io::prelude::*;

use failure::Error;
use yansi::Paint;

use imdb::Imdb;
use input::Input;
use scan::{scan_root, ScanEntry};

fn lookup<'db>(input: &Input, imdb: &'db Imdb, entry: &mut ScanEntry<'db>) {
    loop {
        let fullname = input.ask_line("What movie is it?");
        let (name, year) = parse::parse_movie(&fullname);
        if let Some(title) = imdb.lookup(&name, year) {
            if input.confirm(
                &format!(
                    "Found '{} ({})', is this it?",
                    title.primary_title(),
                    title.year().unwrap()
                ),
                None,
            ) {
                entry.title = title;
                break;
            }
        } else {
            println!("Found nothing.");
        }
    }
}

fn foo() -> Result<(), Error> {
    let imdb = Imdb::load_or_create_index("movies.index.gz", "title.basics.tsv.gz")?;
    // println!("done");

    // loop {
    //     let mut line = String::new();
    //     print!("> ");
    //     std::io::stdout().flush();
    //     std::io::stdin().read_line(&mut line)?;
    //     println!("{} -> {:?}", line, imdb.lookup(line.trim(), None));
    // }

    let input = Input::new();
    let entries = scan_root("/home/simon/tank/movies/en", &imdb)?;
    //println!("{:#?}", entries);

    for mut entry in entries {
        if entry.path != entry.new_path {
            println!("Old: {}", Paint::red(entry.path.display()));
            println!("New: {}", Paint::green(entry.new_path.display()));
            let choice = input.select(
                "What do with this?",
                [
                    ("a", "accept"),
                    ("s", "skip"),
                    ("l", "lookup"),
                    ("d", "delete"),
                ],
                Some("a"),
            );

            match choice {
                "a" => {
                    println!("Accepted...");
                }
                "s" => {
                    println!("Skipping...");
                }
                "l" => {
                    lookup(&input, &imdb, &mut entry);
                }
                "d" => {
                    println!("Deleting...");
                }
                _ => unreachable!(),
            }

            println!();
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = foo() {
        println!("{}", e);
        println!("{}", e.backtrace());
    }
}
