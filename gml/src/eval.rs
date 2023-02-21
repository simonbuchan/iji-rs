use std::collections::{hash_map, HashMap};
use std::rc::Rc;

use thiserror::Error;

use super::{ast, String};

#[derive(Debug, Error)]
pub enum Error {
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
    InvalidId(ObjectId),
    #[error("accessing property {name:?} on invalid object {id:?}")]
    UndefinedProperty { id: ObjectId, name: String },
    #[error("invalid repeat count {0:?}")]
    InvalidCount(Value),
    #[error("invalid condition {0:?}")]
    InvalidCondition(Value),
}

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(Clone, Debug)]
pub enum Value {
    Undefined,
    Bool(bool),
    Int(i32),
    Float(f64),
    String(String),
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
            Some(&*value)
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
            Self::Bool(value) => value.to_string().into(),
            Self::Int(value) => value.to_string().into(),
            Self::Float(value) => value.to_string().into(),
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

impl From<std::string::String> for Value {
    fn from(value: std::string::String) -> Self {
        Self::String(value.into())
    }
}

enum Place {
    Value(Value),
    Property(ObjectId, String),
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct ObjectId(usize);

impl ObjectId {
    const GLOBAL: ObjectId = ObjectId(0);
    const LOCAL: ObjectId = ObjectId(1);
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

pub struct Object {
    id: ObjectId,
    vars: HashMap<String, Value>,
}

impl Object {
    pub fn new(id: ObjectId) -> Self {
        Self {
            id,
            vars: Default::default(),
        }
    }
}

type Property<'a> = hash_map::Entry<'a, String, Value>;

#[derive(Clone)]
pub struct Function(Rc<dyn Fn(&mut Context, Vec<Value>) -> Result<Value>>);

impl Function {
    pub fn new(f: impl Fn(&mut Context, Vec<Value>) -> Result<Value> + 'static) -> Self {
        Self(Rc::new(f))
    }
}

pub struct Context {
    pub objects: Vec<Object>,
    pub instance: ObjectId,
    pub fns: HashMap<String, Function>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            objects: vec![Object::new(ObjectId::GLOBAL), Object::new(ObjectId::LOCAL)],
            instance: ObjectId::GLOBAL,
            fns: Default::default(),
        }
    }

    pub fn def_fn(&mut self, name: impl Into<String>, f: Function) {
        self.fns.insert(name.into(), f);
    }

    pub fn add_object(&mut self) -> ObjectId {
        let id = ObjectId(self.objects.len());
        self.objects.push(Object::new(id));
        id
    }

    pub fn global(&self) -> &Object {
        &self.objects[0]
    }

    pub fn global_mut(&mut self) -> &mut Object {
        &mut self.objects[0]
    }

    pub fn script_locals(&self) -> &Object {
        &self.objects[1]
    }

    pub fn script_locals_mut(&mut self) -> &mut Object {
        &mut self.objects[1]
    }

    pub fn instance(&self) -> &Object {
        &self.objects[self.instance.0]
    }

    pub fn instance_mut(&mut self) -> &mut Object {
        &mut self.objects[self.instance.0]
    }

    pub fn with_instance<R>(
        &mut self,
        instance: ObjectId,
        body: impl FnOnce(&mut Self) -> Result<R>,
    ) -> Result<R> {
        if instance.0 >= self.objects.len() {
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
            Place::Property(id, name) => {
                let object = self.object(id)?;
                Ok(object.vars.get(&name).cloned().unwrap_or_default())
            }
        }
    }

    fn object(&self, id: ObjectId) -> Result<&Object> {
        self.objects.get(id.0).ok_or(Error::InvalidId(id))
    }

    fn object_mut(&mut self, id: ObjectId) -> Result<&mut Object> {
        self.objects.get_mut(id.0).ok_or(Error::InvalidId(id))
    }

    fn property(&mut self, id: ObjectId, name: String) -> Result<Property<'_>> {
        Ok(self.object_mut(id)?.vars.entry(name))
    }

    pub fn exec_script(&mut self, script: &ast::Script) -> Result<Value> {
        let old_locals = std::mem::replace(self.script_locals_mut(), Object::new(ObjectId::LOCAL));
        for stmt in &script.stmts {
            match self.exec(stmt) {
                Err(Error::Exit) => break,
                Err(Error::Return(value)) => return Ok(value),
                result => result,
            }?;
        }
        *self.script_locals_mut() = old_locals;
        Ok(Value::Undefined)
    }

    pub fn exec(&mut self, stmt: &ast::Stmt) -> Result {
        match stmt {
            ast::Stmt::Expr(expr) => {
                self.eval(expr)?;
            }
            ast::Stmt::Var(_) => {}
            ast::Stmt::Assign(assign) => {
                self.exec_assign(assign)?;
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
            Place::Property(id, name) => self.property(id, name)?,
        };
        match assign.op {
            ast::AssignOp::Assign => {
                *lhs.or_default() = rhs;
            }
            _ => todo!(),
        }
        Ok(())
    }

    pub fn eval(&mut self, expr: &ast::Expr) -> Result<Value> {
        let place = self.eval_place(expr)?;
        self.place_value(place)
    }

    fn eval_place(&mut self, expr: &ast::Expr) -> Result<Place> {
        match expr {
            ast::Expr::Var(ast::Var::Global(name)) => {
                Ok(Place::Property(ObjectId::GLOBAL, name.clone()))
            }
            ast::Expr::Var(ast::Var::Local(name)) => {
                Ok(Place::Property(self.instance, name.clone()))
            }
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
                match op {
                    ast::BinaryOp::And => todo!(),
                    ast::BinaryOp::Or => todo!(),
                    ast::BinaryOp::Xor => todo!(),
                    ast::BinaryOp::BitAnd => todo!(),
                    ast::BinaryOp::BitOr => todo!(),
                    ast::BinaryOp::BitXor => todo!(),
                    ast::BinaryOp::Le => todo!(),
                    ast::BinaryOp::Lt => todo!(),
                    ast::BinaryOp::Ge => todo!(),
                    ast::BinaryOp::Gt => todo!(),
                    ast::BinaryOp::Ne => todo!(),
                    ast::BinaryOp::Eq => todo!(),
                    ast::BinaryOp::Add => todo!(),
                    ast::BinaryOp::Sub => todo!(),
                    ast::BinaryOp::Mul => todo!(),
                    ast::BinaryOp::Div => todo!(),
                    ast::BinaryOp::IDiv => todo!(),
                    ast::BinaryOp::IMod => todo!(),
                }
            }
            ast::Expr::Member { lhs, name } => {
                // determine lhs object id
                let id = match self.eval_place(lhs)? {
                    Place::Value(id) => id.to_id()?,
                    Place::Property(id, name) => {
                        // deref property id
                        let obj = self.object(id)?;
                        match obj.vars.get(&name) {
                            None => Err(Error::UndefinedProperty { id, name }),
                            Some(value) => value.to_id(),
                        }?
                    }
                };
                Ok(Place::Property(id, name.clone()))
            }
            ast::Expr::Index { lhs, indices } => {
                let lhs = self.eval_place(lhs)?;
                todo!()
            }
            ast::Expr::Call { id, args } => {
                let f = self
                    .fns
                    .get(id)
                    .ok_or(Error::UndefinedFunction(id.clone()))?
                    .clone();
                let args = args
                    .iter()
                    .map(|arg| self.eval(arg))
                    .collect::<Result<Vec<_>>>()?;
                Ok(Place::Value((f.0)(self, args)?))
            }
        }
    }
}
