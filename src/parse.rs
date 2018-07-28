use std::cmp;
use std::collections::HashSet;

lazy_static! {
    static ref QUALITY: HashSet<&'static str> = hashset!{
        "2160p",
        "1080p",
        "720p",
        "480p",
        "360p",
        "240p",
    };
    static ref VIDEO_FORMAT: HashSet<&'static str> = hashset!{
        "xvid",
        "divx",
        "h264",
        "x264",
        "h265",
        "x265",
        "10bit",
    };
    static ref AUDIO_FORMAT: HashSet<&'static str> = hashset!{
        "ac3",
        "aac",
        "aac2",
        "dd5",
        "dd2",
    };
    static ref ALL: HashSet<&'static str> = {
        QUALITY
            .iter()
            .chain(VIDEO_FORMAT.iter())
            .chain(AUDIO_FORMAT.iter())
            .cloned()
            .collect()
    };
}

fn parse_filename(name: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut pos = 0;

    for (idx, car) in name.char_indices() {
        match car {
            ' ' | '.' | '_' | '-' | ':' | '(' | ')' | '[' | ']' => {
                let text = &name[pos..idx];
                if !text.is_empty() {
                    tokens.push(text.to_lowercase());
                }

                pos = idx + car.len_utf8();
            }
            _ => {}
        }
    }

    let text = &name[pos..];
    if !text.is_empty() {
        tokens.push(text.to_lowercase());
    }

    tokens
}

fn is_year(token: &str) -> bool {
    return token.len() == 4 && token.chars().all(|c| char::is_digit(c, 10));
}

/// Try to extract title and year from filename.
///
/// Usually, the title is placed before the year. There are cases where the movie's name has a year.
/// In that case, use right most year found is assumed to be the movie's release year. If nothing
/// occurs before the year found, the year is assumed to be the movie's title, such as
/// '2001: A Space Odyssey.mp4'.
///
/// If a metadata token is found, the title is assumed to stop before the metadata token. So the title
/// is everything before the year or the first metadata token.
///
/// There are also cases where a releases' name shows up before the title, such as '[foobar] The Matrix.mp4',
/// everything inside square brackets or parens before any normal word is ignored.
pub fn parse_movie(filename: &str) -> (String, Option<i32>) {
    let tokens = parse_filename(&filename);

    let mut year_candidates = vec![];
    let mut first_metadata_token = None;

    for (idx, token) in tokens.iter().enumerate() {
        if is_year(token) {
            year_candidates.push(idx);
        }
        if first_metadata_token.is_none() {
            if ALL.contains(token.as_str()) {
                first_metadata_token = Some(idx);
            }
        }
    }

    let first_metadata_token = first_metadata_token.unwrap_or(tokens.len());

    let mut title_tokens = &tokens[..first_metadata_token];
    let mut year = None;

    if let Some(&year_idx) = year_candidates.last() {
        let min_idx = cmp::min(year_idx, first_metadata_token);
        let new_title_tokens = &tokens[..min_idx];
        if !new_title_tokens.is_empty() {
            title_tokens = new_title_tokens;
            year = Some(tokens[year_idx].parse().unwrap());
        }
    }

    (
        title_tokens
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" "),
        year,
    )
}

#[test]
fn test_is_year() {
    assert!(is_year("2009"));
    assert!(!is_year("1080p"));
}

#[test]
fn test_split_tokens() {
    assert_eq!(
        parse_filename("this.file_name-uses:every separator"),
        vec!["this", "file", "name", "uses", "every", "separator"]
    );

    assert_eq!(parse_filename("foo.-_ .:bar"), vec!["foo", "bar"]);
}

#[test]
fn test_parse_filename_simple() {
    let tokens = parse_filename("american psycho");
    assert_eq!(tokens, vec!["american", "psycho"]);
}

#[test]
fn test_parse_filename_parens_square() {
    let tokens = parse_filename("American.Psycho.(2000).[1080p]");
    assert_eq!(tokens, vec!["american", "psycho", "2000", "1080p"]);
}

#[test]
fn test_simple() {
    assert_eq!(parse_movie("Groundhog Day"), ("groundhog day".into(), None));
    assert_eq!(parse_movie("Snatch! 2005"), ("snatch!".into(), Some(2005)));
    assert_eq!(
        parse_movie("snatch! (2005)"),
        ("snatch!".into(), Some(2005))
    );
    assert_eq!(
        parse_movie("snatch! [2005]"),
        ("snatch!".into(), Some(2005))
    );
}

#[test]
fn test_ambiguous_year() {
    assert_eq!(parse_movie("2011 1968"), ("2011".into(), Some(1968)));
    assert_eq!(parse_movie("2011"), ("2011".into(), None));
}

#[test]
fn test_metadata() {
    assert_eq!(
        parse_movie("Truman Show 1080p 1998.mkv"),
        ("truman show".into(), Some(1998))
    );
    assert_eq!(
        parse_movie("Truman Show 1080p.mkv"),
        ("truman show".into(), None)
    );
}

#[test]
fn test_year_within_scope() {
    assert_eq!(
        parse_movie("Night Of The Living Dead (1968 - Widescreen)"),
        ("night of the living dead".into(), Some(1968))
    )
}
