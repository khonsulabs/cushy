use std::any::{Any, TypeId};

use euclid::{Rect, Size2D};

pub trait Widget: Send + Sync + 'static {
    type MaterializerEvent: Send + Sync;
    type State: Send + Sync + Eq;
    type Layout: Send + Sync + for<'a> Layout<'a>;

    fn state(&self) -> Self::State;
    fn layout(&self) -> Self::Layout;
}

pub trait Materializer<F>: Send + Sync {
    type Widget: Widget;
}

pub trait Frontend: Sized {}

pub struct Gooey {
    root: Box<dyn AnyWidget>,
}

impl Gooey {
    pub fn new<W: Widget>(root: W) -> Self {
        Self {
            root: Box::new(WidgetState {
                widget: root,
                state: None,
            }),
        }
    }

    pub fn update(&mut self) -> bool {
        self.root.update()
    }

    pub fn layout_within(&'_ self, size: Size2D<f32, Points>) -> Vec<WidgetLayout<'_>> {
        self.root.layout_within(size)
    }

    pub fn root_widget(&self) -> &dyn AnyWidget {
        self.root.as_ref()
    }
}

pub trait Layout<'a> {
    fn layout_within(&mut self, size: Size2D<f32, Points>) -> Vec<WidgetLayout<'a>>;
}

pub struct Points;

pub struct WidgetLayout<'a> {
    widget: &'a dyn AnyWidget,
    location: Rect<f32, Points>,
}

pub struct WidgetState<W: Widget> {
    widget: W,
    state: Option<W::State>,
}

pub trait AnyWidget: Send + Sync {
    fn as_any(&'_ self) -> &'_ dyn Any;
    fn widget_type_id(&self) -> TypeId;
    fn state_as_any(&self) -> Option<&'_ dyn Any>;

    fn layout_within(&'_ self, size: Size2D<f32, Points>) -> Vec<WidgetLayout<'_>>;
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
}

impl<'a> Layout<'a> for () {
    fn layout_within(&mut self, _size: Size2D<f32, Points>) -> Vec<WidgetLayout<'a>> {
        Vec::default()
    }
}
