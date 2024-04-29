use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Serialize)]
pub enum Var {
    Local(String),
    Global(String),
}

impl Display for Var {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Var::Local(name) => f.write_str(name),
            Var::Global(name) => write!(f, "global.{name}"),
        }
    }
}
