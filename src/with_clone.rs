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

impl_with_clone!(T1 0);
impl_with_clone!(T1 0, T2 1);
impl_with_clone!(T1 0, T2 1, T3 2);
impl_with_clone!(T1 0, T2 1, T3 2, T4 3);
impl_with_clone!(T1 0, T2 1, T3 2, T4 3, T5 4);
impl_with_clone!(T1 0, T2 1, T3 2, T4 3, T5 4, T6 5);
