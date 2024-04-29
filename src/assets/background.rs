use glam::{uvec2, UVec2};
use macroquad::prelude::Texture2D;
use serde::Serialize;

use super::{texture_from_data, Asset};

#[derive(Serialize)]
pub struct BackgroundAsset {
    #[serde(skip)]
    pub texture: Texture2D,
    pub size: UVec2,
    pub tile_enabled: bool,
    pub tile_pos: UVec2,
    pub tile_size: UVec2,
}

impl Drop for BackgroundAsset {
    fn drop(&mut self) {
        self.texture.delete();
    }
}

impl Asset for BackgroundAsset {
    type Resource = gmk_file::Background;

    fn load(def: &gmk_file::Background) -> Self {
        let data = def.image.as_ref().unwrap().data.as_ref().unwrap();
        let texture = texture_from_data(data, def.transparent.into());
        // always present since GM 5.x
        let tiling = def.tiling.as_ref().unwrap();

        Self {
            texture,
            size: uvec2(def.size.0, def.size.1),
            tile_enabled: tiling.enabled.into(),
            tile_pos: uvec2(tiling.offset.0, tiling.offset.1),
            tile_size: uvec2(tiling.size.0, tiling.size.1),
        }
    }
}
