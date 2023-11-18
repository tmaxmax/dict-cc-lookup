use std::fmt;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum AnnotationKind {
    Explanation, // [ ]
    Alternative, // < >
    Number,      // some between { }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Annotation {
    pub value: String,
    pub kind: AnnotationKind,
}

impl fmt::Debug for Annotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)?;
        write!(f, "(\"{}\")", self.value)
    }
}
