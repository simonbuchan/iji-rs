use std::fmt::{Display, Formatter};

use serde::Serialize;

use super::{Expr, Visitor};

#[derive(Clone, Debug, Serialize)]
pub struct Assign {
    pub lhs: Box<Expr>,
    pub op: AssignOp,
    pub rhs: Box<Expr>,
}

impl Assign {
    pub fn visit<V: Visitor>(&self, visitor: &mut V) {
        if !visitor.assign(self) {
            return;
        }
        self.lhs.visit(visitor);
        self.rhs.visit(visitor);
    }
}

impl Display for Assign {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.lhs, self.op, self.rhs)
    }
}
#[derive(Copy, Clone, Debug, Serialize)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
}

impl Display for AssignOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AssignOp::Assign => "=",
            AssignOp::AddAssign => "+=",
            AssignOp::SubAssign => "-=",
            AssignOp::MulAssign => "*=",
            AssignOp::DivAssign => "/=",
        })
    }
}
