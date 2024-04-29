use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Serialize)]
pub struct Pos {
    pub line: usize,
    pub column: usize,
}

impl Display for Pos {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

impl From<(usize, usize)> for Pos {
    fn from((line, column): (usize, usize)) -> Self {
        Self { line, column }
    }
}
