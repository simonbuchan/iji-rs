use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic;
use std::sync::atomic::AtomicU32;

use glam::{ivec2, IVec2};
use gml::eval::{Object, ObjectId, Value};
use gml::Context;
use macroquad::color::WHITE;
use serde::Serialize;

use super::*;
use crate::assets::{Assets, Loader};

pub use fonts::FontAsset;
pub use objects::{Action, Event, ObjectAsset, ObjectType};

mod fonts;
mod objects;

#[derive(Serialize)]
pub struct Global {
    #[serde(skip)]
    pub content: gmk_file::Content,
    pub assets: RefCell<Assets>,
    pub object_types: HashMap<u32, ObjectAsset>,
    pub consts: gml::eval::Namespace,
    pub vars: gml::eval::Namespace,
    #[serde(skip)]
    pub scripts: DoubleMap<gml::ast::Script>,
    pub room_order_index: RefCell<usize>,
    pub room: RefCell<Room>,
    pub next_room_index: RefCell<Option<u32>>,
    pub state: RefCell<GlobalState>,
    pub last_instance_id: AtomicU32,
}

impl std::fmt::Debug for Global {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Global")
            .field("vars", &self.vars)
            .field("room", &self.room)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl Global {
    pub fn new(content: gmk_file::Content) -> Self {
        let consts = define_consts(&content);
        let object_types = define_objects(&content);
        let scripts = define_scripts(&content);
        let last_instance_id = AtomicU32::new(content.last_instance_id);

        Self {
            content,
            assets: default(),
            object_types,
            consts,
            vars: default(),
            scripts,
            room_order_index: RefCell::new(0),
            room: RefCell::new(Room::new()),
            next_room_index: default(),
            state: default(),
            last_instance_id,
        }
    }

    pub fn content(&self) -> &gmk_file::Content {
        &self.content
    }

    pub fn loader(&self) -> Loader<'_> {
        Loader::new(&self.content, &self.assets)
    }

    pub fn assets(&self) -> Ref<'_, Assets> {
        self.assets.borrow()
    }

    pub fn next_instance_id(&self) -> u32 {
        self.last_instance_id
            .fetch_add(1, atomic::Ordering::Relaxed)
    }

    pub fn room_goto_next(&self) {
        let mut room_order_index = self.room_order_index.borrow_mut();
        *room_order_index += 1;
        let index = *room_order_index;
        drop(room_order_index);
        self.goto_room_order(index);
    }

    pub fn goto_room_order(&self, order_index: usize) {
        let room_index = self.content.room_order.items[order_index];
        self.goto_room(room_index);
    }

    pub fn goto_room(&self, index: u32) {
        let def = &self.content.rooms[index];
        assert_eq!(&*def.creation_code, "");
        let Ok(mut room) = self.room.try_borrow_mut() else {
            *self.next_room_index.borrow_mut() = Some(index);
            return;
        };
        *room = Room::new();
        room.load(self, def);
        // drop mut borrow because instance_create() wants to borrow room
        drop(room);

        for res in &def.instances {
            assert_eq!(&*res.creation_code, "");
            self.instance_create(
                ObjectId::new(res.id),
                ivec2(res.pos.0, res.pos.1),
                res.object_index,
            );
        }
        let room = self.room.borrow();
        room.object_instances.borrow_mut().values =
            std::mem::take(&mut room.added_instances.borrow_mut());

        // hack to work around obj_menuback Step event script spamming errors.
        let face_dummy = Rc::<gml::eval::Namespace>::default();
        face_dummy.insert("count", 0);
        let dummy = self.new_instance(face_dummy);
        self.vars.insert("face", dummy);

        self.dispatch(Event::Create);
    }

    pub fn dispatch(&self, event: Event) {
        self.room.borrow().dispatch(self, event);
    }

    pub fn step(&self) {
        self.room.borrow().step(self);
    }

    pub fn draw(&self) {
        self.room.borrow().draw(self);
    }

    pub fn dump(&self) {
        println!("{self:#?}");
    }

    pub fn cleanup(&self) {
        if let Some(next_room_index) = self.next_room_index.take() {
            self.goto_room(next_room_index);
        } else {
            self.room.borrow_mut().cleanup(self);
        }
    }

    pub fn destroy_instance(&self, id: ObjectId) {
        self.room.borrow().destroy_instance(id);
    }

    pub fn instance_number(&self, object_index: u32) -> i32 {
        self.object_types.get(&object_index).map_or(0, |o| {
            o.object
                .instances
                .borrow()
                .len()
                .try_into()
                .expect("invalid instance count")
        })
    }

    pub fn instance_create(&self, id: ObjectId, pos: IVec2, object_index: u32) -> Rc<Instance> {
        let obj = &self.content.objects[object_index];

        assert!(obj.mask_sprite_index < 0);
        let parent_object_index = obj.parent_object_index.try_into().ok();

        let alarm = Rc::<InstanceAlarm>::default();
        let alarm_id = self.new_instance(alarm.clone());

        let instance = Rc::new(Instance {
            id,
            state: RefCell::new(InstanceState {
                pos: pos.as_dvec2(),
                depth: obj.depth,
                velocity: default(),
                visible: obj.visible.into(),
                sprite_index: obj.sprite_index,
                sprite_asset: None,
                image_speed: 1.0,
                image_index: 0.0,
                image_blend_alpha: WHITE,
            }),
            object_index,
            parent_object_index,
            vars: default(),
            alarm_id,
            alarm,
        });

        self.room
            .borrow()
            .added_instances
            .borrow_mut()
            .insert(id.instance_id(), instance.clone());

        self.object_types[&object_index]
            .object
            .instances
            .borrow_mut()
            .insert(id, instance.clone());

        instance
    }
}

