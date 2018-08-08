use std::cmp::Ordering;
use std::ops::Deref;
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

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct NonNan(f64);

impl NonNan {
    pub fn new(val: f64) -> NonNan {
        if val.is_nan() {
            panic!("NonNan created with NaN value");
        }
        NonNan(val)
    }
}

impl Eq for NonNan {}

impl Ord for NonNan {
    #[inline]
    fn cmp(&self, other: &NonNan) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Deref for NonNan {
    type Target = f64;

    #[inline]
    fn deref(&self) -> &f64 {
        &self.0
    }
}
