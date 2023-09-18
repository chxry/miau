use std::{fmt, mem};
use std::any::{Any, TypeId};
use std::cell::{UnsafeCell, RefCell, Ref, RefMut};
use std::ops::Deref;
use std::collections::HashMap;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::ser::SerializeMap;
use serde::de::{Visitor, MapAccess};
use erased_serde::Deserializer as ErasedDeserializer;

type Storage = HashMap<TypeId, Vec<(u64, Box<RefCell<dyn Any>>)>>;

pub struct World {
  components: UnsafeCell<Storage>,
}

impl World {
  pub fn new() -> Self {
    Self {
      components: UnsafeCell::new(HashMap::new()),
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
    match self.components().get(&TypeId::of::<T>()) {
      Some(v) => v
        .iter()
        .map(|c| {
          (
            Entity::new(self, c.0),
            Ref::map(c.1.borrow(), unsafe { |r| r.downcast_ref_unchecked() }),
          )
        })
        .collect(),
      None => vec![],
    }
  }

  pub fn get_mut<T: Any>(&self) -> Vec<(Entity, RefMut<T>)> {
    match self.components().get(&TypeId::of::<T>()) {
      Some(v) => v
        .iter()
        .map(|c| {
          (
            Entity::new(self, c.0),
            RefMut::map(c.1.borrow_mut(), unsafe { |r| r.downcast_mut_unchecked() }),
          )
        })
        .collect(),
      None => vec![],
    }
  }

  pub fn save(&self) {
    let mut n = Bleh(HashMap::new());
    for (t, v) in self.components() {
      if unsafe { COMPONENTS.get(t).is_some() } {
        n.0.insert(
          *t,
          v.iter() //                         this isnt real
            .map(|c| (c.0, SerdeComponent(unsafe { mem::transmute_copy(&c.1) })))
            .collect(),
        );
      }
    }
    serde_json::to_writer(std::fs::File::create("test").unwrap(), &n).unwrap();
  }

  pub fn load(&self) {
    let n: Bleh = serde_json::from_reader(std::fs::File::open("test").unwrap()).unwrap();
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
      .entry(TypeId::of::<T>())
      .or_insert(vec![])
      .push((self.id, Box::new(RefCell::new(t))));
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

// maybe store all components in this thing
struct SerdeComponent(Box<RefCell<dyn Any>>);

impl Serialize for SerdeComponent {
  fn serialize<S: Serializer>(&self, se: S) -> Result<S::Ok, S::Error> {
    let ser = unsafe { COMPONENTS.get(&SILLY).unwrap() }.0;
    erased_serde::serialize(ser(self.0.borrow()).deref(), se)
  }
}

impl<'de> Deserialize<'de> for SerdeComponent {
  fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
    let de = unsafe { COMPONENTS.get(&SILLY).unwrap() }.1;
    Ok(Self(de(&mut <dyn ErasedDeserializer>::erase(d))))
  }
}

struct Bleh(HashMap<TypeId, Vec<(u64, SerdeComponent)>>);

impl Serialize for Bleh {
  fn serialize<S: Serializer>(&self, se: S) -> Result<S::Ok, S::Error> {
    let mut map = se.serialize_map(Some(self.0.len()))?;
    for (t, v) in &self.0 {
      unsafe { SILLY = *t };
      map.serialize_entry(&unsafe { mem::transmute::<_, u64>(*t) }, v)?;
    }
    map.end()
  }
}

impl<'de> Deserialize<'de> for Bleh {
  fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
    de.deserialize_map(MapVisitor)
  }
}

struct MapVisitor;

impl<'de> Visitor<'de> for MapVisitor {
  type Value = Bleh;

  fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "meow")
  }

  fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
    let mut map = HashMap::with_capacity(a.size_hint().unwrap_or_default());
    while let Ok(Some(t)) = a.next_key() {
      unsafe { SILLY = mem::transmute::<u64, _>(t) };
      map.insert(unsafe { SILLY }, a.next_value().unwrap());
    }
    Ok(Bleh(map))
  }
}
