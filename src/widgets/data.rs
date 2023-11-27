use std::fmt::Debug;
use std::panic::UnwindSafe;

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
    _data: T,
    child: WidgetRef,
}

impl<T> Data<T> {
    /// Returns an empty widget with the contained value.
    pub fn new(value: T) -> Self {
        Self::new_wrapping(value, Space::clear())
    }

    /// Returns a new instance that wraps `widget` and stores `value`.
    pub fn new_wrapping(value: T, widget: impl MakeWidget) -> Self {
        Self {
            _data: value,
            child: WidgetRef::new(widget),
        }
    }
}

impl<T> From<T> for Data<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> WrapperWidget for Data<T>
where
    T: Debug + Send + UnwindSafe + 'static,
{
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }
}
