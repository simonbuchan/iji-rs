use macroquad::prelude::*;
use serde::Serialize;

use super::{texture_from_data, Asset};

#[derive(Serialize)]
pub struct SpriteAsset {
    pub size: UVec2,
    pub origin: IVec2,
    #[serde(skip)]
    pub textures: Vec<Texture2D>,
}

impl SpriteAsset {
    pub fn bounds(&self) -> Rect {
        Rect::new(
            self.origin.x as f32,
            self.origin.y as f32,
            self.size.x as f32,
            self.size.y as f32,
        )
    }
}

impl Drop for SpriteAsset {
    fn drop(&mut self) {
        for t in &self.textures {
            t.delete();
        }
    }
}

impl Asset for SpriteAsset {
    type Resource = gmk_file::Sprite;

    fn load(def: &gmk_file::Sprite) -> Self {
        let textures = def
            .subimages
            .iter()
            .map(|image| texture_from_data(image.data.as_ref().unwrap(), def.transparent.into()))
            .collect::<Vec<_>>();

        Self {
            size: uvec2(def.size.0, def.size.1),
            origin: ivec2(def.origin.0, def.origin.1),
            textures,
        }
    }
}
