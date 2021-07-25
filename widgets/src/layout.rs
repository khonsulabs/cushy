use std::{collections::HashMap, fmt::Debug, hash::Hash};

use gooey_core::{
    euclid::{Length, Size2D},
    styles::Surround,
    AnySendSync, Context, Points, StyledWidget, Widget, WidgetId, WidgetRegistration,
    WidgetStorage,
};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug)]
pub struct Layout {
    children: Box<dyn LayoutChildren>,
}

impl Layout {
    #[must_use]
    pub fn build<K: LayoutKey>(storage: &WidgetStorage) -> Builder<K> {
        Builder::new(storage)
    }

    pub fn remove_child<K: LayoutKey>(&mut self, layout_key: &K, context: &Context<Self>) -> bool {
        let children = self
            .children
            .as_mut()
            .as_mut_any()
            .downcast_mut::<ChildrenMap<K>>()
            .unwrap();
        children.remove(layout_key).map_or(false, |removed_child| {
            context.send_command(LayoutCommand::ChildRemoved(
                removed_child.registration.id().clone(),
            ));
            true
        })
    }

    pub fn insert<K: LayoutKey, W: Widget>(
        &mut self,
        layout_key: Option<K>,
        widget: StyledWidget<W>,
        layout: WidgetLayout,
        context: &Context<Self>,
    ) {
        let registration = context.register(widget);
        self.insert_registration(layout_key, registration, layout, context);
    }

    pub fn insert_registration<K: LayoutKey>(
        &mut self,
        layout_key: Option<K>,
        registration: WidgetRegistration,
        layout: WidgetLayout,
        context: &Context<Self>,
    ) {
        context.send_command(LayoutCommand::ChildAdded(registration.clone()));
        let children = self
            .children
            .as_mut()
            .as_mut_any()
            .downcast_mut::<ChildrenMap<K>>()
            .unwrap();
        children.insert(layout_key, LayoutChild {
            registration,
            layout,
        });
    }
}

#[derive(Debug)]
pub enum LayoutCommand {
    ChildRemoved(WidgetId),
    ChildAdded(WidgetRegistration),
}

impl Widget for Layout {
    type Command = LayoutCommand;
    type Event = ();

    const CLASS: &'static str = "gooey-layout";
}

#[derive(Debug)]
pub struct Builder<K: LayoutKey> {
    storage: WidgetStorage,
    children: ChildrenMap<K>,
}

#[derive(Debug)]
struct ChildrenMap<K> {
    children: HashMap<u32, LayoutChild>,
    keys_to_id: HashMap<K, u32>,
}

impl<K> Default for ChildrenMap<K> {
    fn default() -> Self {
        Self {
            children: HashMap::default(),
            keys_to_id: HashMap::default(),
        }
    }
}

impl<K: LayoutKey> ChildrenMap<K> {
    fn remove(&mut self, key: &K) -> Option<LayoutChild> {
        self.keys_to_id
            .remove(key)
            .and_then(|id| self.children.remove(&id))
    }

    fn insert(&mut self, key: Option<K>, child: LayoutChild) {
        if let Some(key) = key {
            self.keys_to_id.insert(key, child.registration.id().id);
        }
        self.children.insert(child.registration.id().id, child);
    }
}

#[derive(Clone, Debug)]
pub struct LayoutChild {
    pub registration: WidgetRegistration,
    pub layout: WidgetLayout,
}

impl<K: LayoutKey> Builder<K> {
    #[must_use]
    pub fn new(storage: &WidgetStorage) -> Self {
        Self {
            storage: storage.clone(),
            children: ChildrenMap::default(),
        }
    }

    pub fn with<W: Widget>(
        self,
        key: Option<K>,
        widget: StyledWidget<W>,
        layout: WidgetLayout,
    ) -> Self {
        let widget = self.storage.register(widget);
        self.with_registration(key, widget, layout)
    }

    pub fn with_registration(
        mut self,
        key: Option<K>,
        registration: WidgetRegistration,
        layout: WidgetLayout,
    ) -> Self {
        self.children.insert(key, LayoutChild {
            registration,
            layout,
        });
        self
    }

    pub fn finish(self) -> StyledWidget<Layout> {
        StyledWidget::default_for(Layout {
            children: Box::new(self.children),
        })
    }
}

pub trait LayoutKey: Hash + Debug + Eq + PartialEq + Send + Sync + 'static {}

