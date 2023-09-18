use std::{fmt, mem};
use std::any::{Any, TypeId};
use std::cell::{UnsafeCell, RefCell, Ref, RefMut};
use std::ops::Deref;
use std::collections::HashMap;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::ser::SerializeMap;
use serde::de::{Visitor, MapAccess};
use erased_serde::Deserializer as ErasedDeserializer;

pub struct World {
  components: UnsafeCell<Storage>,
}

impl World {
  pub fn new() -> Self {
    Self {
      components: UnsafeCell::new(Storage::new()),
    }
  }

  fn components(&self) -> &Storage {
    unsafe { &*self.components.get() }
  }

  fn components_mut(&self) -> &mut Storage {
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

  pub fn save(&self) {
    serde_json::to_writer(std::fs::File::create("test").unwrap(), &self.components()).unwrap();
  }

  pub fn load(&self) {
    *self.components_mut() = serde_json::from_reader(std::fs::File::open("test").unwrap()).unwrap();
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

pub use macros::component;

#[doc(hidden)]
pub static mut COMPONENTS: HashMap<
  TypeId,
  (
    fn(Ref<dyn Any>) -> Ref<dyn erased_serde::Serialize>,
    fn(&mut dyn ErasedDeserializer) -> Box<RefCell<dyn Any>>,
  ),
> = HashMap::with_hasher(unsafe { mem::transmute([0u64; 2]) });
static mut SILLY: TypeId = TypeId::of::<()>();

struct Component(Box<RefCell<dyn Any>>);

impl Component {
  fn new<T: Any>(t: T) -> Self {
    Self(Box::new(RefCell::new(t)))
  }
}

impl Serialize for Component {
  fn serialize<S: Serializer>(&self, se: S) -> Result<S::Ok, S::Error> {
    match unsafe { COMPONENTS.get(&SILLY) } {
      Some((ser, _)) => erased_serde::serialize(ser(self.0.borrow()).deref(), se),
      None => se.serialize_unit(),
    }
  }
}

impl<'de> Deserialize<'de> for Component {
  fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
    match unsafe { COMPONENTS.get(&SILLY) } {
      Some((_, de)) => Ok(Self(de(&mut <dyn ErasedDeserializer>::erase(d)))),
      None => panic!("unknown component"),
    }
  }
}

struct Storage(HashMap<TypeId, Vec<(u64, Component)>>);

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
        SILLY = *t;
        map.serialize_entry(&mem::transmute::<_, u64>(*t), v)?;
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
    write!(f, "meow")
  }

  fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
    let mut map = HashMap::with_capacity(a.size_hint().unwrap_or_default());
    while let Ok(Some(t)) = a.next_key() {
      unsafe {
        SILLY = mem::transmute::<u64, _>(t);
        map.insert(SILLY, a.next_value().unwrap());
      }
    }
    Ok(Storage(map))
  }
}
