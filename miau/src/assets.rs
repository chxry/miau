use std::mem;
use std::fs::File;
use std::collections::HashMap;
use std::any::{Any, TypeId, type_name};
use std::rc::Rc;
use std::ops::Deref;
use vach::archive::Archive;
use serde::{Serialize, Deserialize, Deserializer, de::Error};
use crate::{Result, world};

pub use macros::asset;

pub struct Assets {
  archive: Archive<File>,
}

impl Assets {
  pub fn new() -> Result<Self> {
    Ok(Self {
      archive: Archive::new(File::open("assets.vach")?)?,
    })
  }

  pub fn load<T: Any>(&self, path: &str) -> Result<Handle<T>> {
    match unsafe { ASSET_LOADERS.get_mut(&TypeId::of::<T>()) } {
      Some(loader) => match loader.assets.iter().find(|h| h.path == path) {
        Some(asset) => Ok(asset.downcast()),
        None => (loader.loader)(&self.archive.fetch(format!("assets/{}", path))?.data).map(|a| {
          loader.assets.push(Handle::new(path, a.clone()));
          Handle::new(path, unsafe { a.downcast_unchecked() })
        }),
      },
      None => Err("todo".into()),
    }
  }

  fn get() -> &'static Self {
    world().get_resource().unwrap()
  }
}

#[doc(hidden)]
pub static mut ASSET_LOADERS: HashMap<TypeId, AssetLoader> =
  HashMap::with_hasher(unsafe { mem::transmute([0u64; 2]) });

#[derive(Serialize)]
#[serde(transparent)]
pub struct Handle<T: ?Sized> {
  path: String,
  #[serde(skip)]
  data: Rc<T>,
}

impl<T: ?Sized> Handle<T> {
  fn new(path: &str, data: Rc<T>) -> Self {
    Self {
      path: path.to_string(),
      data,
    }
  }
}

impl Handle<dyn Any> {
  fn downcast<T: Any>(&self) -> Handle<T> {
    Handle {
      path: self.path.clone(),
      data: unsafe { self.data.clone().downcast_unchecked() },
    }
  }
}

impl<T> Deref for Handle<T> {
  type Target = T;

  fn deref(&self) -> &T {
    &self.data
  }
}

impl<'de, T: Any> Deserialize<'de> for Handle<T> {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let path: String = Deserialize::deserialize(deserializer)?;
    Assets::get().load(&path).map_err(|e| {
      D::Error::custom(format!(
        "could not load '{}' from '{} - {}'",
        type_name::<T>(),
        path,
        e
      ))
    })
  }
}

pub struct AssetLoader {
  pub loader: fn(&[u8]) -> Result<Rc<dyn Any>>,
  pub assets: Vec<Handle<dyn Any>>,
}
