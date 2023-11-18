use anyhow::anyhow;
use colored::Colorize;
use std::env;

use dict_cc_lookup::query;

fn main() -> anyhow::Result<()> {
    let query: query::Query = env::args().skip(1).collect::<Vec<String>>().try_into()?;
    let word = match query {
        query::Query::Gender(word) => word,
        _ => return Err(anyhow!("unsupported query")),
    };

    let dict_archive = zstd::stream::read::Decoder::new(&include_bytes!("dict.txt.zst")[..])?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .flexible(true)
        .from_reader(dict_archive);

    for rec in reader.records() {
        let cols: [_; 4] = match rec?
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
            .try_into()
        {
            Ok(cols) => cols,
            Err(_) => continue,
        };

        let parts: Vec<_> = cols[0].split_whitespace().collect();
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