impl<T> LayoutKey for T where T: Hash + Debug + Eq + PartialEq + Send + Sync + 'static {}

#[derive(Clone, Debug, Default)]
pub struct WidgetLayout {
    pub left: Dimension,
    pub top: Dimension,
    pub right: Dimension,
    pub bottom: Dimension,
    pub width: Dimension,
    pub height: Dimension,
}

impl WidgetLayout {
    pub fn with_left<D: Into<Dimension>>(mut self, left: D) -> Self {
        self.left = left.into();
        self
    }

    pub fn with_right<D: Into<Dimension>>(mut self, right: D) -> Self {
        self.right = right.into();
        self
    }

    pub fn with_top<D: Into<Dimension>>(mut self, top: D) -> Self {
        self.top = top.into();
        self
    }

    pub fn with_bottom<D: Into<Dimension>>(mut self, bottom: D) -> Self {
        self.bottom = bottom.into();
        self
    }

    pub fn with_width<D: Into<Dimension>>(mut self, width: D) -> Self {
        self.width = width.into();
        self
    }

    pub fn with_height<D: Into<Dimension>>(mut self, height: D) -> Self {
        self.height = height.into();
        self
    }

    #[must_use]
    pub fn left_in_points(&self, content_size: Size2D<f32, Points>) -> Option<Length<f32, Points>> {
        self.left.length(Length::new(content_size.width))
    }

    #[must_use]
    pub fn right_in_points(
        &self,
        content_size: Size2D<f32, Points>,
    ) -> Option<Length<f32, Points>> {
        self.right.length(Length::new(content_size.width))
    }

    #[must_use]
    pub fn top_in_points(&self, content_size: Size2D<f32, Points>) -> Option<Length<f32, Points>> {
        self.top.length(Length::new(content_size.height))
    }

    #[must_use]
    pub fn bottom_in_points(
        &self,
        content_size: Size2D<f32, Points>,
    ) -> Option<Length<f32, Points>> {
        self.bottom.length(Length::new(content_size.height))
    }

    #[must_use]
    pub fn width_in_points(&self, content_size: Size2D<f32, Points>) -> Length<f32, Points> {
        self.width
            .length(Length::new(content_size.width))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn height_in_points(&self, content_size: Size2D<f32, Points>) -> Length<f32, Points> {
        self.height
            .length(Length::new(content_size.height))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn surround_in_points(&self, content_size: Size2D<f32, Points>) -> Surround<Points> {
        Surround {
            left: self.left_in_points(content_size),
            top: self.top_in_points(content_size),
            right: self.right_in_points(content_size),
            bottom: self.bottom_in_points(content_size),
        }
    }

    #[must_use]
    pub fn size_in_points(&self, content_size: Size2D<f32, Points>) -> Size2D<f32, Points> {
        Size2D::from_lengths(
            self.width_in_points(content_size),
            self.height_in_points(content_size),
        )
    }
}

pub trait LayoutChildren: AnySendSync {
    fn layout_children(&self) -> Vec<LayoutChild>;
    fn child_by_widget_id(&self, widget_id: &WidgetId) -> Option<&LayoutChild>;
}

#[derive(Debug)]
pub struct LayoutTransmogrifier;

impl<K: LayoutKey> LayoutChildren for ChildrenMap<K> {
    fn layout_children(&self) -> Vec<LayoutChild> {
        self.children.values().cloned().collect()
    }

    fn child_by_widget_id(&self, widget_id: &WidgetId) -> Option<&LayoutChild> {
        self.children.get(&widget_id.id)
    }
}

impl LayoutChildren for Layout {
    fn layout_children(&self) -> Vec<LayoutChild> {
        self.children.layout_children()
    }

    fn child_by_widget_id(&self, widget_id: &WidgetId) -> Option<&LayoutChild> {
        self.children.child_by_widget_id(widget_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    Auto,
    Exact(Length<f32, Points>),
    Percent(f32),
}

impl Default for Dimension {
    fn default() -> Self {
        Self::Auto
    }
}

impl Dimension {
    #[must_use]
    pub fn length(self, content_length: Length<f32, Points>) -> Option<Length<f32, Points>> {
        match self {
            Dimension::Auto => None,
            Dimension::Exact(measurement) => Some(measurement),
            Dimension::Percent(percent) => Some(content_length * percent),
        }
    }
}
