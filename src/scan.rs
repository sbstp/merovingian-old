use std::collections::{HashMap, HashSet};

use failure::Error;

use imdb::{Imdb, Title};
use parse::{parse_movie, tokenize_filename};
use vfs::File;

lazy_static! {
    static ref VIDEO_EXT: HashSet<&'static str> = hashset!{
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
    static ref SUBTITLE_EXT: HashSet<&'static str> = hashset!{
        "srt",
        "sub",
        "idx",
        "usf",
        "smi",
    };
    static ref DIRECTORY_FLAG: HashSet<&'static str> = hashset!{
        "extras",
        "features",
        "samples",
    };
    static ref FILE_FLAG: HashSet<&'static str> = hashset!{
        "trailer",
        "sample",
        "extra",
        "rarbg",
        "etrg",
    };
}

const FILE_MIN_SIZE: u64 = 650 * 1024 * 1024; // 650MB

pub trait FileExt {
    fn is_video(&self) -> bool;
    fn is_subtitle(&self) -> bool;
}

impl FileExt for File {
    fn is_video(&self) -> bool {
        self.is_file() && self
            .extension()
            .map(|ext| VIDEO_EXT.contains(ext))
            .unwrap_or(false)
    }

    fn is_subtitle(&self) -> bool {
        self.is_file() && self
            .extension()
            .map(|ext| SUBTITLE_EXT.contains(ext))
            .unwrap_or(false)
    }
}

#[derive(Debug)]
pub struct ScanEntry<'e> {
    pub movie: File,
    pub title: &'e Title,
    pub images: Vec<File>,
    pub subtitles: Vec<File>,
}

pub struct Scanner<'i> {
    root: File,
    imdb: &'i Imdb,
    is_flagged_cache: HashMap<File, bool>,
    is_movie_cache: HashMap<File, bool>,
}

impl<'i> Scanner<'i> {
    pub fn new(root: File, imdb: &Imdb) -> Scanner {
        Scanner {
            root,
            imdb,
            is_flagged_cache: HashMap::new(),
            is_movie_cache: HashMap::new(),
        }
    }

    fn is_flagged_dir(&mut self, dir: &File) -> bool {
        *self.is_flagged_cache.entry(dir.clone()).or_insert_with(|| {
            let tokens = tokenize_filename(dir.name());
            tokens.iter().any(|t| DIRECTORY_FLAG.contains(t.as_str()))
        })
    }

    fn is_movie_file(&mut self, file: &File) -> bool {
        match self.is_movie_cache.get(file) {
            Some(is_movie) => *is_movie,
            None => {
                let is_garbage = {
                    let tokens = tokenize_filename(file.stem());
                    let parent_flagged = file
                        .parent()
                        .map(|p| self.is_flagged_dir(&p))
                        .unwrap_or(false);
                    let has_token = tokens.iter().any(|t| FILE_FLAG.contains(t.as_str()));
                    let is_small = file.metadata().len() <= FILE_MIN_SIZE;

                    (parent_flagged && (has_token || is_small)) || (has_token && is_small)
                };
                let is_movie = file.is_video() && !is_garbage;
                self.is_movie_cache.insert(file.clone(), is_movie);
                is_movie
            }
        }
    }

    pub fn scan_root(&mut self) -> Result<Vec<ScanEntry<'i>>, Error> {
        let mut scan_entries = vec![];

        for entry in self.root.descendants() {
            if self.is_movie_file(&entry) {
                let stem = entry.stem();
                let (name, year) = parse_movie(stem);
                if let Some(title) = self.imdb.lookup(&name, year) {
                    scan_entries.push(ScanEntry {
                        movie: entry.clone(),
                        title,
                        images: self.scan_images(&entry),
                        subtitles: self.scan_subtitles(&entry, stem),
                    });
                }
            }
        }
        Ok(scan_entries)
    }

    fn scan_images(&self, movie_file: &File) -> Vec<File> {
        let mut images = Vec::new();
        if let Some(siblings) = movie_file.siblings() {
            for entry in siblings {
                if entry.name() == "backdrop.jpg" {
                    images.push(entry);
                } else if entry.name() == "poster.jpg" {
                    images.push(entry);
                }
            }
        }
        images
    }

    fn scan_subtitles(&mut self, movie_file: &File, movie_stem: &str) -> Vec<File> {
        let mut subtitles = Vec::new();
        let mut movies_in_folder = 0;

        // Scan for files whose stem starts with the movie file's stem.
        if let Some(siblings) = movie_file.siblings() {
            for entry in siblings {
                if self.is_movie_file(&entry) {
                    movies_in_folder += 1;
                }

                if entry.is_subtitle() && entry.name().starts_with(movie_stem) {
                    subtitles.push(entry.clone());
                }
            }
        }

        // Scan for subtitles in subfolders called "subs" or "subtitles", but only if
        // the directory contains a single movie file. Since we count the movie files
        // within the siblings of the original movie file, 0 movies means there's just
        // the original movie file.
        if movies_in_folder == 0 {
            if let Some(siblings) = movie_file.siblings() {
                for entry in siblings.filter(|f| {
                    f.is_dir() && (f.name_contains("subs") || f.name_contains("subtitles"))
                }) {
                    subtitles.extend(entry.children().filter(FileExt::is_subtitle));
                }
            }
        }

        subtitles
    }
}
