use std::fmt::{Display, Formatter};

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

#[derive(Clone, Debug)]
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

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
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
