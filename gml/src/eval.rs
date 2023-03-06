use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use thiserror::Error;

use super::ast;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}\n  at {}:{}", .1.line, .1.column)]
    WithPosition(Box<Error>, ast::Pos),
    #[error("{0}\n  in {1}")]
    WithScriptName(Box<Error>, String),
    #[error("{0}")]
    Custom(String),
    #[error("unexpected exit")]
    Exit,
    #[error("unexpected return {0:?}")]
    Return(Value),
    #[error("attempted to assign to value expression")]
    AssignToValue,
    #[error("function {0:?} has no definition")]
    UndefinedFunction(String),
    #[error("invalid bool {0}")]
    InvalidOperands(Value, Value),
    #[error("invalid bool {0}")]
    InvalidBool(Value),
    #[error("invalid int {0}")]
    InvalidInt(Value),
    #[error("invalid float {0}")]
    InvalidFloat(Value),
    #[error("invalid string {0:?}")]
    InvalidString(Value),
    #[error("invalid object id {0}")]
    InvalidObject(Value),
    #[error("accessing property {name:?} on invalid place {place:?}")]
    UndefinedProperty { place: Place, name: String },
    #[error("invalid repeat count {0:?}")]
    InvalidCount(Value),
    #[error("invalid condition {0:?}")]
    InvalidCondition(Value),
}

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

trait ResultExt<T>: Sized {
    fn with_position(self, pos: ast::Pos) -> Self;
    fn with_script_name(self, name: String) -> Self;
}

impl<T> ResultExt<T> for Result<T> {
    fn with_position(self, pos: ast::Pos) -> Self {
        self.map_err(|error| Error::WithPosition(Box::new(error), pos))
    }

    fn with_script_name(self, name: String) -> Self {
        self.map_err(|error| Error::WithScriptName(Box::new(error), name))
    }
}

/// Values are any possible immutable result of evaluating an expression.
/// They cannot explicitly reference an object, but may contain an integer
/// that can be coerced to an object id in the context of an assignment.
#[derive(Clone, Debug)]
pub enum Value {
    Undefined,
    Bool(bool),
    Int(i32),
    Float(f64),
    String(String),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Undefined, Self::Undefined) => true,
            (Self::String(lhs), Self::String(rhs)) => lhs == rhs,
            (Self::Bool(lhs), Self::Bool(rhs)) => lhs == rhs,
            (Self::Int(lhs), Self::Int(rhs)) => lhs == rhs,
            (Self::Float(lhs), rhs) => lhs == &rhs.to_float(),
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Undefined, Self::Undefined) => Some(std::cmp::Ordering::Equal),
            (Self::String(lhs), Self::String(rhs)) => lhs.partial_cmp(rhs),
            (Self::Bool(lhs), Self::Bool(rhs)) => lhs.partial_cmp(rhs),
            (Self::Int(lhs), Self::Int(rhs)) => lhs.partial_cmp(rhs),
            (Self::Float(lhs), rhs) => lhs.partial_cmp(&rhs.to_float()),
            _ => None,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Undefined => write!(f, "undefined"),
            Value::Bool(value) => write!(f, "{value}"),
            Value::Int(value) => write!(f, "{value}"),
            Value::Float(value) => write!(f, "{value:.1}"),
            Value::String(value) => write!(f, "{value:?}"),
        }
    }
}

impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    pub fn as_int(&self) -> Option<i32> {
        if let Self::Int(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        if let Self::Float(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub fn as_object_id(&self) -> Option<ObjectId> {
        if let Self::Int(value) = self {
            Some(ObjectId(*value))
        } else {
            None
        }
    }

    pub fn to_bool(&self) -> bool {
        match self {
            Self::Undefined => false,
            Self::Bool(value) => *value,
            Self::Int(value) => *value != 0,
            Self::Float(value) => !value.is_nan() && *value > 0.5,
            Self::String(value) => !value.is_empty(),
        }
    }

    pub fn to_int(&self) -> i32 {
        match self {
            Self::Undefined => 0,
            Self::Bool(value) => *value as i32,
            Self::Int(value) => *value,
            Self::Float(value) => *value as i32,
            Self::String(value) => value.parse().unwrap_or_default(),
        }
    }

    pub fn to_float(&self) -> f64 {
        match self {
            Self::Undefined => 0.0,
            Self::Bool(value) => *value as i32 as f64,
            Self::Int(value) => *value as f64,
            Self::Float(value) => *value,
            Self::String(value) => value.parse().unwrap_or_default(),
        }
    }

    pub fn to_str(&self) -> String {
        match self {
            Self::Undefined => "".into(),
            Self::Bool(value) => value.to_string(),
            Self::Int(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::String(value) => value.clone(),
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::Undefined
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Self::Undefined
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Int(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<ObjectId> for Value {
    fn from(value: ObjectId) -> Self {
        Self::Int(value.0)
    }
}

impl std::ops::Add for Value {
    type Output = Result<Value>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Int(lhs), Self::Int(rhs)) => Ok((lhs + rhs).into()),
            (lhs, Self::String(value)) => Ok((lhs.to_str() + &value).into()),
            (lhs @ Self::String(_), rhs) => Err(Error::InvalidOperands(lhs, rhs)),
            (lhs, rhs) => Ok((lhs.to_float() + rhs.to_float()).into()),
        }
    }
}

impl std::ops::Sub for Value {
    type Output = Result<Value>;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Int(lhs), Self::Int(rhs)) => Ok((lhs - rhs).into()),
            (lhs, rhs @ Self::String(_)) | (lhs @ Self::String(_), rhs) => {
                Err(Error::InvalidOperands(lhs, rhs))
            }
            (lhs, rhs) => Ok((lhs.to_float() - rhs.to_float()).into()),
        }
    }
}

impl std::ops::Mul for Value {
    type Output = Result<Value>;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Int(lhs), Self::Int(rhs)) => Ok((lhs * rhs).into()),
            (lhs @ Self::Int(_) | lhs @ Self::Float(_), Self::String(rhs)) => {
                let count = lhs.to_int().try_into().unwrap_or_default();
                Ok(rhs.repeat(count).into())
            }
            (lhs @ Self::String(_), rhs) => Err(Error::InvalidOperands(lhs, rhs)),
            (lhs, rhs) => Ok((lhs.to_float() * rhs.to_float()).into()),
        }
    }
}

impl std::ops::Div for Value {
    type Output = Result<Value>;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Int(lhs), Self::Int(rhs)) => Ok((lhs / rhs).into()),
            (lhs @ Self::String(_), rhs) | (lhs, rhs @ Self::String(_)) => {
                Err(Error::InvalidOperands(lhs, rhs))
            }
            (lhs, rhs) => Ok((lhs.to_float() / rhs.to_float()).into()),
        }
    }
}

impl std::ops::Rem for Value {
    type Output = Result<Value>;

    fn rem(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Int(lhs), Self::Int(rhs)) => Ok((lhs % rhs).into()),
            (lhs @ Self::String(_), rhs) | (lhs, rhs @ Self::String(_)) => {
                Err(Error::InvalidOperands(lhs, rhs))
            }
            (lhs, rhs) => Ok((lhs.to_float() % rhs.to_float()).into()),
        }
    }
}

#[derive(Debug)]
pub enum Place {
    Value(Value),
    Var(ast::Var),
    Property(ObjectId, String),
    Index(Box<Place>, Vec<Value>),
}

#[derive(Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct ObjectId(i32);

impl ObjectId {
    const GLOBAL: Self = Self(0);
    const LOCAL: Self = Self(-1);

    pub fn new(id: u32) -> Self {
        Self(id.try_into().expect("invalid object id"))
    }

    pub fn instance_id(&self) -> u32 {
        self.0.try_into().expect("not an instance object id")
    }
}

