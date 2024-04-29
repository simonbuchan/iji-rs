use super::{Assign, Expr, Stmt, Var};

#[allow(unused_variables)]
pub trait Visitor {
    fn stmt(&mut self, value: &Stmt) -> bool {
        true
    }
    fn assign(&mut self, value: &Assign) -> bool {
        true
    }
    fn expr(&mut self, value: &Expr) -> bool {
        true
    }
    fn var(&mut self, value: &Var) {}
}
