use std::collections::HashSet;
use std::path::{Path, PathBuf};

use failure::Error;
use walkdir::{DirEntry, WalkDir};

use imdb::{Imdb, Title};
use parse::parse_movie;

lazy_static! {
    pub static ref VIDEO_EXT: HashSet<&'static str> = hashset!{
        "mkv",
        "mp4",
        "avi",
        "m4v",
        "webm",
        "flv",
        "vob",
        "mov",
        "wmv",
        "ogv",
        "ogg",
    };
    pub static ref SUBTITLE_EXT: HashSet<&'static str> = hashset!{
        "srt",
        "sub",
        "idx",
        "usf",
        "smi",
    };
}

fn filter_filename(source: &str) -> String {
    let mut dest = String::with_capacity(source.len());
    for car in source.chars() {
        dest.push(match car {
            '/' | '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_ascii_control() => '_',
            _ => car,
        });
    }
    let tlen = dest.trim_right_matches(&[' ', '.'][..]).len();
    dest.truncate(tlen);
    dest
}

fn skip_garbage(entry: &DirEntry) -> bool {
    if let Some(filename) = entry.file_name().to_str() {
        let filename = filename.to_lowercase();
        if entry.file_type().is_dir() {
            return !filename.contains("extras") && !filename.contains("samples");
        } else if entry.file_type().is_file() {
            return !filename.contains("sample")
                && !filename.contains("extra")
                && !filename.contains("rarbg");
        }
    }
    true
}

fn path_for_title(title: &Title, root: &Path, ext: &str) -> PathBuf {
    let mut new_path = root.to_path_buf();
    new_path.push(filter_filename(&format!(
        "{} ({})",
        title.primary_title(),
        title.year().unwrap(),
    )));
    new_path.push(filter_filename(&format!(
        "{} ({}).{}",
        title.primary_title(),
        title.year().unwrap(),
        ext,
    )));
    new_path
}

#[derive(Debug)]
pub struct ScanEntry<'e> {
    pub path: PathBuf,
    pub title: &'e Title,
    pub new_path: PathBuf,
}

#[allow(dead_code)]
pub fn scan_root<'e, A>(root: A, movie_db: &'e Imdb) -> Result<Vec<ScanEntry<'e>>, Error>
where
    A: AsRef<Path>,
{
    let mut scan_entires = vec![];
    let walker = WalkDir::new(root.as_ref());

    for entry in walker.into_iter().filter_entry(skip_garbage) {
        let entry = entry?;
        if let (Some(stem), Some(ext)) = (
            entry.path().file_stem().and_then(|s| s.to_str()),
            entry.path().extension().and_then(|s| s.to_str()),
        ) {
            if VIDEO_EXT.contains(ext) {
                let (name, year) = parse_movie(stem);
                if let Some(title) = movie_db.lookup(&name, year) {
                    scan_entires.push(ScanEntry {
                        path: entry.path().to_path_buf(),
                        new_path: path_for_title(title, root.as_ref(), ext),
                        title,
                    });
                }
            }
        }
    }

    Ok(scan_entires)
}
