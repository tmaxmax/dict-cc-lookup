use std::fmt;

use anyhow::anyhow;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Case {
    Nominative,
    Accusative,
    Dative,
    Genitive,
}

impl Case {
    fn repr_str(&self, capitalized: bool) -> &str {
        match self {
            Self::Nominative => {
                if capitalized {
                    "Nominativ"
                } else {
                    "nominativ"
                }
            }
            Self::Accusative => {
                if capitalized {
                    "Akkusativ"
                } else {
                    "akkusativ"
                }
            }
            Self::Dative => {
                if capitalized {
                    "Dativ"
                } else {
                    "dativ"
                }
            }
            Self::Genitive => {
                if capitalized {
                    "Genitiv"
                } else {
                    "genitiv"
                }
            }
        }
    }

    fn repr_letter(&self) -> char {
        unsafe { self.repr_str(true).chars().next().unwrap_unchecked() }
    }
}

impl TryFrom<&str> for Case {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Nom" => Ok(Self::Nominative),
            "Akk" => Ok(Self::Accusative),
            "Gen" => Ok(Self::Genitive),
            "Dat" => Ok(Self::Dative),
            _ => Err(anyhow!("unknown case repr \"{}\"", value)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Placeholder {
    Reflexive(Option<Case>),
    Thing(Option<Case>),
    Person(Case),
}

impl fmt::Display for Placeholder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Thing(case) | Self::Reflexive(case) => {
                let s = match self {
                    Self::Thing(_) => "etw",
                    Self::Reflexive(_) => "sich",
                    _ => unreachable!(),
                };
                match case {
                    Some(case) => write!(f, "{}({})", s, case.repr_letter()),
                    None => f.write_str(s),
                }
            }
            Self::Person(case) => match case {
                Case::Nominative => f.write_str("jd"),
                Case::Accusative => f.write_str("jdn"),
                Case::Dative => f.write_str("jdm"),
                Case::Genitive => f.write_str("jds"),
            },
        }
    }
}
