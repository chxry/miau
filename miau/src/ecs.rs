use std::{fmt, mem, panic};
use std::any::{Any, TypeId};
use std::cell::{UnsafeCell, RefCell, Ref, RefMut};
use std::ops::Deref;
use std::collections::HashMap;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::ser::SerializeMap;
use serde::de::{Visitor, MapAccess};
use erased_serde::Deserializer as ErasedDeserializer;
use log::error;
use crate::Result;

pub use miau_macros::component;

pub mod stage {
  pub const INIT: u64 = 0;
  pub const START: u64 = 1;
  pub const UPDATE: u64 = 2;
  pub const DRAW: u64 = 3;
}

pub trait System = Fn(&World) -> Result;

pub struct World {
  components: UnsafeCell<Storage>,
  resources: UnsafeCell<HashMap<TypeId, Box<dyn Any>>>,
  systems: UnsafeCell<HashMap<u64, Vec<(&'static str, Box<dyn System>)>>>,
}

impl World {
  pub fn new() -> Self {
    Self {
      components: UnsafeCell::new(Storage::new()),
      resources: UnsafeCell::new(HashMap::new()),
      systems: UnsafeCell::new(HashMap::new()),
    }
  }

  pub(crate) fn components(&self) -> &Storage {
    unsafe { &*self.components.get() }
  }

  pub(crate) fn components_mut(&self) -> &mut Storage {
    unsafe { &mut *self.components.get() }
  }

  pub fn spawn(&self) -> Entity {
    Entity {
      world: self,
      id: rand::random(),
    }
  }

  pub fn get<T: Any>(&self) -> Vec<(Entity, Ref<T>)> {
    match self.components().0.get(&TypeId::of::<T>()) {
      Some(v) => v
        .iter()
        .map(|c| {
          (
            Entity::new(self, c.0),
            Ref::map(c.1 .0.borrow(), unsafe { |r| r.downcast_ref_unchecked() }),
          )
        })
        .collect(),
      None => vec![],
    }
  }

  pub fn get_mut<T: Any>(&self) -> Vec<(Entity, RefMut<T>)> {
    match self.components().0.get(&TypeId::of::<T>()) {
      Some(v) => v
        .iter()
        .map(|c| {
          (
            Entity::new(self, c.0),
            RefMut::map(c.1 .0.borrow_mut(), unsafe {
              |r| r.downcast_mut_unchecked()
            }),
          )
        })
        .collect(),
      None => vec![],
    }
  }

  pub fn add_resource<T: Any>(&self, resource: T) {
    unsafe { &mut *self.resources.get() }.insert(TypeId::of::<T>(), Box::new(resource));
  }

  pub fn get_resource<T: Any>(&self) -> Option<&T> {
    unsafe { &*self.resources.get() }
      .get(&TypeId::of::<T>())
      .map(|r| unsafe { r.downcast_ref_unchecked() })
  }

  pub fn get_resource_mut<T: Any>(&self) -> Option<&mut T> {
    unsafe { &mut *self.resources.get() }
      .get_mut(&TypeId::of::<T>())
      .map(|r| unsafe { r.downcast_mut_unchecked() })
  }

  pub fn take_resource<T: Any>(&self) -> Option<T> {
    unsafe { &mut *self.resources.get() }
      .remove(&TypeId::of::<T>())
      .map(|r| *unsafe { r.downcast_unchecked() })
  }

  pub fn add_system<S: System + 'static>(&self, stage: u64, s: S) {
    unsafe { &mut *self.systems.get() }
      .entry(stage)
      .or_insert(vec![])
      .push((std::any::type_name::<S>(), Box::new(s)));
  }

  pub fn run_system(&self, stage: u64) {
    if let Some(vec) = unsafe { &*self.systems.get() }.get(&stage) {
      for (name, sys) in vec {
        panic::set_hook(Box::new(|info| {
          error!("Error in system '{}': {}", *name, info);
        }));
        if let Err(e) = sys(self) {
          panic!("{}", e);
        }
        let _ = panic::take_hook();
      }
    }
  }
}

