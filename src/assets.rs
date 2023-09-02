use std::collections::HashMap;
use std::any::{Any, TypeId};
use std::rc::Rc;
use crate::Result;

pub type Handle<T> = Rc<T>;

pub struct Assets {
  loaders: HashMap<TypeId, AssetLoader>,
}

impl Assets {
  pub fn new() -> Self {
    Self {
      loaders: HashMap::new(),
    }
  }

  pub fn register_loader<T: Any>(&mut self, f: fn() -> Result<T>) {
    self.loaders.insert(
      TypeId::of::<T>(),
      AssetLoader {
        loader: Box::new(move || f().map(|a| Handle::new(a) as _)),
        assets: HashMap::new(),
      },
    );
  }

  pub fn load<T: Any>(&mut self, path: &str) -> Result<Handle<T>> {
    match self.loaders.get_mut(&TypeId::of::<T>()) {
      Some(loader) => unsafe {
        match loader.assets.get(path) {
          Some(asset) => Ok(asset.clone().downcast_unchecked()),
          None => (loader.loader)().map(|a| {
            loader.assets.insert(path.to_string(), a.clone());
            a.downcast_unchecked()
          }),
        }
      },
      None => Err("todo".into()),
    }
  }
}

struct AssetLoader {
  loader: Box<dyn Fn() -> Result<Handle<dyn Any>>>,
  assets: HashMap<String, Handle<dyn Any>>,
}
