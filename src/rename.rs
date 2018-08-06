use std::path::{Path, PathBuf};

use scan::ScanEntry;
use util::PathExt;
use vfs::File;

pub struct Rename<'p> {
    pub orig: &'p Path,
    pub new: PathBuf,
}

impl<'p> Rename<'p> {
    fn new<'a>(orig: &'a Path, new: PathBuf) -> Rename<'a> {
        Rename { orig, new }
    }

    pub fn different(&self) -> bool {
        self.orig != self.new
    }
}

fn format_base<'i, 'e>(entry: &'e ScanEntry<'i>) -> String {
    format!(
        "{} ({})",
        entry.title.primary_title(),
        entry.title.year().unwrap(),
    )
}

fn format_movie<'i, 'e>(entry: &'e ScanEntry<'i>) -> String {
    format!(
        "{} ({}).{}",
        entry.title.primary_title(),
        entry.title.year().unwrap(),
        entry.movie.extension().unwrap(),
    )
}

fn format_subtitle<'i, 'e>(entry: &'e ScanEntry<'i>, file: &File) -> String {
    // Remove the common part between the movie's stem and the subtitle's name.
    let suffix = file.name().trim_left_matches(entry.movie.stem());
    format!(
        "{} ({}){}",
        entry.title.primary_title(),
        entry.title.year().unwrap(),
        suffix
    )
}

pub fn movie<'i, 'e>(root_path: &Path, entry: &'e ScanEntry<'i>) -> Vec<Rename<'e>> {
    let dir_path = root_path.join_filtered(&format_base(entry));

    let mut renames = vec![Rename::new(
        entry.movie.path(),
        dir_path.join_filtered(&format_movie(entry)),
    )];

    // images
    renames.extend(
        entry
            .images
            .iter()
            .map(|f| Rename::new(f.path(), dir_path.join(f.name()))),
    );

    // subtitles
    // TODO: handle languages and duplicates
    renames.extend(
        entry
            .subtitles
            .iter()
            .map(|f| Rename::new(f.path(), dir_path.join_filtered(&format_subtitle(entry, f)))),
    );

    renames
}
