#![allow(dead_code)]

use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic;
use std::sync::atomic::AtomicU32;

use macroquad::prelude::*;
use serde::ser::{SerializeMap, SerializeStruct};
use serde::{Serialize, Serializer};

use gml::eval::{Global as _, Object, ObjectId, Value};
use gml::Context;

use crate::assets::*;

fn color_u32(value: u32) -> Color {
    let [a, r, g, b] = value.to_be_bytes();
    Color::from_rgba(r, g, b, a)
}

fn default<T: Default>() -> T {
    Default::default()
}

#[derive(Debug, Serialize)]
pub struct DoubleMap<V> {
    pub names: HashMap<String, u32>,
    pub values: HashMap<u32, V>,
}

impl<V> Default for DoubleMap<V> {
    fn default() -> Self {
        Self {
            names: default(),
            values: default(),
        }
    }
}

impl<V> std::ops::Index<u32> for DoubleMap<V> {
    type Output = V;

    fn index(&self, index: u32) -> &Self::Output {
        &self.values[&index]
    }
}

impl<V> std::ops::Index<&str> for DoubleMap<V> {
    type Output = V;

    fn index(&self, name: &str) -> &Self::Output {
        &self.values[&self.names[name]]
    }
}

#[derive(Default)]
struct ObjectType {
    instances: RefCell<HashMap<ObjectId, Rc<Instance>>>,
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

#[derive(Default, Debug, Serialize)]
pub struct GlobalState {
    #[serde(serialize_with = "serialize_color")]
    pub color: Color,
    #[serde(skip)]
    pub fonts: FontMap,
}

#[derive(Default, Debug)]
pub struct FontMap {
    last_index: i32,
    items: HashMap<i32, FontAsset>,
    index: i32,
}

impl FontMap {
    pub fn add(&mut self, item: FontAsset) -> i32 {
        self.last_index += 1;
        self.items.insert(self.last_index, item);
        self.last_index
    }

    pub fn get(&self) -> Option<&FontAsset> {
        self.items.get(&self.index)
    }

    pub fn set(&mut self, index: i32) {
        self.index = index;
    }
}

#[derive(Debug)]
pub struct FontAsset {
    sprite: AssetId<SpriteAsset>,
    first: u32,
}

impl FontAsset {
    pub fn new(sprite: AssetId<SpriteAsset>, first: u32) -> Self {
        Self { sprite, first }
    }

    pub fn draw_text(&self, global: &Global, pos: IVec2, string: &str, sep: i32, w: i32) {
        let assets = global.assets.borrow();
        let sprite = assets.sprites.get(self.sprite);

        let wrap_chars = w as usize / sprite.size.x as usize;

        let chars = string
            .chars()
            .flat_map(|c| {
                let is_space = c == ' ';
                let codepoint = u32::try_from(c).ok()?;
                let index = codepoint.checked_sub(self.first)?;
                let index = usize::try_from(index).ok()?;
                Some((is_space, index))
            })
            .collect::<Vec<_>>();

        let mut lines = vec![];
        let mut line_index = 0;
        loop {
            line_index += wrap_chars;
            if line_index >= chars.len() {
                lines.push(chars.len());
                break;
            }
            // is_space
            while !chars[line_index - 1].0 {
                line_index -= 1;
            }
            lines.push(line_index);
        }

        let mut y = pos.y;
        let mut start_index = 0;
        for end_index in lines {
            let mut x = pos.x;

            for (_, index) in &chars[start_index..end_index] {
                let Some(texture) = sprite.textures.get(*index) else {
                    continue;
                };

                draw_texture(*texture, x as f32, y as f32, WHITE);
                x += sprite.size.x as i32;
            }

            start_index = end_index;

            y += sep;
        }
    }
}

#[derive(Serialize)]
pub struct Global {
    #[serde(skip)]
    content: gmk_file::Content,
    assets: RefCell<Assets>,
    object_types: HashMap<u32, ObjectAsset>,
    consts: gml::eval::Namespace,
    vars: gml::eval::Namespace,
    #[serde(skip)]
    scripts: DoubleMap<gml::ast::Script>,
    room_order_index: RefCell<usize>,
    room: RefCell<Room>,
    next_room_index: RefCell<Option<u32>>,
    pub state: RefCell<GlobalState>,
    last_instance_id: AtomicU32,
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

#[derive(Serialize)]
pub struct Room {
    view: View,
    #[serde(serialize_with = "serialize_color")]
    background_color: Color,
    background_layers: Vec<Layer>,
    tiles: Vec<Tile>,
    #[serde(serialize_with = "serialize_object_instances")]
    object_instances: RefCell<DoubleMap<Rc<Instance>>>,
    foreground_layers: Vec<Layer>,
    speed: f32,
    elapsed: RefCell<f32>,

