#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LiteralMarker {
    Curly,
    Square,
    Angle,
}

impl LiteralMarker {
    fn from_start_seq(seq: &str) -> Option<LiteralMarker> {
        match seq {
            "<" => Some(Self::Angle),
            "[" => Some(Self::Square),
            "{" => Some(Self::Curly),
            _ => None,
        }
    }

    fn seq_start(&self) -> &'static str {
        match self {
            Self::Angle => "<",
            Self::Square => "[",
            Self::Curly => "{",
        }
    }

    fn seq_end(&self) -> &'static str {
        match self {
            Self::Angle => ">",
            Self::Square => "]",
            Self::Curly => "}",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token<'a> {
    Parens {
        is_start: bool,
    },
    Slash,
    Literal {
        value: &'a str,
        marker: LiteralMarker,
    },
    Text(&'a str),
    Quoted(&'a str),
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("incomplete literal starting with \"{}\" at position {}", .marker.seq_start(), .at)]
    IncompleteLiteral { marker: LiteralMarker, at: usize },
    #[error("unexpected char at position {0}")]
    Unexpected(usize),
}

pub fn lex(input: &str) -> Tokens {
    Tokens {
        input,
        at: 0,
        space_behavior: SpaceBehavior::None,
    }
}

const QUOTE_PAIRS: &[(&str, &str)] = &[("“", "”"), ("”", "”"), ("„", "”"), ("„", "“"), ("'", "'")];

#[derive(Debug, Clone)]
pub struct Tokens<'a> {
    input: &'a str,
    at: usize,
    space_behavior: SpaceBehavior,
}

impl<'a> Tokens<'a> {
    pub fn as_str(&'a self) -> &'a str {
        &self.input[self.at..]
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Result<Token<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let c = next_char(&self.input[self.at..])?;

            if c == "(" {
                return match try_consume_text(self.input, self.at) {
                    Ok(Some((word, new_at, beh))) => {
                        self.at = new_at;
                        self.space_behavior = beh;
                        Some(Ok(Token::Text(word)))
                    }
                    Ok(None) => {
                        self.at += c.len();
                        self.space_behavior = SpaceBehavior::Maybe;
                        Some(Ok(Token::Parens { is_start: c == "(" }))
                    }
                    Err(err) => Some(Err(err)),
                };
            }

            if c == ")" {
                self.at += c.len();
                self.space_behavior = SpaceBehavior::Maybe;
                return Some(Ok(Token::Parens { is_start: false }));
            }

            if c == "/" {
                self.at += c.len();
                self.space_behavior = SpaceBehavior::Maybe;
                return Some(Ok(Token::Slash));
            }

            if c == " " {
                match self.space_behavior {
                    SpaceBehavior::None => return Some(Err(Error::Unexpected(self.at))),
                    SpaceBehavior::Required => {
                        self.at += 1;
                        self.space_behavior = SpaceBehavior::Maybe;
                    }
                    SpaceBehavior::Maybe => self.at += 1,
                }
                continue;
            }

            if let Some(marker) = LiteralMarker::from_start_seq(c) {
                let (token, new_at, beh) =
                    match try_consume_literal(self.input, self.at + c.len(), marker) {
                        Err(err) => match try_consume_text(self.input, self.at) {
                            Err(_) | Ok(None) => return Some(Err(err)),
                            Ok(Some((word, new_at, beh))) => (Token::Text(word), new_at, beh),
                        },
                        Ok((value, new_at, beh)) => (Token::Literal { value, marker }, new_at, beh),
                    };
                self.at = new_at;
                self.space_behavior = beh;
                return Some(Ok(token));
            }

            if matches!(self.space_behavior, SpaceBehavior::Required) {
                return Some(Err(Error::Unexpected(self.at)));
            }

            if QUOTE_PAIRS.iter().any(|(start_quote, _)| *start_quote == c) {
                if let Some((quoted, new_at, beh)) =
                    consume_quoted(self.input, self.at + c.len(), c)
                {
                    self.at = new_at;
                    self.space_behavior = beh;
                    return Some(Ok(Token::Quoted(quoted)));
                }
            }

            return match try_consume_text(self.input, self.at) {
                Ok(Some((word, new_at, beh))) => {
                    self.at = new_at;
                    self.space_behavior = beh;
                    Some(Ok(Token::Text(word)))
                }
                Ok(None) => Some(Err(Error::Unexpected(self.at))),
                Err(err) => Some(Err(err)),
            };
        }
    }
}

