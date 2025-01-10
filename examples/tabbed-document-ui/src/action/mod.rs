use std::ops::{Deref, DerefMut};

pub struct Action<T>(T);

impl<T> Action<T> {
    pub fn new(value: T) -> Action<T> {
        Self (value)
    }

    pub fn map<O>(
        self,
        mut f: impl FnMut(T) -> O
    ) -> Action<O> {
        let value = f(self.0);
        Action::new(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Action<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Action<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}


