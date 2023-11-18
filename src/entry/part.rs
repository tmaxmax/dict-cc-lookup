use super::{Annotation, AnnotationKind, Case, Gender, Placeholder};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Part {
    Keyword(String),          // Keywords
    Extra(Vec<Part>),         // In parantheses
    VariantSeparator,         // The character "/"
    Placeholder(Placeholder), // etw. jd. jdn. jdm. jds.
    Annotation(Annotation), // Information inside [] (explanation), <> (alternative), {} (numbers) but not cases
    Gender(Gender),         // {m} {n} {f}
}

pub struct Parser<'a> {
    state: state::State<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self::make(input, false)
    }

    fn make(s: &'a str, stop_at_parens: bool) -> Self {
        Self {
            state: state::State::Base(state::Base::new(s, stop_at_parens)),
        }
    }

    fn parse(mut self) -> Result<(Vec<Part>, &'a str), anyhow::Error> {
        use state::Result as R;

        let mut parts = vec![];

        loop {
            match self.state.step() {
                R::Keep(state, part) => {
                    self.state = state;
                    parts.extend(part)
                }
                R::Done(res) => {
                    let (part, left) = res?;
                    parts.extend(part);
                    return Ok((parts, left));
                }
            };
        }
    }

    pub fn parse_parts(self) -> Result<Vec<Part>, anyhow::Error> {
        self.parse().map(|p| p.0)
    }
}

mod state {
    use super::Part;
    use anyhow::anyhow;

    fn is_special_char(c: char) -> bool {
        matches!(c, '[' | '{' | '<' | '(' | '/' | ' ')
    }

    fn is_special_char_with_end_paren(c: char) -> bool {
        matches!(c, '[' | '{' | '<' | '(' | ')' | '/' | ' ')
    }

