use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::str::FromStr;

use bincode;
use csv::ReaderBuilder;
use failure::Error;
use flate2::{read::GzDecoder, write::GzEncoder};
use strsim;
use util::NonNan;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum TitleKind {
    Movie,
    TvMovie,
    Video,
    Short,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Title {
    id: u32,
    year: u16,
    runtime: u16,
    primary_title: String,
    original_title: Option<String>,
    kind: TitleKind,
    votes: u32,
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

impl Hash for Title {
    #[inline]
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.id.hash(hasher)
    }
}

impl PartialEq for Title {
    #[inline]
    fn eq(&self, other: &Title) -> bool {
        self.id == other.id
    }
}

impl Eq for Title {}

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

fn read_votes(path: impl AsRef<Path>) -> Result<HashMap<u32, u32>, Error> {
    let file = File::open(path)?;
    let decompressor = GzDecoder::new(file);
    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .delimiter(b'\t')
        .quoting(false)
        .from_reader(decompressor);

    let mut votes_table = HashMap::new();

    for record in reader.records() {
        let record = record?;

        let id: u32 = record[0][2..].parse()?;
        //let score = record[1].parse()?;
        let votes = record[2].parse()?;

        // 50 is a totally arbitrary cutoff for the number of votes
        if votes >= 50 {
            votes_table.insert(id, votes);
        }
    }

    Ok(votes_table)
}

fn read_titles(
    path: impl AsRef<Path>,
    votes_table: &HashMap<u32, u32>,
) -> Result<HashMap<u32, Title>, Error> {
    let file = File::open(path)?;
    let decompressor = GzDecoder::new(file);
    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .delimiter(b'\t')
        .quoting(false)
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
            // skip titles with no votes
            votes: match votes_table.get(&id) {
                None => continue,
                Some(votes) => *votes,
            },
        };

        titles.insert(id, title);
    }

    titles.shrink_to_fit();
    Ok(titles)
}

// Tag splitter must be a superset of the filter_path function
fn tag_splitter(c: char) -> bool {
    match c {
        c if c.is_whitespace() => true,
        c if c.is_ascii_control() => true,
        '/' | '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*' => true, // from filter_path
        '_' => true,
        '-' => true,
        '.' => true,
        ',' => true,
        '\'' => true,
        '(' => true,
        ')' => true,
        _ => false,
    }
}

fn ignored(tag: &str) -> bool {
    match tag {
        "a" | "an" | "the" | "of" | "in" | "on" | "to" | "t" | "s" => true,
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
    tags.dedup();
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

struct Match<'t> {
    score: NonNan,
    title: &'t Title,
}

#[derive(Deserialize, Serialize)]
pub struct Imdb {
    titles: HashMap<u32, Title>,
    index: HashMap<String, HashSet<u32>>,
}

impl Imdb {
    pub fn create_index(
        titles_path: impl AsRef<Path>,
        ratings_path: impl AsRef<Path>,
    ) -> Result<Imdb, Error> {
        let votes_table = read_votes(ratings_path.as_ref())?;
        let titles = read_titles(titles_path.as_ref(), &votes_table)?;

        let index = build_reverse_index(&titles);
        Ok(Imdb { titles, index })
    }

    pub fn load_index(path: impl AsRef<Path>) -> Result<Imdb, Error> {
        let file = File::open(path)?;
        let decompressor = GzDecoder::new(file);
        let mut imdb: Imdb = bincode::deserialize_from(decompressor)?;

        imdb.titles.shrink_to_fit();
        imdb.index.shrink_to_fit();
        imdb.index
            .values_mut()
            .for_each(|bucket| bucket.shrink_to_fit());

        Ok(imdb)
    }

    pub fn load_or_create_index(
        index_path: impl AsRef<Path>,
        titles_path: impl AsRef<Path>,
        ratings_path: impl AsRef<Path>,
    ) -> Result<Imdb, Error> {
        Ok(match Imdb::load_index(index_path.as_ref()) {
            Ok(imdb) => imdb,
            Err(_) => {
                let imdb = Imdb::create_index(titles_path, ratings_path)?;
                imdb.save(index_path)?;
                imdb
            }
        })
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let file = File::create(path)?;
        let compressor = GzEncoder::new(file, Default::default());
        bincode::serialize_into(compressor, self)?;
        Ok(())
    }

    pub fn lookup(&self, text: &str, year: Option<i32>) -> Option<&Title> {
        let mut tags = Vec::new();
        text_to_tags(&text, &mut tags);

        let scoring_func = |title: &Title| -> NonNan {
            let mut score = match title.original_title() {
                None => strsim::jaro(&title.primary_title().to_lowercase(), text),
                Some(original_title) => f64::max(
                    strsim::jaro(&title.primary_title().to_lowercase(), text),
                    strsim::jaro(&original_title.to_lowercase(), text),
                ),
            };

            score *= match title.kind() {
                TitleKind::Movie => 1.0,
                _ => 0.80,
            };

            NonNan::new(score)
        };

        let mut counter = Counter::new();

        for tag in tags.into_iter() {
            if let Some(title_ids) = self.index.get(&tag) {
                for title_id in title_ids.iter() {
                    let title = &self.titles[title_id];

                    // If we have year information, only keep titles whose year is within one of the target year.
                    if let (Some(year), Some(title_year)) = (year, title.year()) {
                        if (year - title_year).abs() > 1 {
                            continue;
                        }
                    }

                    counter.add(title);
                }
            }
        }

        let mut matches: Vec<_> = counter
            .most_common()
            .into_iter()
            .map(|title| Match {
                score: scoring_func(title),
                title,
            }).collect();

        // sort by score descending
        matches.sort_by_key(|m| Reverse(m.score));

        // this step uses popularity, the best matches with 1% error margin are sorted by popularity
        let mut iterator = matches.into_iter();
        if let Some(best) = iterator.next() {
            let mut candidates = vec![];
            candidates.extend(iterator.take_while(|m| (*best.score - *m.score).abs() <= 0.01));
            candidates.push(best);
            candidates.sort_by_key(|m| Reverse(m.title.votes));
            candidates.into_iter().map(|m| m.title).next()
        } else {
            None
        }
    }
}

#[derive(Debug)]
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
