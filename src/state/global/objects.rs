use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gml::eval::{Object, ObjectId, Value};
use macroquad::input::KeyCode;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use crate::state::Instance;

#[derive(Default)]
pub struct ObjectAsset {
    pub name: String,
    pub object: Rc<ObjectType>,
    pub events: HashMap<Event, Vec<Action>>,
    pub parent_index: Option<u32>,
}

impl Serialize for ObjectAsset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("ObjectAsset", 1)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("object", &*self.object)?;
        s.skip_field("events")?;
        s.serialize_field("parent_index", &self.parent_index)?;
        s.end()
    }
}

#[derive(Default)]
pub struct ObjectType {
    pub instances: RefCell<HashMap<ObjectId, Rc<Instance>>>,
}

impl Serialize for ObjectType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("ObjectType", 1)?;
        s.serialize_field(
            "instances",
            &self.instances.borrow().keys().collect::<Vec<_>>(),
        )?;
        s.end()
    }
}

impl Object for ObjectType {
    fn member(&self, name: &str) -> gml::eval::Result<Option<Value>> {
        let b = self.instances.borrow();
        // grab any instance
        let Some((_, instance)) = b.iter().next() else {
            return Ok(None);
        };
        instance.member(name)
    }

    fn set_member(&self, name: &str, value: Value) -> gml::eval::Result {
        for instance in self.instances.borrow().values() {
            instance.set_member(name, value.clone())?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize)]
pub enum Event {
    Create,
    StepBegin,
    StepNormal,
    StepEnd,
    Draw,
    Alarm(i32),
    Destroy,
    KeyPress(#[serde(skip)] KeyCode),
    KeyRelease(#[serde(skip)] KeyCode),
    KeyDown(#[serde(skip)] KeyCode),
    Collision(i32),
}

#[derive(Serialize)]
pub enum Action {
    ScriptInline(gml::Script),
    ScriptRes(u32),
    Bounce,
    SetAlarm(i32, i32),
    KillObject,
    Move(u32, f32),
    SetVariable(String, Box<gml::ast::Expr>),
}
