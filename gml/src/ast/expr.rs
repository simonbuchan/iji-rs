use serde::Serialize;
use std::fmt::{Display, Formatter};

use super::{Pos, Var, Visitor};

#[derive(Clone, Debug, Serialize)]
pub enum Expr {
    Var(Var),
    Int(i32),
    Float(f64),
    String(String),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        lhs: Box<Expr>,
        op: BinaryOp,
        rhs: Box<Expr>,
    },
    Member {
        lhs: Box<Expr>,
        name: String,
    },
    Index {
        lhs: Box<Expr>,
        indices: Vec<Box<Expr>>,
    },
    Call {
        pos: Pos,
        name: String,
        args: Vec<Box<Expr>>,
    },
}

impl Expr {
    pub fn visit<V: Visitor>(&self, visitor: &mut V) {
        if !visitor.expr(self) {
            return;
        }
        match self {
            Self::Var(var) => {
                visitor.var(var);
            }
            Self::Int(_) | Self::Float(_) | Self::String(_) => {}
            Self::Unary { expr, .. } => {
                expr.visit(visitor);
            }
            Self::Binary { lhs, rhs, .. } => {
                lhs.visit(visitor);
                rhs.visit(visitor);
            }
            Self::Member { lhs: expr, .. } => {
                expr.visit(visitor);
            }
            Self::Index { lhs: expr, indices } => {
                expr.visit(visitor);
                for index in indices {
                    index.visit(visitor);
                }
            }
            Self::Call { args, .. } => {
                for arg in args {
                    arg.visit(visitor);
                }
            }
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Var(var) => write!(f, "{var}"),
            Expr::Int(value) => write!(f, "{value}"),
            Expr::Float(value) => write!(f, "{value}"),
            Expr::String(value) => write!(f, "{value:?}"),
            Expr::Unary { op, expr } => write!(f, "{op}({expr})"),
            Expr::Binary { lhs, op, rhs } => write!(f, "({lhs}) {op} ({rhs})"),
            Expr::Member { lhs, name } => write!(f, "{lhs}.{name}"),
            Expr::Index { lhs, indices } => write!(f, "{lhs}[{}]", CommaSep(&indices)),
            Expr::Call { pos: _, name, args } => write!(f, "{name}({})", CommaSep(&args)),
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
pub enum UnaryOp {
    Not,
    Pos,
    Neg,
    BitNot,
    PreIncr,
    PreDecr,
    PostIncr,
    PostDecr,
}

impl Display for UnaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOp::Not => f.write_str("!"),
            UnaryOp::Pos => f.write_str("+"),
            UnaryOp::Neg => f.write_str("-"),
            UnaryOp::BitNot => f.write_str("~"),
            UnaryOp::PreIncr => f.write_str("++"),
            UnaryOp::PreDecr => f.write_str("--"),
            UnaryOp::PostIncr => todo!(),
            UnaryOp::PostDecr => todo!(),
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
pub enum BinaryOp {
    And,
    Or,
    Xor,
    BitAnd,
    BitOr,
    BitXor,
    Le,
    Lt,
    Ge,
    Gt,
    Ne,
    Eq,
    Add,
    Sub,
    Mul,
    Div,
    IDiv,
    IMod,
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
            BinaryOp::Xor => "^^",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::Le => "<=",
            BinaryOp::Lt => "<",
            BinaryOp::Ge => ">=",
            BinaryOp::Gt => ">",
            BinaryOp::Ne => "!=",
            BinaryOp::Eq => "==",
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::IDiv => "div",
            BinaryOp::IMod => "mod",
        })
    }
}

struct CommaSep<'a, I>(&'a I);

impl<'a, I, T> Display for CommaSep<'a, I>
where
    I: IntoIterator<Item = &'a T> + Copy,
    T: Display + 'a,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut it = self.0.into_iter();
        if let Some(item) = it.next() {
            item.fmt(f)?;
            for item in it {
                write!(f, ", {item}")?;
            }
        }
        Ok(())
    }
}