fn next_char(input: &str) -> Option<&str> {
    if input.is_empty() {
        return None;
    }

    let mut end = 1;
    while !input.is_char_boundary(end) {
        end += 1;
    }

    // SAFETY: Ensured above that end is a char boundary.
    return Some(unsafe { input.get_unchecked(..end) });
}

type Output<'a> = (&'a str, usize, SpaceBehavior);

fn try_consume_literal(input: &str, at: usize, marker: LiteralMarker) -> Result<Output, Error> {
    debug_assert!(!input.is_empty());

    let start = marker.seq_start();
    let end = marker.seq_end();

    let mut level = 1;
    let mut idx_start = at;
    let mut idx_end = idx_start + 1;

    while level > 0 && idx_end <= input.len() {
        let Some(c) = input.get(idx_start..idx_end) else {
            idx_end += 1;
            continue;
        };

        if c == start {
            level += 1
        } else if c == end {
            level -= 1
        }

        idx_start = idx_end;
        idx_end = idx_start + 1;
    }

    if level == 0 {
        Ok((
            &input[at..idx_start - end.len()],
            idx_start,
            SpaceBehavior::Maybe,
        ))
    } else {
        Err(Error::IncompleteLiteral { marker, at })
    }
}

fn try_consume_text(input: &str, at: usize) -> Result<Option<Output>, Error> {
    let mut idx_start = at;
    let mut idx_end = idx_start + 1;
    let mut level = 0;
    let mut idx_prev_parens_open = None;
    let mut idx_parens_open = None;
    let mut idx_prev_parens_close = None;
    let mut idx_parens_close = None;
    let mut parens_count = 0;
    let mut last_char = None;

    while idx_end <= input.len() {
        let Some(c) = input.get(idx_start..idx_end) else {
            idx_end += 1;
            continue;
        };

        last_char = Some(c);

        match c {
            "(" => {
                idx_prev_parens_open = idx_parens_open;
                idx_parens_open = Some(idx_start);
                parens_count += 1;
                level += 1;
            }
            ")" => {
                idx_prev_parens_close = idx_parens_close;
                idx_parens_close = Some(idx_start);
                level -= 1;
            }
            " " | "/" => break,
            _ => {}
        }

        if parens_count > 1 {
            idx_parens_open = idx_prev_parens_open;
            idx_parens_close = idx_prev_parens_close;
            break;
        }

        if !(0..=1).contains(&level) {
            break;
        }

        idx_start = idx_end;
        idx_end = idx_start + 1;
    }

    let new_at;

    if idx_end > input.len() {
        idx_end -= 1;
        new_at = idx_end;
    } else if let Some(c) = last_char {
        match c {
            " " => new_at = idx_end,
            "/" => new_at = idx_start,
            "(" => {
                new_at = idx_parens_open.unwrap();
                return if at == new_at {
                    Ok(None)
                } else {
                    Ok(Some((&input[at..new_at], new_at, SpaceBehavior::None)))
                };
            }
            ")" => {
                new_at = idx_parens_close.unwrap();
                return Ok(Some((&input[at..new_at], new_at, SpaceBehavior::Required)));
            }
            _ => unreachable!("stopped at char {idx_start} \"{c}\", {input}"),
        };
    } else {
        unreachable!("called with fully consumed input, {input}");
    }

    let word = &input[at..idx_start];

    let (idx_parens_open, idx_parens_close) = match (idx_parens_open, idx_parens_close) {
        (Some(a), Some(b)) => (a, b),
        (None, None) => return Ok(Some((word, new_at, SpaceBehavior::Maybe))),
        (Some(i), None) => {
            if i == at {
                return Ok(None);
            }

            let word = &input[at..i];
            return Ok(Some((word, i, SpaceBehavior::None)));
        }
        (None, Some(_)) => unreachable!(),
    };

    let starts_with_paren = idx_parens_open == at;
    let ends_with_paren = idx_parens_close == idx_start - 1;

    if starts_with_paren && ends_with_paren {
        return Ok(None);
    }

    fn is_word_char(c: char) -> bool {
        c.is_alphanumeric() || c == '-'
    }

    let inside_parens = &input[idx_parens_open + 1..idx_parens_close];
    let inside_has_spaces = inside_parens.contains(' ');
    let inside_has_only_non_word_chars = inside_parens.chars().all(|c| !is_word_char(c));
    let outside_has_non_word_chars = input[at..idx_parens_open].chars().any(|c| !is_word_char(c))
        || input[idx_parens_close + 1..idx_start]
            .chars()
            .any(|c| !is_word_char(c));

    if inside_has_spaces || inside_has_only_non_word_chars || outside_has_non_word_chars {
        return if starts_with_paren && !outside_has_non_word_chars {
            Err(Error::Unexpected(idx_parens_open + 1))
        } else if at < idx_parens_open {
            Ok(Some((
                &input[at..idx_parens_open],
                idx_parens_open,
                SpaceBehavior::None,
            )))
        } else {
            Ok(None)
        };
    }

    Ok(Some((
        word,
        new_at,
        if ends_with_paren {
            SpaceBehavior::Maybe
        } else {
            SpaceBehavior::None
        },
    )))
}

