use anyhow::anyhow;
use std::{
    env,
    io::{BufRead, BufReader},
};

use dict_cc_lookup::{
    entry::Term,
    query::{self, Language},
    util,
};

fn main() -> anyhow::Result<()> {
    let bytes = include_bytes!("dict.txt.zst");
    let buf = BufReader::new(zstd::stream::read::Decoder::with_buffer(&bytes[..])?);

    let query: query::Query = env::args().skip(1).collect::<Vec<String>>().try_into()?;
    match query {
        query::Query::Gender(word) => gender_command(&word, buf),
        query::Query::Meaning {
            language,
            components,
            verbose: false,
        } if components.len() == 1 => {
            meaning_command(&components[0], buf, language == Language::English)
        }
        _ => Err(anyhow!("unsupported query")),
    }
}

fn gender_command(word: &str, mut rd: impl BufRead) -> anyhow::Result<()> {
    let mut buf = String::with_capacity(512);

    loop {
        buf.clear();
        if rd.read_line(&mut buf)? == 0 {
            return Err(anyhow!("not found"));
        }

        let input = match buf.split('\t').next() {
            Some(i) => i,
            None => continue,
        };

        let mut parts = input.split_ascii_whitespace();
        if !parts.any(|v| v == word) {
            continue;
        }

        let (gender_text, is_plural) = match input.split_ascii_whitespace().next_back().unwrap() {
            "{m}" => ("der", false),
            "{f}" => ("die", false),
            "{n}" => ("das", false),
            "{pl}" | "{pl.}" => ("die", true),
            _ => {
                continue;
            }
        };

        print!(
            "{} {}{}",
            gender_text,
            word,
            if is_plural { " (pl)" } else { "" }
        );

        return Ok(());
    }
}

fn meaning_command(word: &str, mut rd: impl BufRead, match_english: bool) -> anyhow::Result<()> {
    let mut buf = String::with_capacity(512);

    loop {
        buf.clear();
        if rd.read_line(&mut buf)? == 0 {
            return Ok(());
        }

        let mut components = buf.split('\t');
        let german_input = match components.next() {
            Some(i) => i,
            None => continue,
        };
        let english_input = match components.next() {
            Some(i) => i,
            None => continue,
        };

        let maybe_match = (!match_english && util::case_fold_contains(german_input, word))
            || (match_english && util::case_fold_contains(english_input, word));
        if !maybe_match {
            continue;
        }

        let german = Term::parse(german_input)?;
        let english = Term::parse(english_input)?;
        let exact_match = (!match_english && german.match_exact(word))
            || (match_english && english.match_exact(word));
        if !exact_match {
            continue;
        }

        let grammar_info = components.next().unwrap();

        println!(
            "{} = {}{}",
            german,
            english,
            if grammar_info.is_empty() {
                "".to_string()
            } else {
                format!("  [{}]", grammar_info)
            }
        );
    }
}
