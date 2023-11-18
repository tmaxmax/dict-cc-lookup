use anyhow::anyhow;
use colored::Colorize;
use std::env;

use dict_cc_lookup::{
    entry::Term,
    query::{self, Language},
};

fn main() -> anyhow::Result<()> {
    let dict_archive = zstd::stream::read::Decoder::new(&include_bytes!("dict.txt.zst")[..])?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .flexible(true)
        .from_reader(dict_archive);

    let iterator = reader
        .records()
        .filter_map(|rec| rec.ok())
        .filter_map(|rec| {
            let cols: Result<[_; 4], _> = rec
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
                .try_into();
            cols.ok()
        });

    let query: query::Query = env::args().skip(1).collect::<Vec<String>>().try_into()?;
    match query {
        query::Query::Gender(word) => gender_command(&word, iterator),
        query::Query::Meaning {
            language,
            components,
            verbose: false,
        } if components.len() == 1 => {
            meaning_command(&components[0], iterator, language == Language::English)
        }
        _ => Err(anyhow!("unsupported query")),
    }
}

fn gender_command(word: &str, iter: impl Iterator<Item = [String; 4]>) -> anyhow::Result<()> {
    for rec in iter {
        let parts: Vec<_> = rec[0].split_whitespace().collect();
        if parts.len() < 2 || !parts.iter().any(|&v| v == word) {
            continue;
        }

        let (gender_text, is_plural) = match *parts.last().unwrap() {
            "{m}" => ("der".bright_blue(), false),
            "{f}" => ("die".bright_magenta(), false),
            "{n}" => ("das".bright_green(), false),
            "{pl}" => ("die".bright_yellow(), true),
            _ => continue,
        };

        print!(
            "{} {}{}",
            gender_text.bold(),
            word,
            if is_plural { " (pl)" } else { "" }
        );

        return Ok(());
    }

    Err(anyhow!("not found"))
}

fn meaning_command(
    word: &str,
    iter: impl Iterator<Item = [String; 4]>,
    match_english: bool,
) -> anyhow::Result<()> {
    for rec in iter {
        let german = Term::parse(&rec[0])?;
        let english = Term::parse(&rec[1])?;

        if (!match_english && german.match_exact(word))
            || (match_english && english.match_exact(word))
        {
            println!("{} = {}", german, english);
        }
    }

    Ok(())
}
