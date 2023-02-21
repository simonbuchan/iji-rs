use std::rc::Rc;

pub use ast::parse;

pub type String = Rc<str>;

pub mod ast;
pub mod eval;
