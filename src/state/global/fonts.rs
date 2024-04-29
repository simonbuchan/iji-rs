use glam::IVec2;
use macroquad::color::WHITE;
use macroquad::prelude::draw_texture;
use std::collections::HashMap;

use super::Global;
use crate::assets::{AssetId, SpriteAsset};

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