fn consume_quoted<'a>(input: &'a str, at: usize, start_quote: &'a str) -> Option<Output<'a>> {
    QUOTE_PAIRS
        .iter()
        .filter_map(|(s, e)| (*s == start_quote).then_some(e))
        .find_map(|e| input[at..].find(e).zip(Some(e.len())))
        .map(|(i, l)| (&input[at..at + i], at + i + l, SpaceBehavior::Maybe))
}

#[derive(Debug, Clone)]
enum SpaceBehavior {
    None,
    Maybe,
    Required,
}

#[cfg(test)]
mod tests {
    #[test]
    fn lex_success() {
        type Token = super::Token<'static>;
        use super::LiteralMarker as Marker;

        let data: &[(&str, &[Token])] = &[
            (
                "Magnesioferrit {m} [ein Spinell]",
                &[
                    Token::Text("Magnesioferrit"),
                    Token::Literal {
                        marker: Marker::Curly,
                        value: "m",
                    },
                    Token::Literal {
                        marker: Marker::Square,
                        value: "ein Spinell",
                    },
                ],
            ),
            (
                "Uluguru-(Zwerg-)Galago {m}",
                &[
                    Token::Text("Uluguru-(Zwerg-)Galago"),
                    Token::Literal {
                        marker: Marker::Curly,
                        value: "m",
                    },
                ],
            ),
            (
                "(an etw. [Dat.]) herumbasteln [ugs.]",
                &[
                    Token::Parens { is_start: true },
                    Token::Text("an"),
                    Token::Text("etw."),
                    Token::Literal {
                        value: "Dat.",
                        marker: Marker::Square,
                    },
                    Token::Parens { is_start: false },
                    Token::Text("herumbasteln"),
                    Token::Literal {
                        value: "ugs.",
                        marker: Marker::Square,
                    },
                ],
            ),
            (
                "Vanadoandrosit-(Ce) {m}",
                &[
                    Token::Text("Vanadoandrosit-(Ce)"),
                    Token::Literal {
                        value: "m",
                        marker: Marker::Curly,
                    },
                ],
            ),
            (
                "Requiem(, nach Worten der heiligen Schrift)",
                &[
                    Token::Text("Requiem"),
                    Token::Parens { is_start: true },
                    Token::Text(","),
                    Token::Text("nach"),
                    Token::Text("Worten"),
                    Token::Text("der"),
                    Token::Text("heiligen"),
                    Token::Text("Schrift"),
                    Token::Parens { is_start: false },
                ],
            ),
            (
                "(A(B))",
                &[
                    Token::Parens { is_start: true },
                    Token::Text("A(B)"),
                    Token::Parens { is_start: false },
                ],
            ),
            ("<SFL-Haubitze", &[Token::Text("<SFL-Haubitze")]),
            (
                "(<SFL-Haubitze)",
                &[
                    Token::Parens { is_start: true },
                    Token::Text("<SFL-Haubitze"),
                    Token::Parens { is_start: false },
                ],
            ),
            (
                "unter Spannung von > 50 V",
                &[
                    Token::Text("unter"),
                    Token::Text("Spannung"),
                    Token::Text("von"),
                    Token::Text(">"),
                    Token::Text("50"),
                    Token::Text("V"),
                ],
            ),
            (
                "vicanite-(Ce) [Na0.5(Ce,Ca,Th)15Fe [F9|(AsO3)0.5|(AsO4|BO3|Si3B3O18|SiO4)3]]",
                &[
                    Token::Text("vicanite-(Ce)"),
                    Token::Literal {
                        value: "Na0.5(Ce,Ca,Th)15Fe [F9|(AsO3)0.5|(AsO4|BO3|Si3B3O18|SiO4)3]",
                        marker: Marker::Square,
                    },
                ],
            ),
            (
                "sich [Akk.] (auf jdn./etw.) aufstützen",
                &[
                    Token::Text("sich"),
                    Token::Literal {
                        value: "Akk.",
                        marker: Marker::Square,
                    },
                    Token::Parens { is_start: true },
                    Token::Text("auf"),
                    Token::Text("jdn."),
                    Token::Slash,
                    Token::Text("etw."),
                    Token::Parens { is_start: false },
                    Token::Text("aufstützen"),
                ],
            ),
            (
                "mutwillige / böswillige Beschädigung {f}",
                &[
                    Token::Text("mutwillige"),
                    Token::Slash,
                    Token::Text("böswillige"),
                    Token::Text("Beschädigung"),
                    Token::Literal {
                        value: "f",
                        marker: Marker::Curly,
                    },
                ],
            ),
            (
                "(Afrikanische) Bodenagame {f}",
                &[
                    Token::Parens { is_start: true },
                    Token::Text("Afrikanische"),
                    Token::Parens { is_start: false },
                    Token::Text("Bodenagame"),
                    Token::Literal {
                        value: "f",
                        marker: Marker::Curly,
                    },
                ],
            ),
            (
                "(Ach, du) heilige Scheiße! [vulg.]",
                &[
                    Token::Parens { is_start: true },
                    Token::Text("Ach,"),
                    Token::Text("du"),
                    Token::Parens { is_start: false },
                    Token::Text("heilige"),
                    Token::Text("Scheiße!"),
                    Token::Literal {
                        value: "vulg.",
                        marker: Marker::Square,
                    },
                ],
            ),
            (
                "(am / zu) Anfang des Winters",
                &[
                    Token::Parens { is_start: true },
                    Token::Text("am"),
                    Token::Slash,
                    Token::Text("zu"),
                    Token::Parens { is_start: false },
                    Token::Text("Anfang"),
                    Token::Text("des"),
                    Token::Text("Winters"),
                ],
            ),
            (
                "(ganz) unten(,) am Grund",
                &[
                    Token::Parens { is_start: true },
                    Token::Text("ganz"),
                    Token::Parens { is_start: false },
                    Token::Text("unten"),
                    Token::Parens { is_start: true },
                    Token::Text(","),
                    Token::Parens { is_start: false },
                    Token::Text("am"),
                    Token::Text("Grund"),
                ],
            ),
            (
                "(Ach was), echt?",
                &[
                    Token::Parens { is_start: true },
                    Token::Text("Ach"),
                    Token::Text("was"),
                    Token::Parens { is_start: false },
                    Token::Text(","),
                    Token::Text("echt?"),
                ],
            ),
            ("401(k) Plan", &[Token::Text("401(k)"), Token::Text("Plan")]),
            (
                "Leute {pl}, die",
                &[
                    Token::Text("Leute"),
                    Token::Literal {
                        value: "pl",
                        marker: Marker::Curly,
                    },
                    Token::Text(","),
                    Token::Text("die"),
                ],
            ),
            (
                "Blei(II,IV)-oxid {n}",
                &[
                    Token::Text("Blei(II,IV)-oxid"),
                    Token::Literal {
                        value: "n",
                        marker: Marker::Curly,
                    },
                ],
            ),
            (
                "Jiminy (cricket)!  [Am.]  [coll.]",
                &[
                    Token::Text("Jiminy"),
                    Token::Parens { is_start: true },
                    Token::Text("cricket"),
                    Token::Parens { is_start: false },
                    Token::Text("!"),
                    Token::Literal {
                        value: "Am.",
                        marker: Marker::Square,
                    },
                    Token::Literal {
                        value: "coll.",
                        marker: Marker::Square,
                    },
                ],
            ),
        ];

        for (input, expected) in data {
            eprintln!("Testing \"{input}\"...");
            let output = super::lex(input).collect::<Result<Vec<_>, _>>().unwrap();
            assert_eq!(&output[..], *expected);
        }
    }
}