pub struct Entity<'w> {
  world: &'w World,
  id: u64,
}

impl<'w> Entity<'w> {
  fn new(world: &'w World, id: u64) -> Self {
    Self { world, id }
  }

  pub fn insert<T: Any>(&self, t: T) -> &Self {
    self
      .world
      .components_mut()
      .0
      .entry(TypeId::of::<T>())
      .or_insert(vec![])
      .push((self.id, Component::new(t)));
    self
  }

  pub fn get<T: Any>(&self) -> Vec<Ref<T>> {
    self
      .world
      .get()
      .into_iter()
      .filter_map(|c| (c.0.id == self.id).then_some(c.1))
      .collect()
  }

  pub fn get_one<T: Any>(&self) -> Option<Ref<T>> {
    self
      .world
      .get()
      .into_iter()
      .find_map(|c| (c.0.id == self.id).then_some(c.1))
  }

  pub fn get_mut<T: Any>(&self) -> Vec<RefMut<T>> {
    self
      .world
      .get_mut()
      .into_iter()
      .filter_map(|c| (c.0.id == self.id).then_some(c.1))
      .collect()
  }

  pub fn get_one_mut<T: Any>(&self) -> Option<RefMut<T>> {
    self
      .world
      .get_mut()
      .into_iter()
      .find_map(|c| (c.0.id == self.id).then_some(c.1))
  }
}

impl fmt::Debug for Entity<'_> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_fmt(format_args!("{}", self.id))
  }
}

#[doc(hidden)]
pub static mut COMPONENTS: HashMap<
  TypeId,
  (
    fn(Ref<dyn Any>) -> Ref<dyn erased_serde::Serialize>,
    fn(&mut dyn ErasedDeserializer) -> Box<RefCell<dyn Any>>,
  ),
> = HashMap::with_hasher(unsafe { mem::transmute([0u64; 2]) });
static mut CURRENT: TypeId = TypeId::of::<()>();

pub(crate) struct Component(Box<RefCell<dyn Any>>);

impl Component {
  fn new<T: Any>(t: T) -> Self {
    Self(Box::new(RefCell::new(t)))
  }
}

impl Serialize for Component {
  fn serialize<S: Serializer>(&self, se: S) -> Result<S::Ok, S::Error> {
    let ser = unsafe { COMPONENTS.get(&CURRENT).unwrap().0 };
    erased_serde::serialize(ser(self.0.borrow()).deref(), se)
  }
}

impl<'de> Deserialize<'de> for Component {
  fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
    let de = unsafe { COMPONENTS.get(&CURRENT).unwrap().1 };
    Ok(Self(de(&mut <dyn ErasedDeserializer>::erase(d))))
  }
}

pub(crate) struct Storage(pub(crate) HashMap<TypeId, Vec<(u64, Component)>>);

impl Storage {
  fn new() -> Self {
    Self(HashMap::new())
  }
}

impl Serialize for Storage {
  fn serialize<S: Serializer>(&self, se: S) -> Result<S::Ok, S::Error> {
    let mut map = se.serialize_map(Some(self.0.len()))?;
    for (t, v) in &self.0 {
      unsafe {
        if COMPONENTS.get(t).is_some() {
          CURRENT = *t;
          map.serialize_entry(&mem::transmute::<_, u64>(*t), v)?;
        } else {
          log::warn!("Cannot serialize {:?}", t);
        }
      }
    }
    map.end()
  }
}

impl<'de> Deserialize<'de> for Storage {
  fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
    de.deserialize_map(MapVisitor)
  }
}

struct MapVisitor;

impl<'de> Visitor<'de> for MapVisitor {
  type Value = Storage;

  fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", std::any::type_name::<Storage>())
  }

  fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
    let mut map = HashMap::with_capacity(a.size_hint().unwrap_or_default());
    while let Ok(Some(t)) = a.next_key() {
      unsafe {
        CURRENT = mem::transmute::<u64, _>(t);
        map.insert(CURRENT, a.next_value().unwrap());
      }
    }
    Ok(Storage(map))
  }
}
