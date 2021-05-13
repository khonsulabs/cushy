use std::any::{Any, TypeId};

use euclid::Size2D;

use crate::{Layout, Points, WidgetLayout};

pub trait Widget: Send + Sync + 'static {
    type MaterializerEvent: Send + Sync;
    type State: Send + Sync + Eq;
    type Layout: Send + Sync + for<'a> Layout<'a>;

    fn state(&self) -> Self::State;
    fn layout(&self) -> Self::Layout;
    fn content_size(&self, constraints: Size2D<Option<f32>, Points>) -> Size2D<f32, Points>;
}

pub trait Materializer<F>: Send + Sync {
    type Widget: Widget;
}

pub struct WidgetState<W: Widget> {
    pub widget: W,
    pub state: Option<W::State>,
}

pub trait AnyWidget: Send + Sync {
    fn as_any(&'_ self) -> &'_ dyn Any;
    fn widget_type_id(&self) -> TypeId;
    fn state_as_any(&self) -> Option<&'_ dyn Any>;

    fn layout_within(&'_ self, size: Size2D<f32, Points>) -> Vec<WidgetLayout<'_>>;
    fn update(&mut self) -> bool;
    fn content_size(&self, constraints: Size2D<Option<f32>, Points>) -> Size2D<f32, Points>;
}

impl<T> AnyWidget for WidgetState<T>
where
    T: Widget + Any,
{
    fn as_any(&'_ self) -> &'_ dyn Any {
        self
    }

    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn layout_within(&'_ self, size: Size2D<f32, Points>) -> Vec<WidgetLayout<'_>> {
        let mut layout = self.widget.layout();
        layout.layout_within(size)
    }

    fn update(&mut self) -> bool {
        let new_state = self.widget.state();
        let changed = self
            .state
            .as_ref()
            .map(|old_state| old_state != &new_state)
            .unwrap_or(true);
        self.state = Some(new_state);
        changed
    }

    fn state_as_any(&self) -> Option<&'_ dyn Any> {
        if let Some(state) = &self.state {
            Some(state)
        } else {
            None
        }
    }

    fn content_size(&self, constraints: Size2D<Option<f32>, Points>) -> Size2D<f32, Points> {
        self.widget.content_size(constraints)
    }
}
