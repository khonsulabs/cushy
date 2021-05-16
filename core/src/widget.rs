use std::any::{Any, TypeId};

use euclid::Size2D;
use stylecs::Points;

pub trait Widget: Send + Sync + 'static {
    type TransmogrifierEvent: Send + Sync;
    type State: Send + Sync + Eq;

    fn state(&self) -> Self::State;
}

pub trait Transmogrifier<F>: Send + Sync {
    type Widget: Widget;
    type Context: Send + Sync;

    fn content_size(
        &self,
        state: &<Self::Widget as Widget>::State,
        constraints: Size2D<Option<f32>, Points>,
        context: &Self::Context,
    ) -> Size2D<f32, Points>;
}

pub struct WidgetState<W: Widget> {
    pub widget: W,
    pub state: Option<W::State>,
}

pub trait AnyWidget: Send + Sync {
    fn as_any(&'_ self) -> &'_ dyn Any;
    fn widget_type_id(&self) -> TypeId;
    fn state_as_any(&self) -> Option<&'_ dyn Any>;

    fn update(&mut self) -> bool;
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
}