    #[serde(serialize_with = "serialize_script_instances")]
    script_instances: RefCell<HashMap<ObjectId, Rc<dyn Object>>>,

    #[serde(skip)]
    added_instances: RefCell<HashMap<u32, Rc<Instance>>>,
    #[serde(skip)]
    destroyed_instances: RefCell<Vec<ObjectId>>,
}

fn serialize_object_instances<S: Serializer>(
    this: &RefCell<DoubleMap<Rc<Instance>>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    struct HashMapRcSerialize<'a>(&'a HashMap<u32, Rc<Instance>>);
    impl Serialize for HashMapRcSerialize<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut map = serializer.serialize_map(Some(self.0.len()))?;
            for (key, value) in self.0.iter() {
                map.serialize_entry(&key, &**value)?
            }
            map.end()
        }
    }

    let this = this.borrow();
    let mut map = serializer.serialize_map(Some(2))?;
    map.serialize_entry("names", &this.names)?;
    map.serialize_entry("values", &HashMapRcSerialize(&this.values))?;
    map.end()
}

fn serialize_script_instances<S: Serializer>(
    this: &RefCell<HashMap<ObjectId, Rc<dyn Object>>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let this = this.borrow();
    let mut map = serializer.serialize_map(Some(this.len()))?;
    for (key, value) in this.iter() {
        if let Some(names) = value.debug_member_names() {
            let members = names
                .into_iter()
                .flat_map(|name| value.member(&name).map(|value| (name, value)))
                .collect::<HashMap<_, _>>();

            map.serialize_entry(&key, &members)?;
        } else if let Some(length) = value.debug_index_length() {
            let values = (0..length)
                .map(|index| value.index(&[(index as i32).into()]).unwrap_or_default())
                .collect::<Vec<_>>();

            map.serialize_entry(&key, &values)?;
        } else {
            map.serialize_entry(&key, &())?;
        }
    }
    map.end()
}

impl std::fmt::Debug for Room {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Room")
            .field("view", &self.view)
            .field("background_color", &self.background_color)
            .field("background_layers", &self.background_layers)
            .field("tiles", &self.tiles)
            .field("object_instances", &self.object_instances)
            .field("speed", &self.speed)
            .field("elapsed", &self.elapsed)
            .field("script_instances", &self.script_instances.borrow().keys())
            .field("added_instances", &self.added_instances)
            .field("destroyed_instances", &self.destroyed_instances)
            .finish()
    }
}

impl Room {
    fn new() -> Self {
        Self {
            view: View {
                offset: default(),
                size: vec2(screen_width(), screen_height()).as_uvec2(),
            },
            background_color: default(),
            background_layers: vec![],
            tiles: vec![],
            object_instances: default(),
            foreground_layers: vec![],

            speed: 30.0,
            elapsed: RefCell::new(0.0),

            script_instances: default(),
            added_instances: default(),
            destroyed_instances: default(),
        }
    }

