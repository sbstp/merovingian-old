use std::cmp;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::hash::Hash;
use std::path::Path;
use std::str::FromStr;

use csv::ReaderBuilder;
use failure::Error;
use flate2::read::GzDecoder;
use strsim::levenshtein;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TitleKind {
    Movie,
    TvMovie,
    Video,
    Short,
}

#[derive(Debug, Clone)]
pub struct Title {
    id: u32,
    year: u16,
    runtime: u16,
    primary_title: String,
    original_title: Option<String>,
    kind: TitleKind,
}

impl Title {
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    #[inline]
    pub fn year(&self) -> Option<i32> {
        match self.year {
            0 => None,
            year => Some(year as i32),
        }
    }

    #[inline]
    pub fn runtime(&self) -> Option<i32> {
        match self.year {
            0 => None,
            year => Some(year as i32),
        }
    }

    #[inline]
    pub fn primary_title(&self) -> &str {
        &self.primary_title
    }

    #[inline]
    pub fn original_title(&self) -> Option<&str> {
        self.original_title.as_ref().map(|s| s.as_str())
    }

    #[inline]
    pub fn kind(&self) -> TitleKind {
        self.kind
    }
}

fn parse_none<T: FromStr>(record: &str) -> Option<T> {
    match record {
        "\\N" => None,
        s => s.parse().ok(),
    }
}

macro_rules! some_or_continue {
    ($e:expr) => {
        match $e {
            None => continue,
            Some(x) => x,
        }
    };
}

fn read_titles(path: &str) -> Result<HashMap<u32, Title>, Error> {
    let file = File::open(path)?;
    let decompressor = GzDecoder::new(file);
    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .delimiter(b'\t')
        .from_reader(decompressor);

    let mut titles = HashMap::new();

    for record in reader.records() {
        let record = record?;

        let adult: u8 = some_or_continue!(parse_none(&record[4]));
        if adult == 1 {
            continue;
        }

        let kind = &record[1];

        let kind = match kind {
            "movie" => TitleKind::Movie,
            "tvMovie" => TitleKind::TvMovie,
            "video" => TitleKind::Video,
            "short" => TitleKind::Short,
            _ => continue,
        };

        let year = some_or_continue!(parse_none(&record[5]));
        let runtime = some_or_continue!(parse_none(&record[7]));

        let id = record[0][2..].parse()?;
        let primary_title = &record[2];
        let original_title = &record[3];

        let title = Title {
            id,
            year,
            runtime,
            primary_title: primary_title.to_string(),
            original_title: if primary_title != original_title {
                Some(original_title.to_string())
            } else {
                None
            },
            kind,
        };

        titles.insert(id, title);
    }

    titles.shrink_to_fit();
    Ok(titles)
}

fn tag_splitter(c: char) -> bool {
    match c {
        c if c.is_whitespace() => true,
        '_' => true,
        '-' => true,
        '.' => true,
        ':' => true,
        ',' => true,
        '\'' => true,
        _ => false,
    }
}

fn ignored(tag: &str) -> bool {
    match tag {
        "a" | "an" | "the" | "of" | "in" | "to" | "t" | "s" => true,
        _ => false,
    }
}

fn text_to_tags(text: &str, tags: &mut Vec<String>) {
    let text = text.to_lowercase();
    tags.clear();
    for tag in text.split(tag_splitter) {
        if !tag.is_empty() && !ignored(tag) {
            tags.push(tag.to_string());
        }
    }
}

fn build_reverse_index(titles: &HashMap<u32, Title>) -> HashMap<String, HashSet<u32>> {
    let mut index = HashMap::new();
    let mut tags = Vec::new();

    for title in titles.values() {
        let mut index_title = |text: &str| {
            text_to_tags(&text, &mut tags);
            for tag in tags.drain(..) {
                index
                    .entry(tag)
                    .or_insert_with(|| HashSet::new())
                    .insert(title.id);
            }
        };

        index_title(&title.primary_title);
        if let Some(original_title) = &title.original_title {
            if &title.primary_title != original_title {
                index_title(&original_title);
            }
        }
    }

    index.shrink_to_fit();
    index.values_mut().for_each(|bucket| bucket.shrink_to_fit());

    index
}

pub struct Imdb {
    titles: HashMap<u32, Title>,
    index: HashMap<String, HashSet<u32>>,
}

impl Imdb {
    pub fn create_index(path: &str) -> Result<Imdb, Error> {
        let titles = read_titles(path)?;
        let index = build_reverse_index(&titles);
        Ok(Imdb { titles, index })
    }

    pub fn lookup(&self, text: &str, year: Option<i32>) -> Option<&Title> {
        let mut tags = Vec::new();
        text_to_tags(&text, &mut tags);

        // find the titles that matched the most tags
        let mut titles: Counter<u32> = Counter::new();

        for tag in tags.into_iter() {
            if let Some(index_titles) = self.index.get(&tag) {
                for index_title in index_titles {
                    titles.add(*index_title);
                }
            }
        }

        let mut most_common = titles.most_common();

        // filter on year when possible
        if let Some(year) = year {
            most_common.retain(|&index| {
                let t = &self.titles[index];
                if let Some(title_year) = t.year() {
                    if (title_year - year).abs() > 1 {
                        return false;
                    }
                }
                true
            });
        }

        let diff = |title: &Title| {
            let primary_lev = levenshtein(&title.primary_title, text);
            if let Some(original_title) = &title.original_title {
                cmp::min(primary_lev, levenshtein(&original_title, text))
            } else {
                primary_lev
            }
        };

        // find the best matches from the pool of candidates
        let mut best_matches = Vec::new();
        let mut best_match = None;

        for candidate in most_common {
            let title = &self.titles[candidate];
            let d = diff(title);
            match best_match {
                None => {
                    best_match = Some(d);
                    best_matches.push(title);
                }
                Some(bm) if d == bm => {
                    best_matches.push(title);
                }
                Some(bm) if d < bm => {
                    best_match = Some(d);
                    best_matches.clear();
                    best_matches.push(title);
                }
                _ => {}
            }
        }

        // pick the best kind from the best matches
        best_matches.sort_by(|left, right| left.kind().cmp(&right.kind()));
        best_matches.into_iter().next()
    }
}

struct Counter<K: Hash + Eq> {
    inner: HashMap<K, u32>,
}

impl<K> Counter<K>
where
    K: Hash + Eq,
{
    pub fn new() -> Counter<K> {
        Counter {
            inner: HashMap::new(),
        }
    }

    pub fn add(&mut self, key: K) {
        *self.inner.entry(key).or_insert(0) += 1;
    }

    pub fn most_common(&self) -> Vec<&K> {
        let mut most_common = Vec::new();
        let mut most_count = 0;
        for (key, &count) in self.inner.iter() {
            if count == most_count {
                most_common.push(key);
            } else if count >= most_count {
                most_common.clear();
                most_common.push(key);
                most_count = count;
            }
        }
        most_common
    }
}

#[test]
fn test_most_common() {
    let mut c = Counter::new();
    c.add("hello");
    assert_eq!(c.most_common(), vec![&"hello"]);
    c.add("hey");
    assert_eq!(c.most_common().len(), 2);
    c.add("hello");
    assert_eq!(c.most_common(), vec![&"hello"]);
}