impl std::fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Self::GLOBAL {
            write!(f, "<global>")
        } else if *self == Self::LOCAL {
            write!(f, "<local>")
        } else {
            write!(f, "<#{}>", self.0)
        }
    }
}

#[allow(unused_variables)]
pub trait Global: 'static {
    fn get(&self, name: &str) -> Result<Option<Value>>;

    fn set(&self, name: &str, value: Value) -> Result;

    fn instance(&self, id: ObjectId) -> Option<Rc<dyn Object>>;

    fn new_instance(&self, object: Rc<dyn Object>) -> ObjectId;

    fn call(&self, context: &mut Context<'_>, id: &str, args: Vec<Value>) -> Result<Value>;
}

#[allow(unused_variables)]
pub trait Object: 'static {
    fn member(&self, name: &str) -> Result<Option<Value>> {
        Ok(None)
    }

    fn set_member(&self, name: &str, value: Value) -> Result {
        Ok(())
    }

    fn index(&self, args: &[Value]) -> Result<Option<Value>> {
        Ok(None)
    }

    fn set_index(&self, args: &[Value], value: Value) -> Result {
        Ok(())
    }
}

#[derive(Default)]
pub struct Namespace {
    vars: RefCell<HashMap<String, Value>>,
}

impl Namespace {
    pub fn get(&self, name: &str) -> Option<Value> {
        self.vars.borrow().get(name).cloned()
    }

    pub fn insert(&self, name: impl Into<String>, value: impl Into<Value>) {
        self.vars.borrow_mut().insert(name.into(), value.into());
    }
}

impl std::fmt::Debug for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.vars.borrow().fmt(f)
    }
}

impl Object for Namespace {
    fn member(&self, name: &str) -> Result<Option<Value>> {
        Ok(self.get(name))
    }

    fn set_member(&self, name: &str, value: Value) -> Result {
        self.insert(name, value);
        Ok(())
    }
}

#[derive(Default)]
pub struct Array {
    items: RefCell<Vec<Value>>,
}

impl Object for Array {
    fn index(&self, args: &[Value]) -> Result<Option<Value>> {
        let index = args.get(0).cloned().unwrap_or_default().to_int();
        Ok(index
            .try_into()
            .ok()
            .and_then(|index: usize| self.items.borrow().get(index).cloned()))
    }

    fn set_index(&self, args: &[Value], value: Value) -> Result {
        let Ok(index) = args.get(0).cloned().unwrap_or_default().to_int().try_into() else {
            return Ok(())
        };
        let mut items = self.items.borrow_mut();

        if items.len() <= index {
            items.resize(index + 1, Value::Undefined);
        }
        items[index] = value;
        Ok(())
    }
}

type FunctionImpl = dyn Fn(&mut Context, Vec<Value>) -> Result<Value>;

pub struct Function(Rc<FunctionImpl>);

impl Function {
    pub fn new(f: impl Fn(&mut Context, Vec<Value>) -> Result<Value> + 'static) -> Self {
        Self(Rc::new(f))
    }
}

impl Clone for Function {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct Context<'a> {
    pub global: &'a dyn Global,
    pub instance_id: ObjectId,
    pub instance: Rc<dyn Object>,
    pub locals: Namespace,
}

impl<'a> Context<'a> {
    pub fn new(global: &'a dyn Global, instance_id: ObjectId, instance: Rc<dyn Object>) -> Self {
        Self {
            global,
            instance_id,
            instance,
            locals: Namespace::default(),
        }
    }

    fn instance(&self, id: ObjectId) -> Result<Rc<dyn Object>> {
        self.global
            .instance(id)
            .ok_or(Error::InvalidObject(id.into()))
    }

    pub fn get(&self, id: ObjectId, name: &str) -> Result<Option<Value>> {
        match id {
            ObjectId::GLOBAL => Ok(self.global.get(name)?),
            ObjectId::LOCAL => Ok(self.locals.member(name)?),
            id => Ok(self.instance(id)?.member(name)?),
        }
    }

