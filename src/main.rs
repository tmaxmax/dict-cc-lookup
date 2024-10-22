use anyhow::anyhow;
use std::{
    env,
    io::{self, BufRead, BufReader, Write},
};

use dict_cc_lookup::{
    entry::Term,
    lexer,
    query::{self, Language},
    util,
};

fn main() -> anyhow::Result<()> {
    let dict = include_bytes!("dict.txt.zst");
    let buf = BufReader::new(zstd::stream::read::Decoder::with_buffer(&dict[..])?);

    let res: Result<query::Query, _> = env::args().skip(1).collect::<Vec<String>>().try_into();

    match res {
        Ok(query) => match query {
            query::Query::Gender(word) => gender_command(&word, buf),
            query::Query::Meaning {
                language,
                components,
                verbose: false,
            } if components.len() == 1 => {
                meaning_command(&components[0], buf, language == Language::English)
            }
            query::Query::Interactive {
                language: query::Language::German,
            } => interactive_command(buf),
            _ => Err(anyhow!("unsupported query")),
        },
        Err(_) => lex_command(buf),
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

fn lex_command(mut rd: impl BufRead) -> anyhow::Result<()> {
    let mut buf = String::with_capacity(512);
    let mut i = 0;

    loop {
        buf.clear();
        if rd.read_line(&mut buf)? == 0 {
            return Ok(());
        }

        if i < 9 {
            i += 1;
            continue;
        }

        println!("{buf:?}");
        buf.split('\t')
            .take(2)
            .map(|v| (v, lexer::lex(v)))
            .for_each(|(v, tokens)| {
                println!(
                    "{:?}",
                    tokens
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| match e {
                            lexer::Error::Unexpected(at) =>
                                format!("unexpected value at \"{}\"", &v[at..]),
                            e => e.to_string(),
                        })
                        .unwrap()
                )
            });
        println!();
    }
}

struct Entry {
    german: Term,
    english: Term,
    grammar_info: String,
}

fn interactive_command(mut rd: impl BufRead) -> anyhow::Result<()> {
    println!("dict.cc in command line");

    let mut buf = String::with_capacity(512);
    let mut entries = Vec::<Entry>::new();

    loop {
        buf.clear();
        if rd.read_line(&mut buf)? == 0 {
            break;
        }

        let mut components = buf.split('\t');
        let Ok(german) = components
            .next()
            .ok_or_else(|| anyhow!("no german component"))
            .and_then(Term::parse) else {
                continue
            };
        let Ok(english) = components
            .next()
            .ok_or_else(|| anyhow!("no english component"))
            .and_then(Term::parse) else {
                continue
            };
        let Some(grammar_info) = components.next().map(|s| s.to_string()) else {
            continue
        };

        entries.push(Entry {
            german,
            english,
            grammar_info,
        });
    }

    println!("Input German words:");

    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    loop {
        write!(stdout, "> ")?;
        stdout.flush()?;

        buf.clear();
        if stdin.read_line(&mut buf)? == 0 {
            return Ok(());
        }

        let matches = entries.iter().filter(|&e| e.german.match_exact(buf.trim()));

        for entry in matches {
            writeln!(
                stdout,
                "{} = {}{}",
                entry.german,
                entry.english,
                if entry.grammar_info.is_empty() {
                    "".to_string()
                } else {
                    format!("  [{}]", entry.grammar_info)
                }
            )?;
        }

        stdout.flush()?;
    }
}
