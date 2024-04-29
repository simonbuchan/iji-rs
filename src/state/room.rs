use glam::{ivec2, vec2};
use gml::eval::{Object, ObjectId};
use macroquad::color::Color;
use macroquad::math::Rect;
use macroquad::prelude::{clear_background, get_frame_time, screen_height, screen_width};
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::{
    color_u32, default, serialize_color, DoubleMap, Draw, Event, Global, Instance, Layer, Tile,
    View,
};

#[derive(Serialize)]
pub struct Room {
    pub view: View,
    #[serde(serialize_with = "serialize_color")]
    pub background_color: Color,
    pub background_layers: Vec<Layer>,
    pub tiles: Vec<Tile>,
    #[serde(serialize_with = "serialize_object_instances")]
    pub object_instances: RefCell<DoubleMap<Rc<Instance>>>,
    pub foreground_layers: Vec<Layer>,
    pub speed: f32,
    pub elapsed: RefCell<f32>,

    #[serde(serialize_with = "serialize_script_instances")]
    pub script_instances: RefCell<HashMap<ObjectId, Rc<dyn Object>>>,

    #[serde(skip)]
    pub added_instances: RefCell<HashMap<u32, Rc<Instance>>>,
    #[serde(skip)]
    pub destroyed_instances: RefCell<Vec<ObjectId>>,
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
    pub fn new() -> Self {
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

    pub fn load(&mut self, global: &Global, def: &gmk_file::Room) {
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

                let object_type = &global.object_types[&instance.object_index];
                object_type.object.instances.borrow_mut().remove(&id);
            } else {
                println!("cleanup instance not found: {:?}", id);
            }
        }

        self.object_instances
            .borrow_mut()
            .values
            .extend(self.added_instances.borrow_mut().drain());
    }
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