    pub fn set(&mut self, id: ObjectId, name: &str, value: Value) -> Result {
        match id {
            ObjectId::GLOBAL => {
                self.global.set(name, value)?;
            }
            ObjectId::LOCAL => {
                self.locals.set_member(name, value)?;
            }
            id => {
                self.instance(id)?.set_member(name, value)?;
            }
        }
        Ok(())
    }

    pub fn var(&mut self, var: &ast::Var) -> Result<Value> {
        // Local tries script locals, then active instance, then global.
        // Global only tries global.
        match var {
            ast::Var::Global(id) => Ok(self.global.get(id)?.unwrap_or_default()),
            ast::Var::Local(id) => {
                if let Some(value) = self.locals.member(id)? {
                    return Ok(value);
                }
                if let Some(value) = self.instance.member(id)? {
                    return Ok(value);
                }
                if let Some(value) = self.global.get(id)? {
                    return Ok(value);
                }
                // println!("note: reading undefined local: {id}");
                Ok(Value::Undefined)
            }
        }
    }

    pub fn set_var(&mut self, var: &ast::Var, value: Value) -> Result {
        // Local sets script local if it exists, otherwise it sets on active instance.
        // It will not fall back to a global.
        match var {
            ast::Var::Global(id) => {
                self.global.set(id, value)?;
            }
            ast::Var::Local(id) => {
                if self.locals.get(id).is_some() {
                    self.locals.set_member(id, value)?;
                } else {
                    self.instance.set_member(id, value)?;
                }
            }
        }
        Ok(())
    }

    fn place_value(&mut self, place: &Place) -> Result<Value> {
        match place {
            Place::Value(value) => Ok(value.clone()),
            Place::Var(var) => self.var(var),
            Place::Property(id, name) => {
                let object = self.get(*id, name)?;
                Ok(object.unwrap_or_default())
            }
            Place::Index(lhs, indices) => {
                let lhs = self.place_value(lhs)?;
                match lhs {
                    Value::Undefined => Ok(Value::Undefined),
                    Value::Int(lhs_id) => {
                        // lhs cannot be LOCAL or GLOBAL
                        let lhs = self.instance(ObjectId(lhs_id))?;
                        Ok(lhs.index(indices)?.unwrap_or_default())
                    }
                    Value::String(value) => {
                        let index = 0
                            .max(indices[0].to_int() - 1)
                            .try_into()
                            .unwrap_or_default();
                        let value = value.get(index..).unwrap_or_default();
                        Ok(value.to_string().into())
                    }
                    lhs => Err(Error::InvalidObject(lhs)),
                }
            }
        }
    }

    fn set_place(&mut self, place: &Place, value: Value) -> Result {
        match place {
            Place::Value(_) => return Err(Error::AssignToValue),
            Place::Var(var) => {
                self.set_var(var, value)?;
            }
            Place::Property(id, name) => {
                self.set(*id, name, value)?;
            }
            Place::Index(lhs_place, indices) => {
                // need to be careful here, in `foo[123] = bar`
                // foo may not be defined.
                let lhs_value = self.place_value(lhs_place)?;
                let lhs_id = if matches!(lhs_value, Value::Undefined) {
                    let id = self.global.new_instance(Rc::<Array>::default());
                    self.set_place(lhs_place, id.0.into())?;
                    id
                } else {
                    ObjectId(lhs_value.as_int().ok_or(Error::InvalidObject(lhs_value))?)
                };
                // lhs_id cannot be LOCAL or GLOBAL
                let lhs = self.instance(lhs_id)?;
                lhs.set_index(indices, value)?;
            }
        }
        Ok(())
    }

