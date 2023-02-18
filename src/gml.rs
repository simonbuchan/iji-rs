use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest::Parser as _;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "gml.pest"]
struct G;

#[allow(unused_variables)]
pub trait Visitor {
    fn stmt(&mut self, value: &Stmt) -> bool {
        true
    }
    fn assign(&mut self, value: &Assign) -> bool {
        true
    }
    fn assign_lhs(&mut self, value: &AssignLhs) -> bool {
        true
    }
    fn expr(&mut self, value: &Expr) -> bool {
        true
    }
    fn var(&mut self, value: &Var) {}
}

#[derive(Debug)]
pub struct File {
    pub stmts: Vec<Box<Stmt>>,
}

impl File {
    pub fn visit<V: Visitor>(&self, visitor: &mut V) {
        for stmt in &self.stmts {
            stmt.visit(visitor);
        }
    }
}

pub fn tokenize(input: &str) -> anyhow::Result<()> {
    for pair in G::parse(Rule::tokens, input)? {
        if pair.as_rule() == Rule::EOI {
            break;
        }
        let (line, col) = pair.line_col();
        let str = pair.as_str();
        let rule = pair.into_inner().next().unwrap().as_rule();
        println!("{line:3}:{col:2}: {rule:?} {:?}", str);
    }
    Ok(())
}

pub fn parse(input: &str) -> anyhow::Result<File> {
    let pairs = G::parse(Rule::stmts, input)?;
    let mut stmts = vec![];
    for pair in pairs {
        if pair.as_rule() == Rule::EOI {
            break;
        }
        stmts.push(parse_stmt(pair));
    }
    Ok(File { stmts })
}

pub fn dump_parse(input: &str) -> anyhow::Result<()> {
    let pairs = G::parse(Rule::stmts, input)?;
    dump_tree(0, pairs);
    Ok(())
}

fn dump_tree(indent: usize, pairs: Pairs<'_, Rule>) {
    for pair in pairs {
        let rule = pair.as_rule();
        let span = pair.as_span();
        let start = span.start();
        let end = span.end();
        let (line, col) = pair.line_col();
        let mut str = pair.as_str();
        if str.len() > 20 {
            str = &str[..20];
        }
        println!(
            "{:indent$}{line}:{col}: [{start}-{end}] {rule:?} {str:?}",
            ""
        );
        dump_tree(indent + 2, pair.into_inner());
    }
}

