use std::{
    cmp::Ordering,
    fmt::{self, Display},
};

use super::{
    part::{Parser, Part},
    Annotation, AnnotationKind,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Term {
    parts: Vec<Part>,
}

impl Term {
    pub fn parse(input: &str) -> Result<Term, anyhow::Error> {
        Parser::new(input).parse_parts().map(|v| Self { parts: v })
    }

    pub fn match_exact(&self, input: &str) -> bool {
        let mut it = self.parts.iter().filter_map(|p| match p {
            Part::Keyword(w) => Some(w),
            _ => None,
        });
        let keyword = match it.next() {
            Some(k) => k,
            None => return false,
        };
        if it.next().is_some() {
            return false;
        }

        input.len() == keyword.len() && crate::util::case_fold_eq(input, keyword)
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

impl std::cmp::PartialOrd for Term {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let keywords_self = self.parts.iter().filter(|p| matches!(p, Part::Keyword(_)));
        let keywords_other = other.parts.iter().filter(|p| matches!(p, Part::Keyword(_)));

        if let Some(res) = keywords_self.partial_cmp(keywords_other) {
            if res != Ordering::Equal {
                return Some(res);
            }
        }

        let extra_self = self.parts.iter().filter(|p| matches!(p, Part::Extra(_)));
        let extra_other = other.parts.iter().filter(|p| matches!(p, Part::Extra(_)));

        if let Some(res) = extra_self.partial_cmp(extra_other) {
            if res != Ordering::Equal {
                return Some(res);
            }
        }

        None
    }
}

fn format_parts(parts: &[Part]) -> String {
    let mut out = String::new();

    for p in parts.iter().filter_map(|p| match p {
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
    }) {
        if !out.is_empty() && !out.ends_with("/") && p != "/" {
            out.push_str(" ");
        }

        out.push_str(&p);
    }

    out
}
