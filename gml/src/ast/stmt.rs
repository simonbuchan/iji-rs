use serde::Serialize;

use super::{Assign, Expr, Pos, Visitor};

#[derive(Clone, Debug, Serialize)]
pub enum Stmt {
    Var(String),
    Assign {
        pos: Pos,
        assign: Assign,
    },
    Expr {
        pos: Pos,
        expr: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        body: Box<Stmt>,
        alt: Option<Box<Stmt>>,
    },
    Repeat {
        count: Box<Expr>,
        body: Box<Stmt>,
    },
    While {
        cond: Box<Expr>,
        body: Box<Stmt>,
    },
    For {
        assign: Assign,
        cond: Box<Expr>,
        update: Assign,
        body: Box<Stmt>,
    },
    With {
        obj: Box<Expr>,
        body: Box<Stmt>,
    },
    Return {
        expr: Box<Expr>,
    },
    Exit,
    Block {
        stmts: Vec<Box<Stmt>>,
    },
    Empty,
}

impl Stmt {
    pub fn visit<V: Visitor>(&self, visitor: &mut V) {
        if !visitor.stmt(self) {
            return;
        }
        match self {
            Self::Var(_) => {}
            Self::Assign { assign, .. } => {
                assign.visit(visitor);
            }
            Self::Expr { expr, .. } => {
                expr.visit(visitor);
            }
            Self::If { cond, body, alt } => {
                cond.visit(visitor);
                body.visit(visitor);
                if let Some(alt) = alt {
                    alt.visit(visitor);
                }
            }
            Self::Repeat { count, body } => {
                count.visit(visitor);
                body.visit(visitor);
            }
            Self::While { cond, body } => {
                cond.visit(visitor);
                body.visit(visitor);
            }
            Self::For {
                assign,
                cond,
                update,
                body,
            } => {
                assign.visit(visitor);
                cond.visit(visitor);
                update.visit(visitor);
                body.visit(visitor);
            }
            Self::With { obj, body } => {
                obj.visit(visitor);
                body.visit(visitor);
            }
            Self::Return { expr } => {
                expr.visit(visitor);
            }
            Self::Exit => {}
            Self::Block { stmts } => {
                for stmt in stmts {
                    stmt.visit(visitor);
                }
            }
            Self::Empty => {}
        }
    }
}
