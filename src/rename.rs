use std::collections::HashSet;
use std::fs::{self, DirBuilder};
use std::io;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use same_file::is_same_file;
use same_file::Handle;

use scan::ScanEntry;
use util::PathExt;
use vfs::File;

pub struct Rename {
    pub orig: File,
    pub renamed: PathBuf,
}

impl Rename {
    fn new(orig: &File, renamed: PathBuf) -> Rename {
        Rename {
            orig: orig.clone(),
            renamed,
        }
    }

    #[inline]
    fn different(&self) -> bool {
        self.orig.path() != self.renamed
    }

    #[inline]
    pub fn orig(&self) -> &Path {
        self.orig.path()
    }

    #[inline]
    pub fn renamed(&self) -> &Path {
        &self.renamed
    }
}

fn format_base<'i, 'e>(entry: &'e ScanEntry<'i>) -> String {
    format!("{} ({})", entry.title.primary_title(), entry.title.year(),)
}

fn format_movie<'i, 'e>(entry: &'e ScanEntry<'i>) -> String {
    format!(
        "{} ({}).{}",
        entry.title.primary_title(),
        entry.title.year(),
        entry.movie.extension().unwrap(),
    )
}

fn format_subtitle<'i, 'e>(entry: &'e ScanEntry<'i>, file: &File) -> String {
    // Remove the common part between the movie's stem and the subtitle's name.
    let suffix = file.name().trim_left_matches(entry.movie.stem());
    format!(
        "{} ({}){}",
        entry.title.primary_title(),
        entry.title.year(),
        suffix
    )
}

fn movie<'i, 'e>(root_path: &Path, entry: &'e ScanEntry<'i>) -> Vec<Rename> {
    let dir_path = root_path.join_filtered(&format_base(entry));

    let mut renames = vec![Rename::new(
        &entry.movie,
        dir_path.join_filtered(&format_movie(entry)),
    )];

    // images
    renames.extend(
        entry
            .images
            .iter()
            .map(|f| Rename::new(f, dir_path.join(f.name()))),
    );

    // subtitles
    // TODO: handle languages and duplicates
    renames.extend(
        entry
            .subtitles
            .iter()
            .map(|f| Rename::new(f, dir_path.join_filtered(&format_subtitle(entry, f)))),
    );

    renames
}

pub struct Renames {
    diff: Vec<Rename>,
}

impl Renames {
    pub fn new<'i>(root_path: impl AsRef<Path>, entry: &ScanEntry<'i>) -> Renames {
        let renames = movie(root_path.as_ref(), &entry);
        Renames {
            diff: renames.into_iter().filter(|r| r.different()).collect(),
        }
    }

    pub fn apply(&self) -> io::Result<()> {
        for item in self.diff.iter() {
            let renamed = item.renamed();
            let new_parent = renamed.parent().expect("renamed path has no parent");
            let old_parent = item.orig.parent().expect("original path has no parent");
            // println!("{:?}", Handle::from_path(old_parent.path()));
            // println!("{:?}", Handle::from_path(new_parent));
            // println!("{}", is_same_file(new_parent, old_parent.path())?);
            // if is_same_file(new_parent, old_parent.path())? {
            //     // When the directory is the same, rename the files first and then rename the directory.
            //     // This issue occurs when the name is different but it still points to the same files.
            //     fs::rename(item.orig(), renamed)?;
            //     println!("here tho");
            //     fs::rename(old_parent.path(), new_parent)?;
            // } else {
            DirBuilder::new().recursive(true).create(new_parent)?;
            fs::rename(item.orig(), renamed)?;
            // }
        }
        Ok(())
    }
}

impl Deref for Renames {
    type Target = [Rename];

    #[inline]
    fn deref(&self) -> &[Rename] {
        &self.diff
    }
}

pub struct Cleaner {
    marked_files: HashSet<File>,
}

impl Cleaner {
    pub fn new() -> Cleaner {
        Cleaner {
            marked_files: HashSet::new(),
        }
    }

    pub fn mark<'i>(&mut self, entry: &ScanEntry<'i>) {
        self.marked_files.insert(entry.movie.clone());
        self.marked_files.extend(entry.images.iter().cloned());
        self.marked_files.extend(entry.subtitles.iter().cloned());
    }

    #[inline]
    pub fn is_marked(&self, file: &File) -> bool {
        self.marked_files.contains(file)
    }
}
