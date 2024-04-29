#![allow(dead_code)]

use std::collections::HashMap;

use macroquad::prelude::*;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use gml::eval::Global as _;

pub use self::global::{Action, Event, FontAsset, Global, ObjectAsset};
pub use self::instance::{Instance, InstanceAlarm, InstanceState};
pub use self::room::Room;
pub use crate::assets::*;

mod global;
mod instance;
mod room;

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

trait Draw {
    fn draw(&self, assets: &Global, view: &View);
}

#[derive(Debug, Serialize)]
pub struct View {
    pub offset: IVec2,
    pub size: UVec2,
}

#[derive(Debug, Serialize)]
pub struct Layer {
    pub enabled: bool,
    pub asset: AssetId<BackgroundAsset>,
    pub pos: IVec2,
    #[serde(skip)]
    pub source: Option<Rect>,
    pub tile: bool,
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
pub struct Tile {
    pub depth: i32,
    pub asset: AssetId<BackgroundAsset>,
    pub pos: IVec2,
    #[serde(serialize_with = "serialize_rect")]
    pub source: Rect,
}

pub fn serialize_rect<S>(value: &Rect, serializer: S) -> Result<S::Ok, S::Error>
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
