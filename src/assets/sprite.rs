use macroquad::prelude::*;
use serde::Serialize;

use super::{texture_from_data, Asset};
use crate::state::serialize_rect;

#[derive(Serialize)]
pub struct SpriteAsset {
    pub size: UVec2,
    pub origin: IVec2,
    #[serde(skip)]
    pub textures: Vec<Texture2D>,
    #[serde(serialize_with = "serialize_rect")]
    pub bbox: Rect,
}

impl SpriteAsset {
    pub fn bounds(&self, pos: Vec2) -> Rect {
        self.bbox.offset(pos - self.origin.as_vec2())
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

        let bbox_origin = ivec2(def.bbox_left, def.bbox_top).as_vec2();
        let bbox_size = ivec2(def.bbox_right, def.bbox_bottom).as_vec2() - bbox_origin;

        let bbox = Rect::new(bbox_origin.x, bbox_origin.y, bbox_size.x, bbox_size.y);

        Self {
            size: uvec2(def.size.0, def.size.1),
            origin: ivec2(def.origin.0, def.origin.1),
            textures,
            bbox,
        }
    }
}
