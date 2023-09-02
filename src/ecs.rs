use std::fmt;
use std::any::{Any, TypeId};
use std::cell::{UnsafeCell, RefCell, Ref};
use std::collections::HashMap;

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
      .iter()
      .filter_map(|c| (c.0.id == self.id).then_some(Ref::clone(&c.1)))
      .collect()
  }

  pub fn get_one<T: Any>(&self) -> Option<Ref<T>> {
    self
      .world
      .get()
      .iter()
      .find_map(|c| (c.0.id == self.id).then_some(Ref::clone(&c.1)))
  }
}

impl fmt::Debug for Entity<'_> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_fmt(format_args!("{}", self.id))
  }
}