impl gml::eval::Global for Global {
    fn get(&self, name: &str) -> gml::eval::Result<Option<Value>> {
        if let Some(id) = self.scripts.names.get(name) {
            Ok(Some(Value::Int((*id).try_into().expect("invalid id"))))
        } else if let Some(value) = self.consts.get(name) {
            Ok(Some(value))
        } else {
            self.vars.member(name)
        }
    }

    fn set(&self, name: &str, value: Value) -> gml::eval::Result {
        if self.consts.get(name).is_some() {
            Err(gml::eval::Error::AssignToValue)
        } else {
            self.vars.set_member(name, value)
        }
    }

    fn instance(&self, id: ObjectId) -> Option<Rc<dyn Object>> {
        let room = self.room.borrow();
        if let Some(asset) = self.object_types.get(&id.instance_id()) {
            Some(asset.object.clone())
        } else if let Some(object) = room.added_instances.borrow().get(&id.instance_id()) {
            Some(object.clone())
        } else if let Some(object) = room.script_instances.borrow().get(&id) {
            Some(object.clone())
        } else if let Some(object) = room.object_instances.borrow().values.get(&id.instance_id()) {
            Some(object.clone())
        } else {
            println!("missing instance id: {id:?}");
            println!("added instance ids this event:");
            for id in room.added_instances.borrow().keys() {
                println!("  {id}");
            }
            println!("existing instance ids before this event:");
            for id in room.object_instances.borrow().values.keys() {
                println!("  {id}");
            }
            println!("script object ids:");
            for id in room.script_instances.borrow().keys() {
                println!("  {id:?}");
            }
            None
        }
    }

    fn new_instance(&self, object: Rc<dyn Object>) -> ObjectId {
        let id = self.next_instance_id();
        let id = ObjectId::new(id);
        self.room
            .borrow()
            .script_instances
            .borrow_mut()
            .insert(id, object);
        id
    }

