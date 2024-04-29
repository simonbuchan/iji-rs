use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::rc::Rc;

use glam::{dvec2, DVec2};
use gml::eval::{Object, ObjectId, Value};
use gml::Context;
use macroquad::color::Color;
use macroquad::prelude::draw_texture;
use serde::Serialize;

use super::{default, serialize_color, Action, Draw, Event, Global, View};
use crate::assets::{AssetId, SpriteAsset};

#[derive(Debug, Serialize)]
pub struct Instance {
    pub id: ObjectId,
    pub state: RefCell<InstanceState>,
    pub object_index: u32,
    pub parent_object_index: Option<u32>,
    pub vars: gml::eval::Namespace,
    pub alarm_id: ObjectId,
    #[serde(skip)]
    pub alarm: Rc<InstanceAlarm>,
}

impl Instance {
    pub fn step(self: Rc<Self>, global: &Global) {
        {
            let mut state = self.state.borrow_mut();
            let state = state.deref_mut();
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

#[derive(Debug, Serialize)]
pub struct InstanceState {
    pub pos: DVec2,
    pub depth: i32,
    pub velocity: InstanceVelocity,
    pub visible: bool,
    pub sprite_index: i32,
    pub sprite_asset: Option<AssetId<SpriteAsset>>,
    pub image_speed: f64,
    pub image_index: f64,
    #[serde(serialize_with = "serialize_color")]
    pub image_blend_alpha: Color,
}

#[derive(Default, Debug, Serialize)]
pub struct InstanceAlarm {
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

#[derive(Debug, Serialize)]
pub enum InstanceVelocity {
    Cartesian(DVec2),
    Polar(Polar),
}

impl Default for InstanceVelocity {
    fn default() -> Self {
        Self::Cartesian(default())
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
pub struct Polar {
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
