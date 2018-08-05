use std::collections::{HashMap, HashSet};
use std::fs::Metadata;
use std::path::{Path, PathBuf};

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
    };
}

const FILE_MIN_SIZE: u64 = 650 * 1024 * 1024; // 650MB

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

#[derive(Debug)]
pub struct Rename {
    pub file: File,
    pub new: PathBuf,
}

impl Rename {
    fn new(file: File, base_path: &Path, name: &str) -> Rename {
        Rename {
            file: file,
            new: base_path.join(filter_filename(name)),
        }
    }

    pub fn different(&self) -> bool {
        self.file.path() != self.new
    }
}

// #[derive(Debug)]
// pub enum Image {
//     Backdrop(File),
//     Poster(File),
// }

// #[derive(Debug)]
// pub struct Subtitle {
//     pub sub: Rename,
// }

#[derive(Debug)]
pub struct ScanEntry<'e> {
    pub movie: Rename,
    pub title: &'e Title,
    pub images: Vec<Rename>,
    pub subtitles: Vec<Rename>,
}

fn is_movie_file(ext: &str, metadata: &Metadata, tokens: &[String], parent_flagged: bool) -> bool {
    let has_token = tokens.iter().any(|t| FILE_FLAG.contains(t.as_str()));
    let is_small = metadata.len() <= FILE_MIN_SIZE;
    let is_garbage = (parent_flagged && (has_token || is_small)) || (has_token && is_small);
    VIDEO_EXT.contains(ext) && !is_garbage
}

pub fn scan_root<'i>(root: &File, imdb: &'i Imdb) -> Result<Vec<ScanEntry<'i>>, Error> {
    let mut scan_entries = vec![];
    let mut flagged_cache: HashMap<File, bool> = HashMap::new();

    for entry in root.descendants() {
        if let Some(stem) = entry.stem() {
            let tokens = tokenize_filename(stem);

            if entry.is_dir() {
                flagged_cache.insert(
                    entry.clone(),
                    tokens.iter().any(|t| DIRECTORY_FLAG.contains(t.as_str())),
                );
            }

            if let Some(ext) = entry.extension() {
                let parent_flagged = entry
                    .parent()
                    .and_then(|p| flagged_cache.get(&p))
                    .map(|f| *f)
                    .unwrap_or(false);

                if is_movie_file(ext, entry.metadata(), &tokens, parent_flagged) {
                    let (name, year) = parse_movie(stem);
                    if let Some(title) = imdb.lookup(&name, year) {
                        let base_path = root.path().join(filter_filename(&format!(
                            "{} ({})",
                            title.primary_title(),
                            title.year().unwrap()
                        )));
                        scan_entries.push(ScanEntry {
                            movie: Rename::new(
                                entry.clone(),
                                &base_path,
                                &format!(
                                    "{} ({}).{}",
                                    title.primary_title(),
                                    title.year().unwrap(),
                                    ext
                                ),
                            ),
                            title,
                            images: scan_images(&entry, &base_path),
                            subtitles: scan_subtitles(&entry, &base_path),
                        });
                    }
                }
            }
        }
    }
    Ok(scan_entries)
}

// fn scan_rec<'i>(
//     root: &Path,
//     path: &Path,
//     imdb: &'i Imdb,
//     scan_entries: &mut Vec<ScanEntry<'i>>,
//     parent_flagged: bool,
// ) -> Result<(), Error> {
//     for entry in path.read_dir()? {
//         let entry = entry?;
//         let path = entry.path();
//         let ftype = entry.file_type()?;
//         let metadata = entry.metadata()?;

//         if let Some(stem) = path.file_stem().and_then(OsStr::to_str) {
//             let tokens = tokenize_filename(stem);
//             if ftype.is_dir() {
//                 let flagged = tokens.iter().any(|t| DIRECTORY_FLAG.contains(t.as_str()));
//                 scan_rec(root, &path, imdb, scan_entries, flagged)?;
//             } else if ftype.is_file() {
//                 if let Some(ext) = path.extension().and_then(OsStr::to_str) {
//                     if is_movie_file(ext, metadata, &tokens, parent_flagged) {
//                         let (name, year) = parse_movie(stem);
//                         if let Some(title) = imdb.lookup(&name, year) {
//                             scan_entries.push(ScanEntry {
//                                 path: path.clone(),
//                                 new_path: path_for_title(title, root, ext),
//                                 title,
//                                 images: scan_images(&path),
//                                 subtitles: scan_subtitles(&path).unwrap(),
//                             });
//                         }
//                     }
//                 }
//             }
//         }
//     }
//     Ok(())
// }

fn scan_images(movie_file: &File, base_path: &Path) -> Vec<Rename> {
    let mut images = Vec::new();
    if let Some(siblings) = movie_file.siblings() {
        for entry in siblings {
            if entry.file_name() == Some("backdrop.jpg") {
                images.push(Rename::new(entry, base_path, "backdrop.jpg"));
            } else if entry.file_name() == Some("poster.jpg") {
                images.push(Rename::new(entry, base_path, "poster.jpg"));
            }
        }
    }
    images
}

fn scan_subtitles(movie_file: &File, base_path: &Path) -> Vec<Rename> {
    let mut subtitles = Vec::new();
    if let Some(parent) = movie_file.parent() {
        subtitles.extend(
            parent
                .descendants()
                .filter(|f| {
                    f.extension()
                        .map(|ext| SUBTITLE_EXT.contains(ext))
                        .unwrap_or(false)
                }).map(|f| Rename {
                    new: f.path().to_owned(),
                    file: f,
                }),
        );
    }
    subtitles
}
