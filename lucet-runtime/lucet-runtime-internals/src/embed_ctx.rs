use std::any::{Any, TypeId};
use std::collections::HashMap;

/// A map that holds at most one value of any type.
///
/// This is similar to the type provided by the `anymap` crate, but we can get away with simpler
/// types on the methods due to our more specialized use case.
pub struct CtxMap {
    map: HashMap<TypeId, Box<dyn Any>>,
}

impl CtxMap {
    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn contains<T: Any>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }

    pub fn get<T: Any>(&self) -> Option<&T> {
        self.map.get(&TypeId::of::<T>()).map(|x| {
            x.downcast_ref::<T>()
                .expect("value stored with TypeId::of::<T> is always type T")
        })
    }

    pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.map.get_mut(&TypeId::of::<T>()).map(|x| {
            x.downcast_mut::<T>()
                .expect("value stored with TypeId::of::<T> is always type T")
        })
    }

    pub fn insert<T: Any>(&mut self, x: T) -> Option<T> {
        self.map
            .insert(TypeId::of::<T>(), Box::new(x) as Box<dyn Any>)
            .map(|x_prev| {
                *x_prev
                    .downcast::<T>()
                    .expect("value stored with TypeId::of::<T> is always type T")
            })
    }

    pub fn new() -> Self {
        CtxMap {
            map: HashMap::new(),
        }
    }

    pub fn remove<T: Any>(&mut self) -> Option<T> {
        self.map.remove(&TypeId::of::<T>()).map(|x| {
            *x.downcast::<T>()
                .expect("value stored with TypeId::of::<T> is always type T")
        })
    }
}
