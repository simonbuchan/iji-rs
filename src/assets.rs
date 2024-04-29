use std::cell::RefCell;

use macroquad::prelude::*;
use serde::Serialize;

pub use background::BackgroundAsset;
pub use imp::*;
pub use sprite::SpriteAsset;

mod imp;

mod background;
mod sprite;

#[derive(Default, Serialize)]
pub struct Assets {
    pub backgrounds: AssetSet<BackgroundAsset>,
    pub sprites: AssetSet<SpriteAsset>,
}

pub struct Loader<'a> {
    content: &'a gmk_file::Content,
    assets: &'a RefCell<Assets>,
}

impl<'a> Loader<'a> {
    pub fn new(content: &'a gmk_file::Content, assets: &'a RefCell<Assets>) -> Self {
        Self { content, assets }
    }

    pub fn get_background(&mut self, index: u32) -> AssetId<background::BackgroundAsset> {
        self.assets
            .borrow_mut()
            .backgrounds
            .load(&self.content.backgrounds, index)
    }

    pub fn get_sprite(&mut self, index: u32) -> AssetId<sprite::SpriteAsset> {
        self.assets
            .borrow_mut()
            .sprites
            .load(&self.content.sprites, index)
    }
}

fn texture_from_data(data: &[u8], transparent: bool) -> Texture2D {
    let mut image = Image::from_file_with_format(data, None);

    // the bottom left pixel is the transparent pixel color
    if transparent {
        let t: [u8; 4] = image.get_pixel(0, u32::from(image.height) - 1).into();
        let data = image.get_image_data_mut();

        for p in data {
            if *p == t {
                *p = [0, 0, 0, 0];
            }
        }
    }

    Texture2D::from_image(&image)
}