    pub(super) struct Base<'a> {
        s: &'a str,
        stop_at_parens: bool,
        done: bool,
    }

    impl<'a> Base<'a> {
        pub(super) fn new(input: &'a str, nested: bool) -> Self {
            Self {
                s: input,
                stop_at_parens: nested,
                done: false,
            }
        }

        fn step(mut self) -> self::Result<'a> {
            use self::Result as R;
            use State as S;

            if self.is_empty() || self.done {
                return R::Done(if self.stop_at_parens && !self.done {
                    Err(anyhow!("base: unclosed parenthesis"))
                } else {
                    Ok((None, self.s))
                });
            }

            let i = match self.find(if self.stop_at_parens {
                is_special_char_with_end_paren
            } else {
                is_special_char
            }) {
                Some(i) => i,
                None => {
                    let consumed = self.s;
                    self.s = "";
                    return self.handle_word(consumed);
                }
            };

            let without_consumed = self.s;
            let consumed = &self.s[..i];
            let without_ch = &self.s[i..];
            let ch = &self.s[i..i + 1];
            let is_valid_parens = i == 0 || (i > 0 && matches!(self.s.get(i - 1..i), Some(" ")));
            self.s = &self.s[i + 1..];

            match ch {
                " " if consumed.is_empty() => R::Keep(S::Base(self), None),
                " " => self.handle_word(consumed),
                "/" if consumed.is_empty() => R::Keep(S::Base(self), Some(Part::VariantSeparator)),
                "/" => {
                    self.s = without_ch;
                    self.handle_word(consumed)
                }
                "[" | "<" => R::Keep(
                    S::Annotation(Annotation {
                        kind: match ch {
                            "[" => super::AnnotationKind::Explanation,
                            "<" => super::AnnotationKind::Alternative,
                            _ => unreachable!(),
                        },
                        b: self,
                    }),
                    None,
                ),
                "{" => R::Keep(S::Gender(Curly(self)), None),
                "(" if is_valid_parens => R::Keep(
                    S::Extra(Extra(
                        Box::new(super::Parser::make(self.s, true)),
                        self.stop_at_parens,
                    )),
                    None,
                ),
                "(" => {
                    self.s = without_consumed;
                    R::Keep(
                        S::KeywordParens(KeywordParens {
                            b: self,
                            search_at: i + 1,
                            parens_closed: false,
                        }),
                        None,
                    )
                }
                ")" if self.stop_at_parens => {
                    if consumed.is_empty() {
                        R::Done(Ok((None, self.s)))
                    } else {
                        self.done = true;
                        self.handle_word(consumed)
                    }
                }
                _ => R::Done(Err(anyhow!("base: unexpected special char \"{}\"", ch))),
            }
        }

        fn handle_word(self, word: &'a str) -> self::Result<'a> {
            use self::Result as R;
            use State as S;

            let maybe_person_placeholder_case = match word {
                "jd." => Some(super::Case::Nominative),
                "jdn." => Some(super::Case::Accusative),
                "jds." => Some(super::Case::Dative),
                "jdm." => Some(super::Case::Genitive),
                _ => None,
            };

            if let Some(case) = maybe_person_placeholder_case {
                return R::Keep(
                    S::Base(self),
                    Some(Part::Placeholder(super::Placeholder::Person(case))),
                );
            }

            match word {
                "etw." | "sich" => R::Keep(S::Placeholder(Placeholder(self, word)), None),
                _ => R::Keep(S::Base(self), Some(Part::Keyword(word.to_string()))),
            }
        }
    }

    impl std::ops::Deref for Base<'_> {
        type Target = str;

        fn deref(&self) -> &Self::Target {
            self.s
        }
    }

    pub(super) struct Extra<'a>(Box<super::Parser<'a>>, bool);

    impl<'a> Extra<'a> {
        fn step(self) -> self::Result<'a> {
            use self::Result as R;
            use State as S;

            match self.0.parse() {
                Ok((parts, s)) => R::Keep(
                    S::Base(Base {
                        s,
                        stop_at_parens: self.1,
                        done: false,
                    }),
                    Some(Part::Extra(parts)),
                ),
                Err(e) => R::Done(Err(e)),
            }
        }
    }

    pub(super) struct Placeholder<'a>(Base<'a>, &'a str);

    impl<'a> Placeholder<'a> {
        fn step(mut self) -> self::Result<'a> {
            use self::Result as R;
            use State as S;

            let get_placeholder = |case| match self.1 {
                "etw." => super::Placeholder::Thing(case),
                "sich" => super::Placeholder::Reflexive(case),
                _ => unreachable!(),
            };
            let default_part = || Some(Part::Placeholder(get_placeholder(None)));

            if self.0.is_empty() {
                return R::Done(Ok((default_part(), "")));
            }

            let mut s = self.0.s;
            let reset_state = || {
                R::Keep(
                    S::Base(Base {
                        s,
                        stop_at_parens: self.0.stop_at_parens,
                        done: self.0.done,
                    }),
                    default_part(),
                )
            };

            match s.find('[') {
                Some(i) if i < 2 => s = &s[i + 1..],
                Some(_) | None => return reset_state(),
            };

            if s.is_empty() {
                return R::Done(Err(anyhow!(
                    "thing_placeholder: unfinished placeholder case"
                )));
            }

            if let Some("+") = s.get(..1) {
                s = &s[1..];
            }

            if s.len() < 5 {
                return reset_state();
            }

            let case = match s.get(..4) {
                Some("Nom.") => super::Case::Nominative,
                Some("Akk.") => super::Case::Accusative,
                Some("Gen.") => super::Case::Genitive,
                Some("Dat.") => super::Case::Dative,
                Some(_) | None => return reset_state(),
            };

            s = &s[4..];

            if &s[..1] != "]" {
                return reset_state();
            }

            self.0.s = &s[1..];
            R::Keep(
                S::Base(self.0),
                Some(Part::Placeholder(get_placeholder(Some(case)))),
            )
        }
    }

    pub(super) struct Annotation<'a> {
        kind: super::AnnotationKind,
        b: Base<'a>,
    }

    impl<'a> Annotation<'a> {
        fn step(mut self) -> self::Result<'a> {
            use self::Result as R;
            use State as S;

            let (start_char, end_char) = match self.kind {
                super::AnnotationKind::Alternative => ('<', '>'),
                super::AnnotationKind::Explanation => ('[', ']'),
                super::AnnotationKind::Number => unreachable!(), // The Curly state takes care of this
            };

            let mut nesting = 1;
            let mut end = None;
            let mut first_non_annotation_end = None;

            for (i, c) in self.b.char_indices() {
                if c == end_char {
                    nesting -= 1;
                    if nesting == 0 {
                        end = Some(i);
                        break;
                    }
                } else if c == start_char {
                    nesting += 1;
                } else if self.b.stop_at_parens && is_special_char_with_end_paren(c)
                    || (!self.b.stop_at_parens && is_special_char(c))
                {
                    first_non_annotation_end = Some(i)
                }
            }

            let end = match end {
                Some(end) => end,
                None => {
                    return match first_non_annotation_end {
                        Some(i) => {
                            let consumed = start_char.to_string() + &self.b[..i];
                            self.b.s = &self.b.s[i..];
                            R::Keep(S::Base(self.b), Some(Part::Keyword(consumed)))
                        }
                        None => R::Done(Ok((
                            Some(Part::Keyword(start_char.to_string() + self.b.s)),
                            "",
                        ))),
                    };
                }
            };

            let part = Part::Annotation(super::Annotation {
                kind: self.kind,
                value: self.b[..end].to_string(),
            });

            self.b.s = &self.b.s[end + 1..];

            R::Keep(S::Base(self.b), Some(part))
        }
    }

    pub(super) struct Curly<'a>(Base<'a>);

    impl<'a> Curly<'a> {
        fn step(mut self) -> self::Result<'a> {
            use self::Result as R;
            use super::Gender as G;
            use State as S;

            let end = match self.0.s.find('}') {
                Some(end) => end,
                None => return R::Done(Err(anyhow!("curly: unfinished curly clause"))),
            };

            let gender_str = &self.0.s[..end];
            let gender = match gender_str {
                "m" => Some(G::Masculine),
                "f" => Some(G::Feminine),
                "n" => Some(G::Neutral),
                "pl" | "pl." | "sg" | "sg." => {
                    self.0.s = &self.0.s[end + 1..];

                    return R::Keep(
                        S::Base(self.0),
                        Some(Part::Annotation(super::Annotation {
                            kind: super::AnnotationKind::Number,
                            value: match gender_str {
                                "pl" | "pl." => "nur plural",
                                "sg" | "sg." => "singular",
                                _ => unreachable!(),
                            }
                            .into(),
                        })),
                    );
                }
                _ => None,
            };

            self.0.s = &self.0.s[end + 1..];

            R::Keep(S::Base(self.0), gender.map(Part::Gender))
        }
    }

    pub(super) struct KeywordParens<'a> {
        b: Base<'a>,
        search_at: usize,
        parens_closed: bool,
    }

    impl<'a> KeywordParens<'a> {
        fn step(mut self) -> Result<'a> {
            use self::Result as R;
            use State as S;

            let just_parens = |c| c == ')';

            let i = match self.b[self.search_at..].find(if self.parens_closed {
                is_special_char
            } else if self.b.stop_at_parens {
                is_special_char_with_end_paren
            } else {
                just_parens
            }) {
                Some(i) => i,
                None => return R::Done(Err(anyhow!("keyword_parens: unmatched parens"))),
            } + self.search_at;

            match &self.b[i..i + 1] {
                ")" => {
                    self.parens_closed = true;
                    self.search_at = i + 1;
                    if self.search_at == self.b.len() {
                        R::Done(Ok((
                            Some(Part::Keyword(self.b[..self.search_at].to_string())),
                            "",
                        )))
                    } else {
                        R::Keep(S::KeywordParens(self), None)
                    }
                }
                _ if self.parens_closed => {
                    let consumed = self.b[..i].to_string();
                    self.b.s = &self.b.s[i..];
                    R::Keep(
                        S::Base(self.b),
                        if consumed.is_empty() {
                            None
                        } else {
                            Some(Part::Keyword(consumed))
                        },
                    )
                }
                _ => R::Done(Err(anyhow!(
                    "keyword_parens: special char before closing parens"
                ))),
            }
        }
    }

    pub(super) enum Result<'a> {
        Keep(State<'a>, Option<Part>),
        Done(std::result::Result<(Option<Part>, &'a str), anyhow::Error>),
    }

    pub(super) enum State<'a> {
        Base(Base<'a>),
        Extra(Extra<'a>),
        Placeholder(Placeholder<'a>),
        Annotation(Annotation<'a>),
        Gender(Curly<'a>),
        KeywordParens(KeywordParens<'a>),
    }

    impl<'a> State<'a> {
        pub(super) fn step(self) -> Result<'a> {
            match self {
                Self::Base(s) => s.step(),
                Self::Annotation(s) => s.step(),
                Self::Placeholder(s) => s.step(),
                Self::Gender(s) => s.step(),
                Self::Extra(s) => s.step(),
                Self::KeywordParens(s) => s.step(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    #[test]
    fn parse_parts() {
        let data = [
            (
                "(an etw. [Dat.]) herumbasteln [ugs.]",
                vec![
                    Part::Extra(vec![
                        Part::Keyword("an".into()),
                        Part::Placeholder(Placeholder::Thing(Some(Case::Dative))),
                    ]),
                    Part::Keyword("herumbasteln".into()),
                    Part::Annotation(Annotation {
                        value: "ugs.".into(),
                        kind: AnnotationKind::Explanation,
                    }),
                ],
            ),
            (
                "sich [Akk.] ((bis) zu etw. [Dat.]) steigern",
                vec![
                    Part::Placeholder(Placeholder::Reflexive(Some(Case::Accusative))),
                    Part::Extra(vec![
                        Part::Extra(vec![Part::Keyword("bis".into())]),
                        Part::Keyword("zu".into()),
                        Part::Placeholder(Placeholder::Thing(Some(Case::Dative))),
                    ]),
                    Part::Keyword("steigern".into()),
                ],
            ),
            (
                "sich [Akk.] (auf jdn./etw.) aufstützen",
                vec![
                    Part::Placeholder(Placeholder::Reflexive(Some(Case::Accusative))),
                    Part::Extra(vec![
                        Part::Keyword("auf".into()),
                        Part::Placeholder(Placeholder::Person(Case::Accusative)),
                        Part::VariantSeparator,
                        Part::Placeholder(Placeholder::Thing(None)),
                    ]),
                    Part::Keyword("aufstützen".into()),
                ],
            ),
            (
                "Vanadoandrosit-(Ce) {m}",
                vec![
                    Part::Keyword("Vanadoandrosit-(Ce)".into()),
                    Part::Gender(Gender::Masculine),
                ],
            ),
            (
                "Uluguru-(Zwerg-)Galago {m}",
                vec![
                    Part::Keyword("Uluguru-(Zwerg-)Galago".into()),
                    Part::Gender(Gender::Masculine),
                ],
            ),
            (
                "vicanite-(Ce) [Na0.5(Ce,Ca,Th)15Fe [F9|(AsO3)0.5|(AsO4|BO3|Si3B3O18|SiO4)3]]",
                vec![
                    Part::Keyword("vicanite-(Ce)".into()),
                    Part::Annotation(Annotation {
                        value: "Na0.5(Ce,Ca,Th)15Fe [F9|(AsO3)0.5|(AsO4|BO3|Si3B3O18|SiO4)3]"
                            .into(),
                        kind: AnnotationKind::Explanation,
                    }),
                ],
            ),
            (
                "((für) etw. [Akk.]) pauken [ugs.] [intensiv lernen]",
                vec![
                    Part::Extra(vec![
                        Part::Extra(vec![Part::Keyword("für".into())]),
                        Part::Placeholder(Placeholder::Thing(Some(Case::Accusative))),
                    ]),
                    Part::Keyword("pauken".into()),
                    Part::Annotation(Annotation {
                        value: "ugs.".into(),
                        kind: AnnotationKind::Explanation,
                    }),
                    Part::Annotation(Annotation {
                        value: "intensiv lernen".into(),
                        kind: AnnotationKind::Explanation,
                    }),
                ],
            ),
            (
                "unter Spannung von > 50 V",
                vec![
                    Part::Keyword("unter".into()),
                    Part::Keyword("Spannung".into()),
                    Part::Keyword("von".into()),
                    Part::Keyword(">".into()),
                    Part::Keyword("50".into()),
                    Part::Keyword("V".into()),
                ],
            ),
            (
                "Blattgemüse {pl.}",
                vec![
                    Part::Keyword("Blattgemüse".into()),
                    Part::Annotation(Annotation {
                        value: "nur plural".into(),
                        kind: AnnotationKind::Number,
                    }),
                ],
            ),
            (
                "{sg.}",
                vec![Part::Annotation(Annotation {
                    value: "singular".into(),
                    kind: AnnotationKind::Number,
                })],
            ),
            (
                "Requiem(, nach Worten der heiligen Schrift)",
                vec![Part::Keyword(
                    "Requiem(, nach Worten der heiligen Schrift)".into(),
                )],
            ),
            // (
            //     "(A(B))",
            //     vec![Part::Extra(vec![Part::Keyword("A(B)".into())])],
            // ),
            (
                "Filovirus {n} {ugs.: m}",
                vec![
                    Part::Keyword("Filovirus".into()),
                    Part::Gender(Gender::Neutral),
                ],
            ),
            ("<SFL-Haubitze", vec![Part::Keyword("<SFL-Haubitze".into())]),
            (
                "(<SFL-Haubitze)",
                vec![Part::Extra(vec![Part::Keyword("<SFL-Haubitze".into())])],
            ),
        ];

        for (input, expected) in data {
            println!("{}", input);
            let output = Parser::new(input).parse_parts().unwrap();
            assert_eq!(output, expected);
        }
    }
}