    fn call(
        &self,
        context: &mut Context<'_>,
        id: &str,
        args: Vec<Value>,
    ) -> gml::eval::Result<Value> {
        if let Some(id) = self.scripts.names.get(id) {
            context.exec_script(&self.scripts.values[id], &args)
        } else {
            crate::scripts::call(self, context, id, args)
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub struct GlobalState {
    #[serde(serialize_with = "serialize_color")]
    pub color: Color,
    #[serde(skip)]
    pub fonts: fonts::FontMap,
}

fn define_scripts(content: &gmk_file::Content) -> DoubleMap<gml::ast::Script> {
    // scripts parsed in parallel
    use rayon::prelude::*;
    let scripts = content
        .scripts
        .items
        .par_iter()
        .enumerate()
        .flat_map(|(index, item)| {
            item.as_ref().map(|item| {
                let script = gml::parse(&item.name.0, &item.data.script.0).unwrap();
                (index as u32, item.name.0.clone(), script)
            })
        })
        .collect::<Vec<_>>();

    let mut result = DoubleMap::default();

    for (index, name, script) in scripts {
        result.names.insert(name, index);
        result.values.insert(index, script);
    }

    result
}

fn define_consts(content: &gmk_file::Content) -> gml::eval::Namespace {
    let mut vars = gml::eval::Namespace::default();
    use gmk_file::Key;

    // vk
    fn vk(key: Key) -> Value {
        i32::from(key).into()
    }

    vars.insert("vk_nokey", vk(Key::NoKey));
    vars.insert("vk_anykey", vk(Key::AnyKey));

    vars.insert("vk_backspace", vk(Key::Backspace));
    vars.insert("vk_tab", vk(Key::Tab));

    vars.insert("vk_enter", vk(Key::Enter));

    vars.insert("vk_shift", vk(Key::Shift));
    vars.insert("vk_control", vk(Key::Control));
    vars.insert("vk_alt", vk(Key::Alt));

    vars.insert("vk_escape", vk(Key::Escape));

    vars.insert("vk_space", vk(Key::Space));
    vars.insert("vk_pageup", vk(Key::PageUp));
    vars.insert("vk_pagedown", vk(Key::PageDown));
    vars.insert("vk_end", vk(Key::End));
    vars.insert("vk_home", vk(Key::Home));
    vars.insert("vk_left", vk(Key::Left));
    vars.insert("vk_up", vk(Key::Up));
    vars.insert("vk_right", vk(Key::Right));
    vars.insert("vk_down", vk(Key::Down));

    vars.insert("vk_insert", vk(Key::Insert));
    vars.insert("vk_delete", vk(Key::Delete));

    vars.insert("vk_numpad0", vk(Key::Numpad0));
    vars.insert("vk_numpad1", vk(Key::Numpad1));
    vars.insert("vk_numpad2", vk(Key::Numpad2));
    vars.insert("vk_numpad3", vk(Key::Numpad3));
    vars.insert("vk_numpad4", vk(Key::Numpad4));
    vars.insert("vk_numpad5", vk(Key::Numpad5));
    vars.insert("vk_numpad6", vk(Key::Numpad6));
    vars.insert("vk_numpad7", vk(Key::Numpad7));
    vars.insert("vk_numpad8", vk(Key::Numpad8));
    vars.insert("vk_numpad9", vk(Key::Numpad9));
    vars.insert("vk_multiply", vk(Key::Multiply));
    vars.insert("vk_add", vk(Key::Add));
    vars.insert("vk_subtract", vk(Key::Subtract));
    vars.insert("vk_decimal", vk(Key::Decimal));
    vars.insert("vk_divide", vk(Key::Divide));

    // colors
    vars.insert("c_aqua", 16776960);
    vars.insert("c_black", 0);
    vars.insert("c_blue", 16711680);
    vars.insert("c_dkgray", 4210752);
    vars.insert("c_fuchsia", 16711935);
    vars.insert("c_gray", 8421504);
    vars.insert("c_green", 32768);
    vars.insert("c_lime", 65280);
    vars.insert("c_ltgray", 12632256);
    vars.insert("c_maroon", 128);
    vars.insert("c_navy", 8388608);
    vars.insert("c_olive", 32896);
    vars.insert("c_orange", 4235519);
    vars.insert("c_purple", 8388736);
    vars.insert("c_red", 255);
    vars.insert("c_silver", 12632256);
    vars.insert("c_teal", 8421376);
    vars.insert("c_white", 16777215);
    vars.insert("c_yellow", 65535);

    resources(&mut vars, &content.objects);
    resources(&mut vars, &content.rooms);
    resources(&mut vars, &content.scripts);
    resources(&mut vars, &content.backgrounds);
    resources(&mut vars, &content.sprites);
    resources(&mut vars, &content.sounds);

    return vars;

    fn resources<T>(vars: &mut gml::eval::Namespace, chunk: &gmk_file::ResourceChunk<T>) {
        for (index, name, _) in chunk {
            let id = ObjectId::new(index);
            vars.insert(name, id);
        }
    }
}

fn define_objects(content: &gmk_file::Content) -> HashMap<u32, ObjectAsset> {
    let mut result = HashMap::new();

    for (object_index, name, def) in &content.objects {
        let mut object = ObjectAsset {
            name: name.to_string(),
            parent_index: def.parent_object_index.try_into().ok(),
            ..Default::default()
        };

        for (event_id, event) in &def.events {
            object.events.insert(
                {
                    use gmk_file::{DrawEventId, EventId, StepEventId};
                    match event_id {
                        EventId::Create => Event::Create,
                        EventId::Destroy => Event::Destroy,
                        EventId::Step(StepEventId::Begin) => Event::StepBegin,
                        EventId::Step(StepEventId::Normal) => Event::StepNormal,
                        EventId::Step(StepEventId::End) => Event::StepEnd,
                        EventId::Draw(DrawEventId::Normal) => Event::Draw,
                        EventId::Alarm(index) => Event::Alarm(*index),
                        EventId::KeyPress(key) => Event::KeyPress(key_code(*key)),
                        EventId::KeyRelease(key) => Event::KeyRelease(key_code(*key)),
                        EventId::Keyboard(key) => Event::KeyDown(key_code(*key)),
                        EventId::Collision(object_index) => Event::Collision(*object_index),
                        _ => unimplemented!("EventId: {event_id:?}"),
                    }
                },
                event
                    .actions
                    .iter()
                    .flat_map(|action| {
                        Some(match action.kind {
                            gmk_file::ActionKind::Code => Action::ScriptInline(
                                gml::parse(
                                    &format!("{name}/{event_id:?}"),
                                    action.argument_values[0].0.as_str(),
                                )
                                .unwrap(),
                            ),
                            gmk_file::ActionKind::Normal => {
                                if action.exec == gmk_file::ActionExec::None {
                                    return None;
                                }
                                assert_eq!(action.exec, gmk_file::ActionExec::Function);
                                match action.function_name.0.as_str() {
                                    "action_bounce" => Action::Bounce,
                                    "action_move" => Action::Move(
                                        action.argument_values[0].parse().unwrap(),
                                        action.argument_values[1].parse().unwrap(),
                                    ),
                                    "action_execute_script" => Action::ScriptRes(
                                        action.argument_values[0].parse().unwrap(),
                                    ),
                                    "action_kill_object" => Action::KillObject,
                                    "action_set_alarm" => Action::SetAlarm(
                                        action.argument_values[0].parse().unwrap(),
                                        action.argument_values[1].parse().unwrap(),
                                    ),
                                    name => unimplemented!("action function_name: {name}"),
                                }
                            }
                            gmk_file::ActionKind::Variable => {
                                assert!(!bool::from(action.relative));
                                let name = action.argument_values[0].0.clone();
                                let value =
                                    gml::parse_expr(action.argument_values[0].0.as_str()).unwrap();
                                Action::SetVariable(name, value)
                            }
                            _ => unimplemented!("action kind: {:?}", action.kind),
                        })
                    })
                    .collect(),
            );
        }

        result.insert(object_index, object);
    }

    result
}
