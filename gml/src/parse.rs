use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest::Parser as _;
use pest_derive::Parser;

use super::ast::*;

#[derive(Parser)]
#[grammar = "gml.pest"]
struct G;

#[allow(dead_code)]
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

pub fn parse(name: &str, input: &str) -> anyhow::Result<Script> {
    let name = name.to_string();
    let pairs = G::parse(Rule::script, input)?;
    let mut stmts = vec![];
    for pair in pairs {
        if pair.as_rule() == Rule::EOI {
            break;
        }
        stmts.push(parse_stmt(pair));
    }
    Ok(Script { name, stmts })
}

#[allow(dead_code)]
pub fn dump_parse(input: &str) -> anyhow::Result<()> {
    let pairs = G::parse(Rule::script, input)?;
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

fn parse_stmt(pair: Pair<'_, Rule>) -> Box<Stmt> {
    match pair.as_rule() {
        Rule::if_stmt => {
            let mut inner = pair.into_inner();
            // initially
            //    kw_if expr stmt (kw_else stmt)*
            // but that can overflow the stack, so it's flattened to:
            //    kw_if expr stmt (kw_else kw_if expr stmt)* (kw_else stmt)
            assert_eq!(inner.next().unwrap().as_rule(), Rule::kw_if);
            let cond = parse_expr(inner.next().unwrap());
            let body = parse_stmt(inner.next().unwrap());
            let mut alt = None;
            let mut alts = vec![];
            // Now we should always be on an else...
            while let Some(kw) = inner.next() {
                assert_eq!(kw.as_rule(), Rule::kw_else);
                let next = inner.next().unwrap();
                // but if it's followed by another if...
                if next.as_rule() == Rule::kw_if {
                    // we push the left-hand onto a stack...
                    let cond = parse_expr(inner.next().unwrap());
                    let body = parse_stmt(inner.next().unwrap());
                    alts.push((cond, body));
                } else {
                    // and otherwise we're done and have the right hand...
                    alt = Some(parse_stmt(next));
                    assert!(inner.next().is_none());
                }
            }
            // now, build up else-ifs from the right...
            while let Some((cond, body)) = alts.pop() {
                alt = Some(Box::new(Stmt::If { cond, body, alt }));
            }
            // so now alt is the left-most else body, if any.
            Box::new(Stmt::If { cond, body, alt })
        }
        Rule::repeat_stmt => {
            let mut inner = pair.into_inner();
            assert_eq!(inner.next().unwrap().as_rule(), Rule::kw_repeat);
            let count = parse_expr(inner.next().unwrap());
            let stmt = parse_stmt(inner.next().unwrap());
            Box::new(Stmt::Repeat { count, body: stmt })
        }
        Rule::while_stmt => {
            let mut inner = pair.into_inner();
            assert_eq!(inner.next().unwrap().as_rule(), Rule::kw_while);
            let cond = parse_expr(inner.next().unwrap());
            let body = parse_stmt(inner.next().unwrap());
            Box::new(Stmt::While { cond, body })
        }
        Rule::for_stmt => {
            let mut inner = pair.into_inner();
            assert_eq!(inner.next().unwrap().as_rule(), Rule::kw_for);
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
            assert_eq!(inner.next().unwrap().as_rule(), Rule::kw_with);
            let obj = parse_expr(inner.next().unwrap());
            let body = parse_stmt(inner.next().unwrap());
            Box::new(Stmt::With { obj, body })
        }
        Rule::return_stmt => {
            let mut inner = pair.into_inner();
            assert_eq!(inner.next().unwrap().as_rule(), Rule::kw_return);
            let expr = parse_expr(inner.next().unwrap());
            Box::new(Stmt::Return { expr })
        }
        Rule::exit_stmt => Box::new(Stmt::Exit),
        Rule::block_stmt => {
            let inner = pair.into_inner();
            let stmts = inner.map(parse_stmt).collect();
            Box::new(Stmt::Block { stmts })
        }
        Rule::var_stmt => {
            let mut inner = pair.into_inner();
            assert_eq!(inner.next().unwrap().as_rule(), Rule::kw_var);
            let id = inner.next().unwrap().as_str().into();
            Box::new(Stmt::Var(id))
        }
        Rule::assign_stmt => {
            let pos = Pos::from(pair.line_col());
            let mut inner = pair.into_inner();
            let assign = parse_assign(inner.next().unwrap());
            Box::new(Stmt::Assign { pos, assign })
        }
        Rule::expr_stmt => {
            let pos = Pos::from(pair.line_col());
            let mut inner = pair.into_inner();
            let expr = parse_expr(inner.next().unwrap());
            Box::new(Stmt::Expr { pos, expr })
        }
        Rule::empty_stmt => Box::new(Stmt::Empty),
        _ => unreachable!("bad stmt: {pair:?}"),
    }
}

fn parse_var(pair: Pair<'_, Rule>) -> Var {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap();
    match id.as_rule() {
        Rule::global => Var::Global(inner.next().unwrap().as_str().into()),
        Rule::id => Var::Local(id.as_str().into()),
        _ => unreachable!("bad var: {id:?}"),
    }
}

fn parse_assign_lhs(pair: Pair<'_, Rule>) -> Box<Expr> {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap();
    let mut lhs = match id.as_rule() {
        Rule::var => Box::new(Expr::Var(parse_var(id))),
        Rule::assign_id => {
            let mut inner = id.into_inner();
            return parse_expr(inner.next().unwrap());
        }
        _ => unreachable!("bad assign lhs: {id:?}"),
    };
    for op in inner {
        match op.as_rule() {
            Rule::member => {
                let mut inner = op.into_inner();
                let name = inner.next().unwrap().as_str().into();
                lhs = Box::new(Expr::Member { lhs, name })
            }
            Rule::index => {
                let inner = op.into_inner();
                let indices = inner.map(parse_expr).collect();
                lhs = Box::new(Expr::Index { lhs, indices })
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

fn parse_expr(pair: Pair<'_, Rule>) -> Box<Expr> {
    parse_expr_rec(pair, &pratt())
}

fn parse_expr_rec(pair: Pair<'_, Rule>, pratt: &PrattParser<Rule>) -> Box<Expr> {
    assert_eq!(pair.as_rule(), Rule::expr);
    pratt
        .map_primary(|primary| {
            match primary.as_rule() {
                Rule::expr => parse_expr_rec(primary, pratt),
                Rule::call_expr => {
                    let pos = Pos::from(primary.line_col());
                    let mut inner = primary.into_inner();
                    let id = inner.next().unwrap().as_str().into();
                    let args = inner.map(|pair| parse_expr_rec(pair, pratt)).collect();
                    Box::new(Expr::Call {
                        pos,
                        name: id,
                        args,
                    })
                }
                Rule::var => Box::new(Expr::Var(parse_var(primary))),
                Rule::int => Box::new(Expr::Int(primary.as_str().parse().unwrap())),
                Rule::float => Box::new(Expr::Float(primary.as_str().parse().unwrap())),
                Rule::str => {
                    let source = primary.as_str();
                    // trim quotes
                    let source = &source[1..source.len() - 1];
                    Box::new(Expr::String(source.into()))
                }
                _ => unreachable!("bad primary: {primary:?}"),
            }
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
                let name = inner.next().unwrap().as_str().into();
                Box::new(Expr::Member { lhs: expr, name })
            }
            Rule::index => {
                let inner = op.into_inner();
                let indices = inner.map(|pair| parse_expr_rec(pair, pratt)).collect();
                Box::new(Expr::Index { lhs: expr, indices })
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

#[cfg(test)]
mod tests {
    use super::{Rule, G};
    use pest::{consumes_to, parses_to};

    #[test]
    fn test_grammar() {
        parses_to! {
            parser: G,
            input: "",
            rule: Rule::script,
            tokens: []
        }
        parses_to! {
            parser: G,
            //      0     6 7  10
            input: "// foo\nbar",
            rule: Rule::script,
            tokens: [
                expr_stmt(7, 10, [
                    expr(7, 10, [
                        var(7, 10, [
                            id(7, 10)
                        ])
                    ]),
                    EOI(10, 10)
                ])
            ]
        }
    }
}