    fn load(&mut self, global: &Global, def: &gmk_file::Room) {
        self.background_color = color_u32(def.background_color);

        for b in &def.backgrounds {
            let Ok(index) = b.background_image_index.try_into() else {
                continue;
            };

            let asset = global.loader().get_background(index);
            let assets = global.assets.borrow();
            let bg = assets.backgrounds.get(asset);

            if b.foreground_image.into() {
                &mut self.foreground_layers
            } else {
                &mut self.background_layers
            }
            .push(Layer {
                enabled: b.visible.into(),
                pos: ivec2(b.pos.0, b.pos.1),
                asset,
                tile: b.tile.0 != 0,
                source: bg.tile_enabled.then_some({
                    let pos = bg.tile_pos.as_vec2();
                    let size = bg.tile_size.as_vec2();
                    Rect::new(pos.x, pos.y, size.x, size.y)
                }),
            });
        }

        for t in &def.tiles {
            self.tiles.push(Tile {
                depth: t.depth,
                asset: global.loader().get_background(t.background_index),
                pos: ivec2(t.pos.0, t.pos.1),
                source: Rect {
                    x: t.tile.0 as f32,
                    y: t.tile.1 as f32,
                    w: t.size.0 as f32,
                    h: t.size.1 as f32,
                },
            });
        }

        self.speed = def.speed as f32;
    }

    pub fn step(&self, global: &Global) {
        let mut elapsed = self.elapsed.borrow_mut();
        *elapsed += get_frame_time() * self.speed;
        while *elapsed >= 1.0 {
            *elapsed -= 1.0;
            self.dispatch(global, Event::StepBegin);
            for instance in self.object_instances.borrow().values.values() {
                instance.clone().step(global);
            }
            self.dispatch(global, Event::StepNormal);
            self.dispatch(global, Event::StepEnd);
        }
    }

    pub fn draw(&self, global: &Global) {
        clear_background(self.background_color);
        for layer in &self.background_layers {
            layer.draw(global, &self.view);
        }

        let object_instances = self.object_instances.borrow();
        enum DrawItem<'a> {
            Tile(&'a Tile),
            Instance(Rc<Instance>),
        }
        let mut depth_draws = Vec::new();
        depth_draws.extend(self.tiles.iter().map(DrawItem::Tile));
        depth_draws.extend(
            object_instances
                .values
                .values()
                .filter(|item| {
                    let state = item.state.borrow();
                    state.visible && (-16000..=16000).contains(&state.depth)
                })
                .map(|item| DrawItem::Instance(item.clone())),
        );
        depth_draws.sort_by_key(|item| match item {
            DrawItem::Tile(tile) => -tile.depth,
            DrawItem::Instance(instance) => -instance.state.borrow().depth,
        });

        for draw in depth_draws {
            match draw {
                DrawItem::Tile(tile) => tile.draw(global, &self.view),
                DrawItem::Instance(instance) => {
                    instance.draw(global, &self.view);
                    instance.dispatch(global, Event::Draw);
                }
            }
        }
        drop(object_instances);

        for layer in &self.foreground_layers {
            layer.draw(global, &self.view);
        }
    }

    pub fn destroy_instance(&self, id: ObjectId) {
        self.destroyed_instances.borrow_mut().push(id);
    }

    pub fn dispatch(&self, global: &Global, event: Event) {
        for instance in self.object_instances.borrow().values.values() {
            instance.clone().dispatch(global, event);
        }
        self.cleanup(global);
    }

    pub fn cleanup(&self, global: &Global) {
        for id in self.destroyed_instances.borrow_mut().drain(..) {
            if let Some(instance) = self
                .object_instances
                .borrow_mut()
                .values
                .remove(&id.instance_id())
            {
                instance.clone().dispatch(global, Event::Destroy);

                global.object_types[&instance.object_index]
                    .object
                    .instances
                    .borrow_mut()
                    .remove(&id);
            }
        }

        self.object_instances
            .borrow_mut()
            .values
            .extend(self.added_instances.borrow_mut().drain());
    }
}

trait Draw {
    fn draw(&self, assets: &Global, view: &View);
}

#[derive(Debug, Serialize)]
struct View {
    offset: IVec2,
    size: UVec2,
}

#[derive(Debug, Serialize)]
struct Layer {
    enabled: bool,
    asset: AssetId<BackgroundAsset>,
    pos: IVec2,
    #[serde(skip)]
    source: Option<Rect>,
    tile: bool,
}