#[derive(Debug)]
pub enum Stmt {
    Expr(Box<Expr>),
    Assign(Assign),
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
            Self::Expr(expr) => {
                expr.visit(visitor);
            }
            Self::Assign(assign) => {
                assign.visit(visitor);
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

fn parse_stmt(pair: Pair<'_, Rule>) -> Box<Stmt> {
    match pair.as_rule() {
        Rule::if_stmt => {
            let mut inner = pair.into_inner();
            let cond = parse_expr(inner.next().unwrap());
            let stmt = parse_stmt(inner.next().unwrap());
            let alt = inner.next().map(parse_stmt);
            Box::new(Stmt::If {
                cond,
                body: stmt,
                alt,
            })
        }
        Rule::repeat_stmt => {
            let mut inner = pair.into_inner();
            let count = parse_expr(inner.next().unwrap());
            let stmt = parse_stmt(inner.next().unwrap());
            Box::new(Stmt::Repeat { count, body: stmt })
        }
        Rule::while_stmt => {
            let mut inner = pair.into_inner();
            let cond = parse_expr(inner.next().unwrap());
            let body = parse_stmt(inner.next().unwrap());
            Box::new(Stmt::While { cond, body })
        }
        Rule::for_stmt => {
            let mut inner = pair.into_inner();
            let assign = parse_assign(inner.next().unwrap());
            let cond = parse_expr(inner.next().unwrap());
            let update = parse_assign(inner.next().unwrap());
            let body = parse_stmt(inner.next().unwrap());
            Box::new(Stmt::For {
                assign,
                cond,
                update,
                body,
            })
        }
        Rule::with_stmt => {
            let mut inner = pair.into_inner();
            let obj = parse_expr(inner.next().unwrap());
            let body = parse_stmt(inner.next().unwrap());
            Box::new(Stmt::With { obj, body })
        }
        Rule::return_stmt => {
            let mut inner = pair.into_inner();
            let expr = parse_expr(inner.next().unwrap());
            Box::new(Stmt::Return { expr })
        }
        Rule::exit_stmt => Box::new(Stmt::Exit),
        Rule::block_stmt => {
            let inner = pair.into_inner();
            let stmts = inner.map(parse_stmt).collect();
            Box::new(Stmt::Block { stmts })
        }
        Rule::assign_stmt => {
            let mut inner = pair.into_inner();
            let assign = parse_assign(inner.next().unwrap());
            Box::new(Stmt::Assign(assign))
        }
        Rule::expr_stmt => {
            let mut inner = pair.into_inner();
            let expr = parse_expr(inner.next().unwrap());
            Box::new(Stmt::Expr(expr))
        }
        Rule::empty_stmt => Box::new(Stmt::Empty),
        _ => unreachable!("bad stmt: {pair:?}"),
    }
}

#[derive(Debug)]
pub struct Assign {
    pub lhs: Box<AssignLhs>,
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

#[derive(Debug)]
pub enum AssignLhs {
    Var(Var),
    Id(Box<Expr>),
    Member {
        lhs: Box<AssignLhs>,
        id: String,
    },
    Index {
        lhs: Box<AssignLhs>,
        indices: Vec<Box<Expr>>,
    },
}

impl AssignLhs {
    pub fn visit<V: Visitor>(&self, visitor: &mut V) {
        if !visitor.assign_lhs(self) {
            return;
        }
        match self {
            Self::Var(var) => {
                visitor.var(var);
            }
            Self::Id(expr) => {
                expr.visit(visitor);
            }
            Self::Member { lhs, .. } => {
                lhs.visit(visitor);
            }
            Self::Index { lhs, indices } => {
                lhs.visit(visitor);
                for index in indices {
                    index.visit(visitor);
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum Var {
    Local(String),
    Global(String),
}

#[derive(Debug)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
}

fn parse_var(pair: Pair<'_, Rule>) -> Var {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap();
    match id.as_rule() {
        Rule::global => Var::Global(inner.next().unwrap().as_str().to_string()),
        Rule::id => Var::Local(id.as_str().to_string()),
        _ => unreachable!("bad var: {id:?}"),
    }
}

fn parse_assign_lhs(pair: Pair<'_, Rule>) -> Box<AssignLhs> {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap();
    let mut lhs = match id.as_rule() {
        Rule::var => Box::new(AssignLhs::Var(parse_var(id))),
        Rule::assign_id => {
            let mut inner = id.into_inner();
            let expr = parse_expr(inner.next().unwrap());
            Box::new(AssignLhs::Id(expr))
        }
        _ => unreachable!("bad assign lhs: {id:?}"),
    };
    for op in inner {
        match op.as_rule() {
            Rule::member => {
                let mut inner = op.into_inner();
                let id = inner.next().unwrap().as_str().to_string();
                lhs = Box::new(AssignLhs::Member { lhs, id })
            }
            Rule::index => {
                let inner = op.into_inner();
                let indices = inner.map(parse_expr).collect();
                lhs = Box::new(AssignLhs::Index { lhs, indices })
            }
            _ => unreachable!("bad assign lhs op: {op:?}"),
        }
    }
    lhs
}

fn parse_assign(pair: Pair<'_, Rule>) -> Assign {
    let mut inner = pair.into_inner();
    let lhs = parse_assign_lhs(inner.next().unwrap());
    let op = match inner.next().unwrap().as_rule() {
        Rule::assign => AssignOp::Assign,
        Rule::add_assign => AssignOp::AddAssign,
        Rule::sub_assign => AssignOp::SubAssign,
        Rule::mul_assign => AssignOp::MulAssign,
        Rule::div_assign => AssignOp::DivAssign,
        rule => unreachable!("bad assign op: {rule:?}"),
    };
    let rhs = parse_expr(inner.next().unwrap());
    Assign { lhs, op, rhs }
}

#[derive(Debug)]
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
        expr: Box<Expr>,
        id: String,
    },
    Index {
        expr: Box<Expr>,
        indices: Vec<Box<Expr>>,
    },
    Call {
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
            Self::Member { expr, .. } => {
                expr.visit(visitor);
            }
            Self::Index { expr, indices } => {
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

fn parse_expr(pair: Pair<'_, Rule>) -> Box<Expr> {
    parse_expr_rec(pair, &pratt())
}

fn parse_expr_rec(pair: Pair<'_, Rule>, pratt: &PrattParser<Rule>) -> Box<Expr> {
    pratt
        .map_primary(|primary| match primary.as_rule() {
            Rule::expr => parse_expr_rec(primary, pratt),
            Rule::call_expr => {
                let mut inner = primary.into_inner();
                let id = inner.next().unwrap().as_str().to_string();
                let args = inner.map(|pair| parse_expr_rec(pair, pratt)).collect();
                Box::new(Expr::Call { id, args })
            }
            Rule::var => Box::new(Expr::Var(parse_var(primary))),
            Rule::int => Box::new(Expr::Int(primary.as_str().parse().unwrap())),
            Rule::float => Box::new(Expr::Float(primary.as_str().parse().unwrap())),
            Rule::str => Box::new(Expr::String(primary.as_str().to_string())),
            _ => unreachable!("bad primary: {primary:?}"),
        })
        .map_prefix(|op, expr| {
            let op = match op.as_rule() {
                Rule::not => UnaryOp::Not,
                Rule::neg => UnaryOp::Neg,
                Rule::pos => UnaryOp::Pos,
                Rule::bit_not => UnaryOp::BitNot,
                Rule::pre_incr => UnaryOp::PreIncr,
                Rule::pre_decr => UnaryOp::PreDecr,
                _ => unreachable!("bad prefix op: {op:?}"),
            };
            Box::new(Expr::Unary { op, expr })
        })
        .map_postfix(|expr, op| match op.as_rule() {
            Rule::member => {
                let mut inner = op.into_inner();
                let id = inner.next().unwrap().as_str().to_string();
                Box::new(Expr::Member { expr, id })
            }
            Rule::index => {
                let inner = op.into_inner();
                let indices = inner.map(|pair| parse_expr_rec(pair, pratt)).collect();
                Box::new(Expr::Index { expr, indices })
            }
            Rule::post_incr => Box::new(Expr::Unary {
                op: UnaryOp::PostIncr,
                expr,
            }),
            Rule::post_decr => Box::new(Expr::Unary {
                op: UnaryOp::PostDecr,
                expr,
            }),
            _ => unreachable!("bad postfix op: {op:?}"),
        })
        .map_infix(|lhs, op, rhs| {
            let op = match op.as_rule() {
                Rule::and => BinaryOp::And,
                Rule::or => BinaryOp::Or,
                Rule::xor => BinaryOp::Xor,
                Rule::bit_and => BinaryOp::BitAnd,
                Rule::bit_or => BinaryOp::BitOr,
                Rule::bit_xor => BinaryOp::BitXor,
                Rule::le => BinaryOp::Le,
                Rule::lt => BinaryOp::Lt,
                Rule::ge => BinaryOp::Ge,
                Rule::gt => BinaryOp::Gt,
                Rule::ne => BinaryOp::Ne,
                Rule::eq => BinaryOp::Eq,
                Rule::add => BinaryOp::Add,
                Rule::sub => BinaryOp::Sub,
                Rule::mul => BinaryOp::Mul,
                Rule::div => BinaryOp::Div,
                Rule::idiv => BinaryOp::IDiv,
                Rule::imod => BinaryOp::IMod,
                _ => unreachable!("box infix op: {op:?}"),
            };
            Box::new(Expr::Binary { lhs, op, rhs })
        })
        .parse(pair.into_inner())
}

fn pratt() -> PrattParser<Rule> {
    PrattParser::new()
        .op(Op::infix(Rule::and, Assoc::Left)
            | Op::infix(Rule::or, Assoc::Left)
            | Op::infix(Rule::xor, Assoc::Left))
        .op(Op::infix(Rule::le, Assoc::Left)
            | Op::infix(Rule::lt, Assoc::Left)
            | Op::infix(Rule::ge, Assoc::Left)
            | Op::infix(Rule::gt, Assoc::Left)
            | Op::infix(Rule::ne, Assoc::Left)
            | Op::infix(Rule::eq, Assoc::Left))
        .op(Op::infix(Rule::bit_and, Assoc::Left)
            | Op::infix(Rule::bit_or, Assoc::Left)
            | Op::infix(Rule::bit_xor, Assoc::Left))
        .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left))
        .op(Op::infix(Rule::mul, Assoc::Left)
            | Op::infix(Rule::div, Assoc::Left)
            | Op::infix(Rule::idiv, Assoc::Left)
            | Op::infix(Rule::imod, Assoc::Left))
        .op(Op::prefix(Rule::not)
            | Op::prefix(Rule::neg)
            | Op::prefix(Rule::pos)
            | Op::prefix(Rule::bit_not)
            | Op::prefix(Rule::pre_incr)
            | Op::prefix(Rule::pre_decr))
        .op(Op::postfix(Rule::member)
            | Op::postfix(Rule::index)
            | Op::postfix(Rule::post_incr)
            | Op::postfix(Rule::post_decr))
}

#[derive(Debug)]
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

#[derive(Debug)]
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
