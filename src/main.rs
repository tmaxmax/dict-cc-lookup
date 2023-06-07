use anyhow::{anyhow, Result};
use std::env;

fn main() -> Result<()> {
    let filepath = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("File path argument not provided"))?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .flexible(true)
        .from_path(filepath)?;

    let mut previous = String::new();
    let mut meanings = Vec::<String>::new();

    fn show(word: &str, meanings: &[String]) {
        println!("{} = {}", word, meanings.join(", "))
    }

    for res in reader.records() {
        let record = res?;
        let first: Vec<_> = record.get(0).unwrap().split_whitespace().collect();
        let gender = first.last().cloned().unwrap();
        if !first.iter().any(|v| v.ends_with("gefühl")) || !gender.contains('{') || first.len() != 2
        {
            continue;
        }

        let word: String = first.iter().take(first.len() - 1).cloned().collect();
        let gender = match gender {
            "{n}" => "das",
            _ => unreachable!(),
        };

        if word != previous {
            if !previous.is_empty() {
                show(&previous, &meanings);
            }

            previous = gender.to_owned() + " " + &word;
            meanings.clear();
        }

        meanings.push(record.get(1).unwrap().into());
    }

    show(&previous, &meanings);

    Ok(())
}
