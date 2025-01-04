use std::fmt::Debug;

use crate::widget::{MakeWidget, WidgetRef, WrapperWidget};
use crate::widgets::Space;

/// A widget that stores arbitrary data in the widget hierachy.
///
/// This widget is useful if data needs to live as long as a related widget. For
/// example, [`ProgressBar`](crate::widgets::ProgressBar) is not a "real" widget
/// -- it implements [`MakeWidget`] and returns a customized
/// [`Slider`](crate::widgets::Slider). To ensure the indeterminant animation
/// lives only as long as the created slider does, `ProgressBar` wraps the
/// `Slider` in a `Data` widget to store the animation handle.
#[derive(Debug)]
pub struct Data<T> {
    #[allow(dead_code)] // This affects formatting in Debug to rename it.
    data: T,
    child: WidgetRef,
}

impl<T> Data<T>
where
    T: Debug,
{
    /// Returns an empty widget with the contained value.
    pub fn new(value: T) -> Self {
        Self::new_wrapping(value, Space::clear())
    }

    /// Returns a new instance that wraps `widget` and stores `value`.
    pub fn new_wrapping(value: T, widget: impl MakeWidget) -> Self {
        Self {
            data: value,
            child: WidgetRef::new(widget),
        }
    }

    /// Returns a reference to the wrapped data.
    pub fn data(&self) -> &T {
        &self.data
    }
}

impl<T> From<T> for Data<T>
where
    T: Debug,
{
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> WrapperWidget for Data<T>
where
    T: Debug + Send + 'static,
{
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }
}
