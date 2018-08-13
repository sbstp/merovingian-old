#![feature(nll)]

#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate maplit;
extern crate same_file;
#[macro_use]
extern crate structopt;
extern crate yansi;

extern crate ffprobe;
extern crate imdb;

mod input;
mod parse;
mod rename;
mod scan;
mod util;
mod vfs;

use std::fs;

use failure::Error;
use structopt::StructOpt;
use yansi::Paint;

use imdb::Imdb;
use rename::{Cleaner, Renames};
use scan::Scanner;
use util::format_runtime;

#[derive(Debug, StructOpt)]
struct App {
    /// Path to the directory containing movies.
    path: Option<String>,
    /// Apply the changes.
    #[structopt(short = "a", long = "--apply")]
    apply: bool,
}

fn foo() -> Result<(), Error> {
    let args = App::from_args();

    let imdb = Imdb::load_or_create_index(".merovingian")?;

    println!("Index contains {} titles.", imdb.len());
    println!("Scanning folder...");

    let root_path = fs::canonicalize(args.path.as_ref().map(|s| s.as_str()).unwrap_or("."))
        .expect("unable to canonicalize root path");
    let root = vfs::walk(&root_path)?;
    let mut entries = Scanner::new(&root, &imdb).scan_root()?;
    let mut cleaner = Cleaner::new();

    println!("Scan found {} movies.", entries.len());
    println!();

    for entry in entries.iter_mut() {
        cleaner.mark(&entry);
        let renames = Renames::new(&root_path, &entry);

        if !renames.is_empty() {
            println!("\tFile: {}", Paint::yellow(entry.movie.name()));
            println!(
                "\tMatch: {} ({}) | {}",
                Paint::yellow(format!(
                    "{} ({})",
                    entry.title.primary_title(),
                    entry.title.year()
                )).underline(),
                format_runtime(entry.title.runtime()),
                Paint::new(format!("https://imdb.com/title/tt{:07}/", entry.title.id()))
                    .underline(),
            );

            println!();

            for rename in renames.iter() {
                println!(
                    "{}",
                    Paint::red(rename.orig().strip_prefix(&root_path).unwrap().display())
                );
            }
            for rename in renames.iter() {
                println!(
                    "{}",
                    Paint::green(rename.renamed().strip_prefix(&root_path).unwrap().display())
                );
            }

            if args.apply {
                if let Err(err) = renames.apply() {
                    println!("=> Could not rename movie: {}", err);
                }
            }

            println!();
        }
    }

    println!("Files that will be removed:");

    for file in root.descendants() {
        if file.is_file() && !cleaner.is_marked(&file) {
            println!("{}", Paint::red(file.path().display()));
            if args.apply {
                if let Err(err) = fs::remove_file(file.path()) {
                    println!("=> Could not remove {}: {}", file.path().display(), err);
                }
            }
        }
    }

    // Remove all the empty directories.
    if args.apply {
        for file in root.descendants() {
            if file.is_dir() {
                //println!("Trying to remove {}", file.path().display());
                let _ = fs::remove_dir(file.path());
            }
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
