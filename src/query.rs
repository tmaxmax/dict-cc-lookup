use std::ops::Deref;

use anyhow::anyhow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    German,
    English,
}

#[derive(Debug, Clone)]
pub enum Query {
    Gender(String),
    Meaning {
        language: Language,
        components: Vec<String>,
        verbose: bool,
    },
    Interactive {
        language: Language,
    },
}

impl TryFrom<Vec<String>> for Query {
    type Error = anyhow::Error;

    fn try_from(mut value: Vec<String>) -> anyhow::Result<Self> {
        if value.is_empty() {
            return Err(anyhow!("input is empty"));
        }

        let maybe_specifier = value.remove(0);
        if value.is_empty() && maybe_specifier.to_lowercase() != "i" {
            return Ok(Query::Meaning {
                language: Language::German,
                components: maybe_specifier
                    .split_whitespace()
                    .map(String::from)
                    .collect(),
                verbose: false,
            });
        }

        let (language, verbose) = match maybe_specifier.to_lowercase().deref() {
            "g" => {
                let word =
                    to_upper(value.first().unwrap()).ok_or_else(|| anyhow!("empty input"))?;
                return Ok(Query::Gender(word));
            }
            "d" => (Language::German, false),
            "dv" => (Language::German, true),
            "e" => (Language::English, false),
            "ev" => (Language::English, true),
            "i" => {
                return Ok(Query::Interactive {
                    language: Language::German,
                })
            }
            _ => return Err(anyhow!("invalid query specifier \"{}\"", maybe_specifier)),
        };

        let components: Vec<_> = value
            .iter()
            .flat_map(|v| v.split_whitespace())
            .map(String::from)
            .collect();

        Ok(Query::Meaning {
            language,
            components,
            verbose,
        })
    }
}

fn to_upper(s: &str) -> Option<String> {
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().to_string() + chars.as_str())
}
