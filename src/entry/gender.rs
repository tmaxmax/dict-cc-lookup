use std::fmt;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Gender {
    Masculine,
    Feminine,
    Neutral,
}

impl fmt::Display for Gender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Masculine => "der",
            Self::Feminine => "die",
            Self::Neutral => "das",
        })
    }
}
