use std::path::{Path, PathBuf};

pub fn filter_path(source: &str) -> String {
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

pub trait PathExt {
    fn join_filtered(&self, segment: &str) -> PathBuf;
}

impl PathExt for Path {
    fn join_filtered(&self, segment: &str) -> PathBuf {
        self.join(filter_path(segment))
    }
}
