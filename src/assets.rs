use std::collections::HashMap;
use std::marker::PhantomData;

use macroquad::prelude::*;

trait Asset {
    type Resource;

    fn load(res: &Self::Resource) -> Self;
}

pub struct AssetId<T>(u32, PhantomData<T>);

impl<T> AssetId<T> {
    pub fn new(index: u32) -> Self {
        Self(index, PhantomData)
    }
}

// Need to manually impl Copy, Clone due to T parameter
impl<T> Clone for AssetId<T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<T> Copy for AssetId<T> {}

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
        AssetId(index, PhantomData)
    }

    pub fn entry(&self, id: AssetId<T>) -> (&str, &T) {
        let (name, item) = &self.items[&id.0];
        (name, item)
    }

    pub fn get(&self, id: AssetId<T>) -> &T {
        &self.items[&id.0].1
    }
}

#[derive(Default)]
pub struct Assets {
    pub backgrounds: AssetSet<BackgroundAsset>,
    pub sprites: AssetSet<SpriteAsset>,
}

pub struct Loader<'content> {
    // indirecting content so it can be concurrently referenced
    content: &'content gmk_file::Content,
    assets: Assets,
}

impl<'content> Loader<'content> {
    pub fn new(content: &'content gmk_file::Content) -> Self {
        Self {
            content,
            assets: Default::default(),
        }
    }

    pub fn content(&self) -> &'content gmk_file::Content {
        self.content
    }

    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    pub fn reset_assets(&mut self) {
        self.assets = Assets::default();
    }

    pub fn take_assets(&mut self) -> Assets {
        std::mem::take(&mut self.assets)
    }

    pub fn get_background(&mut self, index: u32) -> AssetId<BackgroundAsset> {
        self.assets
            .backgrounds
            .load(&self.content.backgrounds, index)
    }

    pub fn get_sprite(&mut self, index: u32) -> AssetId<SpriteAsset> {
        self.assets.sprites.load(&self.content.sprites, index)
    }
}

pub struct BackgroundAsset {
    pub texture: Texture2D,
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
        let texture = Texture2D::from_file_with_format(data, None);
        // always present since GM 5.x
        // let tiling = def.tiling.as_ref().unwrap();

        // let mut tile_size = None;
        // if tiling.enabled == gmk_file::Bool32::True {
        //     tiling.
        // }
        Self { texture }
    }
}

pub struct SpriteAsset {
    pub size: Vec2,
    pub origin: Vec2,
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
            .map(|image| {
                let data = image.data.as_ref().unwrap();
                let mut image = Image::from_file_with_format(data, None);

                if def.transparent == gmk_file::Bool32::True {
                    let data = image.get_image_data_mut();
                    let t = data[0];
                    for p in data {
                        if *p == t {
                            *p = [0, 0, 0, 0];
                        }
                    }
                }

                Texture2D::from_image(&image)
            })
            .collect::<Vec<_>>();

        let size = Vec2::new(def.size.0 as f32, def.size.1 as f32);
        let origin = Vec2::new(def.origin.0 as f32, def.origin.1 as f32);

        Self {
            size,
            origin,
            textures,
        }
    }
}

impl Asset for gml::ast::Script {
    type Resource = gmk_file::Script;

    fn load(res: &gmk_file::Script) -> Self {
        gml::parse(&res.script).unwrap()
    }
}