    pub fn exec_script(&mut self, script: &ast::Script, arguments: &[Value]) -> Result<Value> {
        let old_locals = std::mem::take(&mut self.locals);
        for (index, value) in arguments.iter().enumerate() {
            self.locals
                .set_member(&format!("argument{index}"), value.clone())?;
        }
        for stmt in &script.stmts {
            match self.exec(stmt) {
                Err(Error::Exit) => break,
                Err(Error::Return(value)) => return Ok(value),
                result => result.with_script_name(script.name.clone()),
            }?;
        }
        self.locals = old_locals;
        Ok(Value::Undefined)
    }

    pub fn exec(&mut self, stmt: &ast::Stmt) -> Result {
        match stmt {
            ast::Stmt::Expr { pos, expr } => {
                self.eval(expr).with_position(*pos)?;
            }
            ast::Stmt::Var(id) => {
                // var foo; ensures there is an entry in locals, so later references use it.
                self.locals.set_member(id, ().into())?;
            }
            ast::Stmt::Assign { pos, assign } => {
                self.exec_assign(assign).with_position(*pos)?;
            }
            ast::Stmt::If { cond, body, alt } => {
                if self.eval(cond)?.to_bool() {
                    self.exec(body)?;
                } else if let Some(alt) = alt {
                    self.exec(alt)?;
                }
            }
            ast::Stmt::Repeat { count, body } => {
                let count = self.eval(count)?.to_int();
                for _ in 0..count {
                    self.exec(body)?;
                }
            }
            ast::Stmt::While { cond, body } => loop {
                if !self.eval(cond)?.to_bool() {
                    break;
                }
                self.exec(body)?;
            },
            ast::Stmt::For {
                assign,
                cond,
                update,
                body,
            } => {
                self.exec_assign(assign)?;
                loop {
                    if !self.eval(cond)?.to_bool() {
                        break;
                    }
                    self.exec(body)?;
                    self.exec_assign(update)?;
                }
            }
            ast::Stmt::With { obj, body } => {
                // todo: obj can be a *set* of objects, which this then loops over.
                //       this is used by Iji at least in the scr_firekey "null driver" weapon.
                let value = self.eval(obj)?;
                let id = value.as_object_id().ok_or(Error::InvalidObject(value))?;
                let new_instance = self.instance(id)?;
                self.with_instance(id, new_instance, |ctx| ctx.exec(body))?;
            }
            ast::Stmt::Return { expr } => {
                let value = self.eval(expr)?;
                return Err(Error::Return(value));
            }
            ast::Stmt::Exit => return Err(Error::Exit),
            ast::Stmt::Block { stmts } => {
                for stmt in stmts {
                    self.exec(stmt)?;
                }
            }
            ast::Stmt::Empty => {}
        }
        Ok(())
    }

    pub fn with_instance<F: FnOnce(&mut Self) -> R, R>(
        &mut self,
        instance_id: ObjectId,
        instance: Rc<dyn Object>,
        body: F,
    ) -> R {
        let old_instance_id = std::mem::replace(&mut self.instance_id, instance_id);
        let old_instance = std::mem::replace(&mut self.instance, instance);
        let result = body(self);
        self.instance = old_instance;
        self.instance_id = old_instance_id;
        result
    }

    fn exec_assign(&mut self, assign: &ast::Assign) -> Result<()> {
        let lhs_place = self.eval_place(&assign.lhs)?;
        let rhs = self.eval(&assign.rhs)?;
        let value = match assign.op {
            ast::AssignOp::Assign => rhs,
            op => {
                let lhs = self.place_value(&lhs_place)?;
                match op {
                    ast::AssignOp::Assign => unreachable!(),
                    ast::AssignOp::AddAssign => (lhs + rhs)?,
                    ast::AssignOp::SubAssign => (lhs - rhs)?,
                    ast::AssignOp::MulAssign => (lhs * rhs)?,
                    ast::AssignOp::DivAssign => (lhs / rhs)?,
                }
            }
        };
        self.set_place(&lhs_place, value)?;
        Ok(())
    }

