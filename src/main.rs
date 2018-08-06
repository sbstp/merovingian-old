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
extern crate yansi;

mod imdb;
mod input;
mod parse;
mod rename;
mod scan;
mod util;
mod vfs;

use std::io::prelude::*;
use std::path::{Path, PathBuf};

use failure::Error;
use yansi::Paint;

use imdb::{Imdb, Title};
use input::Input;
use scan::{ScanEntry, Scanner};
use util::filter_path;

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

    let input = Input::new();
    let root_path = Path::new("/home/simon/tank/movies/en");
    let root = vfs::walk(&root_path)?;
    let entries = Scanner::new(root, &imdb).scan_root()?;

    for entry in entries {
        let renames = rename::movie(&root_path, &entry);

        let diff: Vec<_> = renames.iter().filter(|r| r.different()).collect();

        if !diff.is_empty() {
            for rename in diff.iter() {
                println!("Old: {}", Paint::red(rename.orig.display()));
            }
            for rename in diff.iter() {
                println!("New: {}", Paint::green(rename.new.display()));
            }

            println!();
        }

        // if entry.movie.file.path() != entry.movie.new {
        //     println!("Old: {}", Paint::red(entry.movie.file.path().display()));
        //     println!("New: {}", Paint::green(entry.movie.new.display()));
        //     // let choice = input.select(
        //     //     "What do with this?",
        //     //     [
        //     //         ("a", "accept"),
        //     //         ("s", "skip"),
        //     //         ("l", "lookup"),
        //     //         ("d", "delete"),
        //     //     ],
        //     //     Some("a"),
        //     // );

        //     // match choice {
        //     //     "a" => {
        //     //         println!("Accepted...");
        //     //     }
        //     //     "s" => {
        //     //         println!("Skipping...");
        //     //     }
        //     //     "l" => {
        //     //         lookup(&input, &imdb, &mut entry);
        //     //     }
        //     //     "d" => {
        //     //         println!("Deleting...");
        //     //     }
        //     //     _ => unreachable!(),
        //     // }

        //     println!();
        // }
    }

    Ok(())
}

fn main() {
    if let Err(e) = foo() {
        println!("{}", e);
        println!("{}", e.backtrace());
    }
}
