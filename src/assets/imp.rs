use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::marker::PhantomData;

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
        *self
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
    pub(super) fn load(
        &mut self,
        chunk: &gmk_file::ResourceChunk<T::Resource>,
        index: u32,
    ) -> AssetId<T> {
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
