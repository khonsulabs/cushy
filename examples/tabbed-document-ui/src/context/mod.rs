use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Default)]
pub struct Context {
    values: HashMap<TypeId, Box<dyn Any + Send>>
}

impl Context {
    pub fn provide<T: Sized + Send + Any>(&mut self, value: T) -> Option<T>{
        let key = TypeId::of::<T>();
        self.values.insert(key, Box::new(value))
            .map(|value|{

                fn unbox<T>(value: Box<T>) -> T {
                    *value
                }

                unbox(value.downcast::<T>().unwrap())
            })
    }

    pub fn with_context<T: Sized + Any, F: FnOnce(&mut T) -> R, R>(&mut self, f: F) -> Option<R> {
        let key = TypeId::of::<T>();
        self.values.get_mut(&key)
            .map(|boxed_value|{
                let value = boxed_value.downcast_mut().unwrap();

                f(value)
            })
    }
}