use std::{fmt, mem};
use std::any::{Any, TypeId};
use std::cell::{UnsafeCell, RefCell, Ref, RefMut};
use std::collections::HashMap;
use erased_serde::{Serializer, Deserializer};

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
    for (t, v) in self.components() {
      if let Some((ser, de)) = unsafe { COMPONENTS.get(t) } {
        // use serializer.serialize_map
        // use bincode::{DefaultOptions};
        // let mut se = bincode::Serializer::new(
        //   std::fs::File::create("test").unwrap(),
        //   DefaultOptions::new(),
        // );
        let mut json_se = serde_json::Serializer::pretty(std::fs::File::create("test").unwrap());
        ser(
          // Ref::leak(v[0].1.borrow()), // dontleak
          v[0].1.borrow(),
          &mut <dyn Serializer>::erase(&mut json_se),
        );

        // let json_de = serde_json::Deserializer::from_reader(std::fs::File::open("test").unwrap());
        // de()
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
    for<'a> fn(&'a dyn Any, &'a mut dyn Serializer), //remove the silly for
    fn(&dyn Deserializer) -> Box<dyn Any>,
  ),
> = HashMap::with_hasher(unsafe { mem::transmute([0u64; 2]) });
