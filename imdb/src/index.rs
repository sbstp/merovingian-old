use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fs::{DirBuilder, File};
use std::path::Path;
use std::str::FromStr;

use bincode;
use csv::ReaderBuilder;
use flate2::{read::GzDecoder, write::GzEncoder};
use reqwest::Client;
use strsim;

use error::Result;
use title::{Title, TitleKind};
use util::{Counter, NonNan};

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

fn read_votes(path: impl AsRef<Path>) -> Result<HashMap<u32, u32>> {
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
) -> Result<HashMap<u32, Title>> {
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

        if year == 0 || runtime == 0 {
            continue;
        }

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
                    .insert(title.id());
            }
        };

        index_title(title.primary_title());
        if let Some(original_title) = title.original_title() {
            if title.primary_title() != original_title {
                index_title(&original_title);
            }
        }
    }

    index.shrink_to_fit();
    index.values_mut().for_each(|bucket| bucket.shrink_to_fit());

    index
}

fn download_file(client: &Client, url: &str, dest: impl AsRef<Path>) -> Result<()> {
    let mut file = File::create(dest)?;
    let mut resp = client.get(url).send()?;
    resp.copy_to(&mut file)?;
    Ok(())
}

fn download_file_if_missing(client: &Client, url: &str, dest: impl AsRef<Path>) -> Result<()> {
    if !dest.as_ref().exists() {
        download_file(client, url, dest)?;
    }
    Ok(())
}

const SRC_FILE_BASICS: &str = "title.basics.tsv.gz";
const SRC_FILE_RATINGS: &str = "title.ratings.tsv.gz";

fn check_source_files(index_dir: &Path) -> Result<()> {
    let client = Client::new();

    download_file_if_missing(
        &client,
        "https://datasets.imdbws.com/title.basics.tsv.gz",
        index_dir.join(SRC_FILE_BASICS),
    )?;

    download_file_if_missing(
        &client,
        "https://datasets.imdbws.com/title.ratings.tsv.gz",
        index_dir.join(SRC_FILE_RATINGS),
    )?;

    Ok(())
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
    pub fn create_index(index_dir: &Path) -> Result<Imdb> {
        let votes_table = read_votes(index_dir.join(SRC_FILE_RATINGS))?;
        let titles = read_titles(index_dir.join(SRC_FILE_BASICS), &votes_table)?;

        let index = build_reverse_index(&titles);
        Ok(Imdb { titles, index })
    }

    pub fn load_index(path: impl AsRef<Path>) -> Result<Imdb> {
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

    pub fn load_or_create_index(index_dir: impl AsRef<Path>) -> Result<Imdb> {
        let index_dir = index_dir.as_ref();
        let index_path = index_dir.join("index.gz");

        DirBuilder::new().recursive(true).create(index_dir)?;
        check_source_files(index_dir)?;

        Ok(match Imdb::load_index(&index_path) {
            Ok(imdb) => imdb,
            Err(_) => {
                let imdb = Imdb::create_index(index_dir)?;
                imdb.save(&index_path)?;
                imdb
            }
        })
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
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

            if let Some(year) = year {
                if title.year() != year {
                    score *= 0.85;
                }
            }

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
                    if let Some(year) = year {
                        if (year - title.year()).abs() > 1 {
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
            candidates.sort_by_key(|m| Reverse(m.title.votes()));
            candidates.into_iter().map(|m| m.title).next()
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.titles.len()
    }
}
