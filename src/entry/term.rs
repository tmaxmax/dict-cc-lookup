use std::{
    cmp::Ordering,
    fmt::{self, Display},
};

use super::{
    part::{Parser, Part},
    Annotation, AnnotationKind,
};

pub struct Term {
    parts: Vec<Part>,
}

impl Term {
    pub fn parse(input: &str) -> Result<Term, anyhow::Error> {
        Parser::new(input).parse_parts().map(|v| Self { parts: v })
    }

    pub fn match_exact(&self, input: &str) -> bool {
        let keyword: Vec<_> = self
            .parts
            .iter()
            .filter_map(|p| match p {
                Part::Keyword(w) => Some(w),
                _ => None,
            })
            .collect();

        keyword.len() == 1
            && (keyword[0] == input || keyword[0].to_lowercase() == input.to_lowercase())
    }
}

impl Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = self.parts.clone();
        parts.sort_by(|a, b| {
            if matches!(a, Part::Gender(_)) {
                Ordering::Less
            } else if matches!(b, Part::Gender(_)) {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
        f.write_str(&format_parts(&parts))
    }
}

fn format_parts(parts: &[Part]) -> String {
    parts
        .iter()
        .filter_map(|p| match p {
            Part::Keyword(k) => Some(k.clone()),
            Part::Placeholder(ph) => Some(ph.to_string()),
            Part::VariantSeparator => Some("/".to_string()),
            Part::Gender(g) => Some(g.to_string()),
            Part::Annotation(Annotation {
                value,
                kind: AnnotationKind::Number,
            }) => Some(format!("[{}]", value)),
            Part::Extra(ps) => Some("(".to_string() + &format_parts(ps) + ")"),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}
