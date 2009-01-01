use std::fs;
use std::mem::MaybeUninit;
use std::collections::HashMap;
use std::any::{Any, TypeId, type_name};
use std::rc::Rc;
use std::ops::Deref;
use serde::{Serialize, Deserialize, Deserializer, de::Error};
use crate::Result;

static mut ASSETS: MaybeUninit<Assets> = MaybeUninit::uninit();

pub fn init() -> &'static mut Assets {
  let _ = unsafe { ASSETS.write(Assets::new()) };
  assets()
}

#[inline(always)]
pub fn assets() -> &'static mut Assets {
  unsafe { ASSETS.assume_init_mut() }
}

pub struct Assets {
  loaders: HashMap<TypeId, AssetLoader>,
}

impl Assets {
  fn new() -> Self {
    Self {
      loaders: HashMap::new(),
    }
  }

  pub fn register_loader<T: Any>(&mut self, f: fn(&[u8]) -> Result<T>) {
    self.loaders.insert(
      TypeId::of::<T>(),
      AssetLoader {
        loader: Box::new(move |d| f(d).map(|a| Rc::new(a) as _)),
        assets: vec![],
      },
    );
  }

  pub fn load<T: Any>(&mut self, path: &str) -> Result<Handle<T>> {
    match self.loaders.get_mut(&TypeId::of::<T>()) {
      Some(loader) => match loader.assets.iter().find(|h| h.path == path) {
        Some(asset) => Ok(asset.downcast()),
        None => (loader.loader)(&fs::read(path)?).map(|a| {
          loader.assets.push(Handle::new(path, a.clone()));
          Handle::new(path, unsafe { a.downcast_unchecked() })
        }),
      },
      None => Err("todo".into()),
    }
  }
}

#[derive(Serialize)]
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
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
    let path = Deserialize::deserialize(deserializer)?;
    assets().load(path).map_err(|e| {
      D::Error::custom(format!(
        "could not load '{}' from '{} - {}'",
        type_name::<T>(),
        path,
        e
      ))
    })
  }
}

struct AssetLoader {
  loader: Box<dyn Fn(&[u8]) -> Result<Rc<dyn Any>>>,
  assets: Vec<Handle<dyn Any>>,
}