    pub fn eval(&mut self, expr: &ast::Expr) -> Result<Value> {
        let place = self.eval_place(expr)?;
        self.place_value(&place)
    }

    fn eval_place(&mut self, expr: &ast::Expr) -> Result<Place> {
        match expr {
            ast::Expr::Var(var) => Ok(Place::Var(var.clone())),
            ast::Expr::Int(value) => Ok(Place::Value(Value::Int(*value))),
            ast::Expr::Float(value) => Ok(Place::Value(Value::Float(*value))),
            ast::Expr::String(value) => Ok(Place::Value(Value::String(value.clone()))),
            ast::Expr::Unary { op, expr } => {
                let place = self.eval_place(expr)?;
                let value = match op {
                    ast::UnaryOp::Not => (!self.place_value(&place)?.to_bool()).into(),
                    ast::UnaryOp::Pos => self.place_value(&place)?.to_int().into(),
                    ast::UnaryOp::Neg => (-self.place_value(&place)?.to_int()).into(),
                    ast::UnaryOp::BitNot => (!self.place_value(&place)?.to_int()).into(),
                    ast::UnaryOp::PreIncr => todo!(),
                    ast::UnaryOp::PreDecr => todo!(),
                    ast::UnaryOp::PostIncr => todo!(),
                    ast::UnaryOp::PostDecr => todo!(),
                };
                Ok(Place::Value(value))
            }
            ast::Expr::Binary { lhs, op, rhs } => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                let value = match op {
                    ast::BinaryOp::And => (lhs.to_bool() && rhs.to_bool()).into(),
                    ast::BinaryOp::Or => (lhs.to_bool() || rhs.to_bool()).into(),
                    ast::BinaryOp::Xor => (lhs.to_bool() != rhs.to_bool()).into(),
                    ast::BinaryOp::BitAnd => (lhs.to_int() & rhs.to_int()).into(),
                    ast::BinaryOp::BitOr => (lhs.to_int() | rhs.to_int()).into(),
                    ast::BinaryOp::BitXor => (lhs.to_int() ^ rhs.to_int()).into(),
                    ast::BinaryOp::Le => (lhs <= rhs).into(),
                    ast::BinaryOp::Lt => (lhs < rhs).into(),
                    ast::BinaryOp::Ge => (lhs >= rhs).into(),
                    ast::BinaryOp::Gt => (lhs > rhs).into(),
                    ast::BinaryOp::Ne => (lhs != rhs).into(),
                    ast::BinaryOp::Eq => (lhs == rhs).into(),
                    ast::BinaryOp::Add => (lhs + rhs)?,
                    ast::BinaryOp::Sub => (lhs - rhs)?,
                    ast::BinaryOp::Mul => (lhs * rhs)?,
                    ast::BinaryOp::Div => (lhs / rhs)?,
                    ast::BinaryOp::IDiv => (lhs.to_int() / rhs.to_int()).into(),
                    ast::BinaryOp::IMod => (lhs % rhs)?,
                };
                Ok(Place::Value(value))
            }
            ast::Expr::Member { lhs, name } => {
                let value = self.eval(lhs)?;
                let id = value.as_object_id().ok_or(Error::InvalidObject(value))?;
                Ok(Place::Property(id, name.clone()))
            }
            ast::Expr::Index { lhs, indices } => {
                let lhs = self.eval_place(lhs)?.into();
                let indices = indices
                    .iter()
                    .map(|index| self.eval(index))
                    .collect::<Result<Vec<_>>>()?;
                Ok(Place::Index(lhs, indices))
            }
            ast::Expr::Call {
                pos,
                name: id,
                args,
            } => {
                // println!("{line}:{column}: {id}()");
                let args = args
                    .iter()
                    .map(|arg| self.eval(arg))
                    .collect::<Result<Vec<_>>>()
                    .with_position(*pos)?;
                let result = self.global.call(self, id, args).with_position(*pos)?;
                Ok(Place::Value(result))
            }
        }
    }
}