impl Draw for Layer {
    fn draw(&self, global: &Global, view: &View) {
        if !self.enabled {
            return;
        }
        let assets = global.assets.borrow();
        let bg = assets.backgrounds.get(self.asset);

        let pos = self.pos - view.offset;
        if !self.tile {
            let pos = pos.as_vec2();
            draw_texture_ex(
                bg.texture,
                pos.x,
                pos.y,
                WHITE,
                DrawTextureParams {
                    source: self.source,
                    ..default()
                },
            )
        } else {
            let tiles = (view.size + bg.size - uvec2(1, 1)) % bg.size;
            for ix in 0..tiles.x {
                for iy in 0..tiles.y {
                    let pos = (pos + (view.size * uvec2(ix, iy)).as_ivec2()).as_vec2();
                    draw_texture_ex(
                        bg.texture,
                        pos.x,
                        pos.y,
                        WHITE,
                        DrawTextureParams {
                            source: self.source,
                            ..default()
                        },
                    );
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct Tile {
    depth: i32,
    asset: AssetId<BackgroundAsset>,
    pos: IVec2,
    #[serde(serialize_with = "serialize_rect")]
    source: Rect,
}

fn serialize_rect<S>(value: &Rect, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut s = serializer.serialize_struct("Rect", 2)?;
    s.serialize_field("point", &value.point())?;
    s.serialize_field("size", &value.size())?;
    s.end()
}

impl Draw for Tile {
    fn draw(&self, global: &Global, view: &View) {
        let pos = (self.pos - view.offset).as_vec2();
        draw_texture_ex(
            global.assets.borrow().backgrounds.get(self.asset).texture,
            pos.x,
            pos.y,
            WHITE,
            DrawTextureParams {
                source: Some(self.source),
                ..Default::default()
            },
        );
    }
}

#[derive(Debug, Serialize)]
struct InstanceState {
    pos: DVec2,
    depth: i32,
    velocity: InstanceVelocity,
    visible: bool,
    sprite_index: i32,
    sprite_asset: Option<AssetId<SpriteAsset>>,
    image_speed: f64,
    image_index: f64,
    #[serde(serialize_with = "serialize_color")]
    image_blend_alpha: Color,
}

fn serialize_color<S>(value: &Color, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut s = serializer.serialize_struct("Color", 4)?;
    s.serialize_field("r", &value.r)?;
    s.serialize_field("g", &value.g)?;
    s.serialize_field("b", &value.b)?;
    s.serialize_field("a", &value.a)?;
    s.end()
}

#[derive(Debug, Serialize)]
enum InstanceVelocity {
    Cartesian(DVec2),
    Polar(Polar),
}

impl Default for InstanceVelocity {
    fn default() -> Self {
        Self::Cartesian(default())
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
struct Polar {
    pub length: f64,
    pub direction: f64,
}

impl From<DVec2> for Polar {
    fn from(value: DVec2) -> Self {
        // f64 version of macroquad cartesian_to_polar()
        let length = (value.x.powi(2) + value.y.powi(2)).sqrt();
        let direction = value.y.atan2(value.x).to_degrees();
        Self { length, direction }
    }
}

impl From<Polar> for DVec2 {
    fn from(value: Polar) -> Self {
        let (y, x) = value.direction.to_radians().sin_cos();
        dvec2(x, y) * value.length
    }
}

impl InstanceVelocity {
    fn cartesian(&self) -> DVec2 {
        match self {
            Self::Cartesian(result) => *result,
            Self::Polar(polar) => (*polar).into(),
        }
    }

    fn cartesian_mut(&mut self) -> &mut DVec2 {
        match self {
            Self::Cartesian(result) => result,
            Self::Polar(polar) => {
                *self = Self::Cartesian((*polar).into());
                let Self::Cartesian(result) = self else {
                    unreachable!()
                };
                result
            }
        }
    }

    fn polar(&self) -> Polar {
        match self {
            Self::Cartesian(cartesian) => (*cartesian).into(),
            Self::Polar(polar) => *polar,
        }
    }

    fn polar_mut(&mut self) -> &mut Polar {
        match self {
            Self::Polar(result) => result,
            Self::Cartesian(cartesian) => {
                *self = Self::Polar((*cartesian).into());
                let Self::Polar(result) = self else {
                    unreachable!()
                };
                result
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Instance {
    id: ObjectId,
    state: RefCell<InstanceState>,
    object_index: u32,
    parent_object_index: Option<u32>,
    vars: gml::eval::Namespace,
    alarm_id: ObjectId,
    #[serde(skip)]
    alarm: Rc<InstanceAlarm>,
}

impl Instance {
    pub fn step(self: Rc<Self>, global: &Global) {
        {
            let mut state = self.state.borrow_mut();
            let mut state = &mut *state; // I have no idea.
            state.image_index += state.image_speed;
            state.pos += state.velocity.cartesian();
        }

        let mut alarm_ids = vec![];
        self.alarm.active.borrow_mut().retain(|&alarm_id, steps| {
            *steps -= 1;
            if *steps > 0 {
                true
            } else {
                alarm_ids.push(alarm_id);
                false
            }
        });

        for alarm_id in alarm_ids {
            self.clone().dispatch(global, Event::Alarm(alarm_id));
        }
    }

    pub fn dispatch(self: Rc<Self>, global: &Global, event: Event) {
        let mut ctx = Context::new(global, self.id, self.clone());

        // implicit inheritance
        let mut object_index = self.object_index;
        let actions = loop {
            let obj = &global.object_types[&object_index];

            if let Some(a) = obj.events.get(&event) {
                break a;
            };
            if let Some(i) = obj.parent_index {
                object_index = i;
            } else {
                return;
            }
        };

        for action in actions {
            match action {
                Action::ScriptInline(script) => {
                    if let Err(error) = ctx.exec_script(script, &[]) {
                        eprintln!("{error}");
                    }
                }
                Action::ScriptRes(index) => {
                    let script = &global.scripts.values[index];
                    if let Err(error) = ctx.exec_script(script, &[]) {
                        eprintln!("{error}");
                    }
                }
                Action::Bounce => {
                    unimplemented!("Action: Bounce");
                }
                Action::Move(dir, amount) => {
                    unimplemented!("Action: Move({dir}, {amount})");
                }
                Action::KillObject => {
                    global.destroy_instance(ctx.instance_id);
                }
                Action::SetAlarm(index, steps) => {
                    self.alarm.set(*index, *steps);
                }
                Action::SetVariable(name, expr) => {
                    let value = ctx.eval(expr).unwrap();
                    self.set_member(name, value).unwrap();
                }
            }
        }
    }
}

impl Draw for Instance {
    fn draw(&self, global: &Global, view: &View) {
        let mut state = self.state.borrow_mut();
        if let Some(&mut sprite_asset) = state.sprite_index.try_into().ok().map(|index| {
            state
                .sprite_asset
                .get_or_insert_with(|| global.loader().get_sprite(index))
        }) {
            let assets = global.assets.borrow();
            let sprite = assets.sprites.get(sprite_asset);

            let sprite_frame = state.image_index % sprite.textures.len() as f64;
            state.image_index = sprite_frame;

            let texture = sprite.textures[sprite_frame.floor() as usize];
            let pos = state.pos.as_vec2() - sprite.origin.as_vec2() - view.offset.as_vec2();
            draw_texture(texture, pos.x, pos.y, state.image_blend_alpha);
        }
    }
}

impl Object for Instance {
    fn member(&self, name: &str) -> gml::eval::Result<Option<Value>> {
        // dbg!(name);
        let state = self.state.borrow();
        Ok(Some(match name {
            "visible" => state.visible.into(),
            "depth" => state.depth.into(),
            "x" => state.pos.x.into(),
            "y" => state.pos.y.into(),
            "alarm" => self.alarm_id.into(),
            "sprite_index" => state.sprite_index.into(),
            "image_speed" => state.image_speed.into(),
            "image_index" => state.image_index.into(),
            "image_single" => if state.image_speed > 0.0 {
                state.image_index
            } else {
                -1.0
            }
            .into(),
            "image_alpha" => (state.image_blend_alpha.a as f64).into(),
            _ => return self.vars.member(name),
        }))
    }

    fn set_member(&self, name: &str, value: Value) -> gml::eval::Result {
        // dbg!(name);
        let mut state = self.state.borrow_mut();
        match name {
            "visible" => state.visible = value.to_bool(),
            "depth" => state.depth = value.to_int(),
            "x" => state.pos.x = value.to_float(),
            "y" => state.pos.y = value.to_float(),
            "speed" => {
                state.velocity.polar_mut().length = value.to_float();
            }
            "direction" => {
                state.velocity.polar_mut().direction = value.to_float();
            }
            "hspeed" => {
                state.velocity.cartesian_mut().x = value.to_float();
            }
            "vspeed" => {
                state.velocity.cartesian_mut().y = value.to_float();
            }
            "alarm" => return Err(gml::eval::Error::AssignToValue),
            "sprite_index" => {
                state.sprite_index = value.to_int();
                state.image_index = 0.0;
                state.sprite_asset = None;
            }
            "image_speed" => state.image_speed = value.to_float(),
            "image_index" => state.image_index = value.to_float(),
            "image_single" => {
                let value = value.to_float();
                if value < 0.0 {
                    state.image_speed = 1.0;
                } else {
                    state.image_speed = 0.0;
                    state.image_index = value;
                }
            }
            "image_blend" => {
                let color = value.to_int();
                let [r, g, b, _] = color.to_le_bytes();
                state.image_blend_alpha.r = r as f32 / 255.0;
                state.image_blend_alpha.g = g as f32 / 255.0;
                state.image_blend_alpha.b = b as f32 / 255.0;
            }
            "image_alpha" => state.image_blend_alpha.a = value.to_float() as f32,
            _ => self.vars.set_member(name, value)?,
        }
        Ok(())
    }
}

#[derive(Default, Debug, Serialize)]
struct InstanceAlarm {
    active: RefCell<HashMap<i32, i32>>,
}

impl InstanceAlarm {
    fn set(&self, index: i32, steps: i32) {
        if steps <= 0 {
            self.active.borrow_mut().remove(&index);
        } else {
            self.active.borrow_mut().insert(index, steps);
        }
    }
}

impl Object for InstanceAlarm {
    fn set_index(&self, args: &[Value], value: Value) -> gml::eval::Result {
        let index = args[0].to_int();
        let steps = value.to_int();
        self.set(index, steps);
        Ok(())
    }
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

#[derive(Default)]
struct ObjectAsset {
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
enum Action {
    ScriptInline(gml::Script),
    ScriptRes(u32),
    Bounce,
    SetAlarm(i32, i32),
    KillObject,
    Move(u32, f32),
    SetVariable(String, Box<gml::ast::Expr>),
}

pub const KEY_CODES: &[KeyCode] = &[
    KeyCode::Backspace,
    KeyCode::Tab,
    KeyCode::Enter,
    KeyCode::LeftShift,
    KeyCode::LeftControl,
    KeyCode::LeftAlt,
    KeyCode::Escape,
    KeyCode::Space,
    KeyCode::PageUp,
    KeyCode::PageDown,
    KeyCode::End,
    KeyCode::Home,
    KeyCode::Left,
    KeyCode::Up,
    KeyCode::Right,
    KeyCode::Down,
    KeyCode::Insert,
    KeyCode::Delete,
    KeyCode::Key0,
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::Key4,
    KeyCode::Key5,
    KeyCode::Key6,
    KeyCode::Key7,
    KeyCode::Key8,
    KeyCode::Key9,
    KeyCode::A,
    KeyCode::B,
    KeyCode::C,
    KeyCode::D,
    KeyCode::E,
    KeyCode::F,
    KeyCode::G,
    KeyCode::H,
    KeyCode::I,
    KeyCode::J,
    KeyCode::K,
    KeyCode::L,
    KeyCode::M,
    KeyCode::N,
    KeyCode::O,
    KeyCode::P,
    KeyCode::Q,
    KeyCode::R,
    KeyCode::S,
    KeyCode::T,
    KeyCode::U,
    KeyCode::V,
    KeyCode::W,
    KeyCode::X,
    KeyCode::Y,
    KeyCode::Z,
    KeyCode::Kp0,
    KeyCode::Kp1,
    KeyCode::Kp2,
    KeyCode::Kp3,
    KeyCode::Kp4,
    KeyCode::Kp5,
    KeyCode::Kp6,
    KeyCode::Kp7,
    KeyCode::Kp8,
    KeyCode::Kp9,
    KeyCode::KpMultiply,
    KeyCode::KpAdd,
    KeyCode::KpSubtract,
    KeyCode::KpDecimal,
    KeyCode::KpDivide,
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F8,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
    KeyCode::F12,
];

pub fn key_code(vk: gmk_file::Key) -> KeyCode {
    use gmk_file::Key;

    // Maybe there's a nice way to macro this up?
    match vk {
        Key::Backspace => KeyCode::Backspace,
        Key::Tab => KeyCode::Tab,

        Key::Enter => KeyCode::Enter,

        Key::Shift => KeyCode::LeftShift,
        Key::Control => KeyCode::LeftControl,
        Key::Alt => KeyCode::LeftAlt,

        Key::Escape => KeyCode::Escape,

        Key::Space => KeyCode::Space,
        Key::PageUp => KeyCode::PageUp,
        Key::PageDown => KeyCode::PageDown,
        Key::End => KeyCode::End,
        Key::Home => KeyCode::Home,
        Key::Left => KeyCode::Left,
        Key::Up => KeyCode::Up,
        Key::Right => KeyCode::Right,
        Key::Down => KeyCode::Down,

        Key::Insert => KeyCode::Insert,
        Key::Delete => KeyCode::Delete,

        Key::Key0 => KeyCode::Key0,
        Key::Key1 => KeyCode::Key1,
        Key::Key2 => KeyCode::Key2,
        Key::Key3 => KeyCode::Key3,
        Key::Key4 => KeyCode::Key4,
        Key::Key5 => KeyCode::Key5,
        Key::Key6 => KeyCode::Key6,
        Key::Key7 => KeyCode::Key7,
        Key::Key8 => KeyCode::Key8,
        Key::Key9 => KeyCode::Key9,

        Key::A => KeyCode::A,
        Key::B => KeyCode::B,
        Key::C => KeyCode::C,
        Key::D => KeyCode::D,
        Key::E => KeyCode::E,
        Key::F => KeyCode::F,
        Key::G => KeyCode::G,
        Key::H => KeyCode::H,
        Key::I => KeyCode::I,
        Key::J => KeyCode::J,
        Key::K => KeyCode::K,
        Key::L => KeyCode::L,
        Key::M => KeyCode::M,
        Key::N => KeyCode::N,
        Key::O => KeyCode::O,
        Key::P => KeyCode::P,
        Key::Q => KeyCode::Q,
        Key::R => KeyCode::R,
        Key::S => KeyCode::S,
        Key::T => KeyCode::T,
        Key::U => KeyCode::U,
        Key::V => KeyCode::V,
        Key::W => KeyCode::W,
        Key::X => KeyCode::X,
        Key::Y => KeyCode::Y,
        Key::Z => KeyCode::Z,

        Key::Numpad0 => KeyCode::Kp0,
        Key::Numpad1 => KeyCode::Kp1,
        Key::Numpad2 => KeyCode::Kp2,
        Key::Numpad3 => KeyCode::Kp3,
        Key::Numpad4 => KeyCode::Kp4,
        Key::Numpad5 => KeyCode::Kp5,
        Key::Numpad6 => KeyCode::Kp6,
        Key::Numpad7 => KeyCode::Kp7,
        Key::Numpad8 => KeyCode::Kp8,
        Key::Numpad9 => KeyCode::Kp9,
        Key::Multiply => KeyCode::KpMultiply,
        Key::Add => KeyCode::KpAdd,
        Key::Subtract => KeyCode::KpSubtract,
        Key::Decimal => KeyCode::KpDecimal,
        Key::Divide => KeyCode::KpDivide,
        Key::F1 => KeyCode::F1,
        Key::F2 => KeyCode::F2,
        Key::F3 => KeyCode::F3,
        Key::F4 => KeyCode::F4,
        Key::F5 => KeyCode::F5,
        Key::F6 => KeyCode::F6,
        Key::F7 => KeyCode::F7,
        Key::F8 => KeyCode::F8,
        Key::F9 => KeyCode::F9,
        Key::F10 => KeyCode::F10,
        Key::F11 => KeyCode::F11,
        Key::F12 => KeyCode::F12,

        _ => KeyCode::Unknown,
    }
}
