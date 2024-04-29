use serde::Serialize;

use super::{Stmt, Visitor};

#[derive(Clone, Debug, Serialize)]
pub struct Script {
    pub name: String,
    pub stmts: Vec<Box<Stmt>>,
}

impl Script {
    pub fn visit<V: Visitor>(&self, visitor: &mut V) {
        for stmt in &self.stmts {
            stmt.visit(visitor);
        }
    }
}
