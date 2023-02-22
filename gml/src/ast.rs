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

#[derive(Clone, Debug)]
pub enum Var {
    Local(String),
    Global(String),
}

#[derive(Copy, Clone, Debug)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
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
        id: String,
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
