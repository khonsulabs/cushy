/// Invokes a function with a clone of `self`.
pub trait WithClone: Sized {
    /// The type that results from cloning.
    type Cloned;

    /// Maps `with` with the results of cloning `self`.
    fn with_clone<R>(&self, with: impl FnOnce(Self::Cloned) -> R) -> R;
}

macro_rules! impl_with_clone {
    ($($name:ident $field:tt),+) => {
        impl<'a, $($name: Clone,)+> WithClone for ($(&'a $name,)+)
        {
            type Cloned = ($($name,)+);

            fn with_clone<R>(&self, with: impl FnOnce(Self::Cloned) -> R) -> R {
                with(($(self.$field.clone(),)+))
            }
        }
    };
}

impl<'a, T> WithClone for &'a T
where
    T: Clone,
{
    type Cloned = T;

    fn with_clone<R>(&self, with: impl FnOnce(Self::Cloned) -> R) -> R {
        with((*self).clone())
    }
}

impl_all_tuples!(impl_with_clone);
