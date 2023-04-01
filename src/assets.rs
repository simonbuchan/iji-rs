use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;

use macroquad::prelude::*;
use serde::{Serialize, Serializer};

pub trait Asset {
    type Resource;

    fn load(res: &Self::Resource) -> Self;
}

pub struct AssetId<T>(u32, PhantomData<T>);

impl<T> Serialize for AssetId<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("AssetId", &self.0)
    }
}

impl<T> AssetId<T> {}

impl<T> AssetId<T> {
    pub fn new(index: u32) -> Self {
        Self(index, PhantomData)
    }
}

impl<T> std::fmt::Debug for AssetId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

// Need to manually impl Copy, Clone due to T parameter
impl<T> Clone for AssetId<T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<T> Copy for AssetId<T> {}

#[derive(Serialize)]
pub struct AssetSet<T> {
    indices: HashMap<String, u32>,
    items: HashMap<u32, (String, T)>,
}

impl<T> Default for AssetSet<T> {
    fn default() -> Self {
        Self {
            indices: Default::default(),
            items: Default::default(),
        }
    }
}

impl<T: Asset> AssetSet<T> {
    fn load(&mut self, chunk: &gmk_file::ResourceChunk<T::Resource>, index: u32) -> AssetId<T> {
        self.items.entry(index).or_insert_with(|| {
            let item = chunk.items.get(index as usize).unwrap().as_ref().unwrap();
            let name = item.name.0.clone();
            self.indices.insert(name.clone(), index);
            (name, T::load(&item.data))
        });
        AssetId::new(index)
    }

    pub fn get(&self, id: AssetId<T>) -> &T {
        &self.items[&id.0].1
    }
}

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

    pub fn get_background(&mut self, index: u32) -> AssetId<BackgroundAsset> {
        self.assets
            .borrow_mut()
            .backgrounds
            .load(&self.content.backgrounds, index)
    }

    pub fn get_sprite(&mut self, index: u32) -> AssetId<SpriteAsset> {
        self.assets
            .borrow_mut()
            .sprites
            .load(&self.content.sprites, index)
    }
}

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

#[derive(Serialize)]
pub struct SpriteAsset {
    pub size: UVec2,
    pub origin: IVec2,
    #[serde(skip)]
    pub textures: Vec<Texture2D>,
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
