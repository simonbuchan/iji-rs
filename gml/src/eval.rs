use std::collections::HashMap;
use std::rc::Rc;

use thiserror::Error;

use super::ast;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}\n  at {}:{}", .1.line, .1.column)]
    WithPosition(Box<Error>, ast::Pos),
    #[error("unexpected exit")]
    Exit,
    #[error("unexpected return {0:?}")]
    Return(Value),
    #[error("attempted to assign to value expression")]
    AssignToValue,
    #[error("function {0:?} has no definition")]
    UndefinedFunction(String),
    #[error("invalid object id {0:?}")]
    InvalidObject(Value),
    #[error("invalid object id {0:?}")]
    InvalidId(i32),
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
}

impl<T> ResultExt<T> for Result<T> {
    fn with_position(self, pos: ast::Pos) -> Self {
        self.map_err(|error| Error::WithPosition(Box::new(error), pos))
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    Undefined,
    Bool(bool),
    Int(i32),
    Float(f64),
    String(String),
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

    pub fn to_bool(&self) -> bool {
        match self {
            Self::Undefined => false,
            Self::Bool(value) => *value,
            Self::Int(value) => *value != 0,
            Self::Float(value) => !value.is_nan() && *value != 0.0,
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

    pub fn to_id(&self) -> Result<ObjectId> {
        self.as_int()
            .and_then(|value| value.try_into().ok())
            .map(ObjectId)
            .ok_or(Error::InvalidObject(self.clone()))
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
        Self::Int(value.0 as i32)
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

pub trait Object {
    fn member(&self, name: &str) -> Result<Option<Value>> {
        Ok(None)
    }

    fn set_member(&mut self, name: &str, value: Value) -> Result {
        Ok(())
    }

    fn index(&self, args: &[Value]) -> Result<Option<Value>> {
        Ok(None)
    }

    fn set_index(&mut self, args: &[Value], value: Value) -> Result {
        Ok(())
    }
}

#[derive(Default)]
pub struct Namespace {
    pub vars: HashMap<String, Value>,
}

impl Object for Namespace {
    fn member(&self, name: &str) -> Result<Option<Value>> {
        Ok(self.vars.get(name).cloned())
    }

    fn set_member(&mut self, name: &str, value: Value) -> Result {
        self.vars.insert(name.into(), value);
        Ok(())
    }
}

#[derive(Clone)]
pub struct Function(Rc<dyn Fn(&mut Context, Vec<Value>) -> Result<Value>>);

impl Function {
    pub fn new(f: impl Fn(&mut Context, Vec<Value>) -> Result<Value> + 'static) -> Self {
        Self(Rc::new(f))
    }
}

pub struct Context {
    global: Namespace,
    local: Namespace,
    instances: HashMap<i32, Box<dyn Object>>,
    last_instance_id: i32,
    instance: ObjectId,
    fns: HashMap<String, Function>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            global: Namespace::default(),
            local: Namespace::default(),
            instances: Default::default(),
            last_instance_id: 0,
            instance: ObjectId::GLOBAL,
            fns: Default::default(),
        }
    }

    pub fn def_fn(
        &mut self,
        name: impl Into<String>,
        f: impl Fn(&mut Context, Vec<Value>) -> Result<Value> + 'static,
    ) {
        self.fns.insert(name.into(), Function::new(f));
    }

    pub fn new_instance(&mut self, value: Option<Box<dyn Object>>) -> ObjectId {
        self.last_instance_id += 1;
        let id = self.last_instance_id;
        self.instances
            .insert(id, value.unwrap_or_else(|| Box::new(Namespace::default())));
        ObjectId(id)
    }

    pub fn set_global(&mut self, name: impl Into<String>, value: impl Into<Value>) {
        self.global_mut().vars.insert(name.into(), value.into());
    }

    fn global(&self) -> &Namespace {
        &self.global
    }

    fn global_mut(&mut self) -> &mut Namespace {
        &mut self.global
    }

    fn instance(&self) -> &dyn Object {
        self.object(self.instance).unwrap()
    }

    fn instance_mut(&mut self) -> &mut dyn Object {
        self.object_mut(self.instance).unwrap()
    }

    fn local(&self) -> &Namespace {
        &self.local
    }

    fn local_mut(&mut self) -> &mut Namespace {
        &mut self.local
    }

    fn object(&self, id: ObjectId) -> Result<&dyn Object> {
        match id {
            ObjectId::GLOBAL => Ok(&self.global),
            ObjectId::LOCAL => Ok(&self.local),
            ObjectId(id) => self
                .instances
                .get(&id)
                .map(|b| &**b)
                .ok_or(Error::InvalidId(id)),
        }
    }

    fn object_mut(&mut self, id: ObjectId) -> Result<&mut dyn Object> {
        match id {
            ObjectId::GLOBAL => Ok(&mut self.global),
            ObjectId::LOCAL => Ok(&mut self.local),
            ObjectId(id) => self
                .instances
                .get_mut(&id)
                .map(|b| &mut **b)
                .ok_or(Error::InvalidId(id)),
        }
    }

    pub fn var(&self, var: &ast::Var) -> Value {
        match var {
            ast::Var::Global(id) => self.global().vars.get(id).cloned(),
            ast::Var::Local(id) => self
                .local()
                .vars
                .get(id)
                .cloned()
                .or_else(|| self.instance().member(&id).ok().flatten())
                .or_else(|| self.global().vars.get(id).cloned()),
        }
        .unwrap_or_default()
    }

    pub fn var_mut(&mut self, var: &ast::Var) -> &mut Value {
        match var {
            ast::Var::Global(id) => self.global_mut().vars.entry(id.clone()).or_default(),
            ast::Var::Local(id) => {
                use polonius_workaround::PoloniusExt as _;
                self.try_get_mut_with(|x| x.local_mut().vars.get_mut(id))
                    .unwrap_or_else(|x| x.instance_mut().vars.entry(id.clone()).or_default())
            }
        }
    }

    pub fn with_instance<R>(
        &mut self,
        instance: ObjectId,
        body: impl FnOnce(&mut Self) -> Result<R>,
    ) -> Result<R> {
        if instance.0 >= self.instances.len() {
            return Err(Error::InvalidId(instance));
        }

        let old = std::mem::replace(&mut self.instance, instance);
        let res = body(self);
        self.instance = old;
        res
    }

    fn place_value(&self, place: Place) -> Result<Value> {
        match place {
            Place::Value(value) => Ok(value),
            Place::Var(var) => Ok(self.var(&var)),
            Place::Property(id, name) => {
                let object = self.object(id)?;
                Ok(object.vars.get(&name).cloned().unwrap_or_default())
            }
            Place::Index(..) => Ok(().into()),
        }
    }

    pub fn exec_script(&mut self, script: &ast::Script) -> Result<Value> {
        let old_locals = std::mem::replace(self.local_mut(), Object::new(ObjectId::LOCAL));
        for stmt in &script.stmts {
            match self.exec(stmt) {
                Err(Error::Exit) => break,
                Err(Error::Return(value)) => return Ok(value),
                result => result,
            }?;
        }
        *self.local_mut() = old_locals;
        Ok(Value::Undefined)
    }

    pub fn exec(&mut self, stmt: &ast::Stmt) -> Result {
        match stmt {
            ast::Stmt::Expr { pos, expr } => {
                self.eval(expr).with_position(*pos)?;
            }
            ast::Stmt::Var(id) => {
                self.local_mut().vars.insert(id.clone(), ().into());
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
                let obj = self.eval(obj)?.to_id()?;
                self.with_instance(obj, |ctx| ctx.exec(body))?;
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

    fn exec_assign(&mut self, assign: &ast::Assign) -> Result<()> {
        let lhs = self.eval_place(&assign.lhs)?;
        let rhs = self.eval(&assign.rhs)?;
        let lhs = match lhs {
            Place::Value(_) => return Err(Error::AssignToValue),
            Place::Var(var) => self.var_mut(&var),
            Place::Property(id, name) => self.property(id, name)?,
            Place::Index(..) => {
                // println!("todo: assigning to index");
                return Ok(());
            }
        };
        match assign.op {
            ast::AssignOp::Assign => {
                *lhs = rhs;
            }
            // todo: Float, String, ...
            ast::AssignOp::AddAssign => {
                *lhs = (lhs.to_int() + rhs.to_int()).into();
            }
            ast::AssignOp::SubAssign => {
                *lhs = (lhs.to_int() - rhs.to_int()).into();
            }
            ast::AssignOp::MulAssign => {
                *lhs = (lhs.to_int() * rhs.to_int()).into();
            }
            ast::AssignOp::DivAssign => {
                *lhs = (lhs.to_int() / rhs.to_int()).into();
            }
        }
        Ok(())
    }

    pub fn eval(&mut self, expr: &ast::Expr) -> Result<Value> {
        let place = self.eval_place(expr)?;
        self.place_value(place)
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
                    ast::UnaryOp::Not => (!self.place_value(place)?.to_bool()).into(),
                    ast::UnaryOp::Pos => self.place_value(place)?.to_int().into(),
                    ast::UnaryOp::Neg => (-self.place_value(place)?.to_int()).into(),
                    ast::UnaryOp::BitNot => (!self.place_value(place)?.to_int()).into(),
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
                    // todo: coerce (e.g. "0" == 0)?
                    ast::BinaryOp::Ne => (lhs != rhs).into(),
                    ast::BinaryOp::Eq => (lhs == rhs).into(),
                    // todo: float, string?
                    ast::BinaryOp::Add => (lhs.to_int() + rhs.to_int()).into(),
                    ast::BinaryOp::Sub => (lhs.to_int() - rhs.to_int()).into(),
                    ast::BinaryOp::Mul => (lhs.to_int() * rhs.to_int()).into(),
                    ast::BinaryOp::Div => (lhs.to_int() / rhs.to_int()).into(),
                    ast::BinaryOp::IDiv => (lhs.to_int() / rhs.to_int()).into(),
                    ast::BinaryOp::IMod => (lhs.to_int() % rhs.to_int()).into(),
                };
                Ok(Place::Value(value))
            }
            ast::Expr::Member { lhs, name } => {
                let id = self.eval(lhs)?.to_id()?;
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
            ast::Expr::Call { pos, id, args } => {
                // println!("{line}:{column}: {id}()");
                let f = self
                    .fns
                    .get(id)
                    .ok_or(Error::UndefinedFunction(id.clone()))?
                    .clone();
                let args = args
                    .iter()
                    .map(|arg| self.eval(arg))
                    .collect::<Result<Vec<_>>>()
                    .with_position(*pos)?;
                Ok(Place::Value((f.0)(self, args).with_position(*pos)?))
            }
        }
    }
}
