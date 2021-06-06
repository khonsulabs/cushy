use std::{collections::HashMap, fmt::Debug, hash::Hash};

use gooey_core::{
    euclid::Length, AnySendSync, Points, StyledWidget, Widget, WidgetRegistration, WidgetStorage,
};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug)]
pub struct CustomLayout {
    children: Box<dyn LayoutChildren>,
}

impl CustomLayout {
    #[must_use]
    pub fn build<K: LayoutKey>(storage: &WidgetStorage) -> Builder<K> {
        Builder::new(storage)
    }
}

impl Widget for CustomLayout {
    type Command = ();
    type TransmogrifierCommand = ();
    type TransmogrifierEvent = ();

    const CLASS: &'static str = "gooey-layout";
}

#[derive(Debug)]
pub struct Builder<K: LayoutKey> {
    storage: WidgetStorage,
    children: ChildrenMap<K>,
}

type ChildrenMap<K> = HashMap<K, LayoutChild>;

#[derive(Clone, Debug)]
pub struct LayoutChild {
    pub registration: WidgetRegistration,
    pub layout: Layout,
}

impl<K: LayoutKey> Builder<K> {
    pub fn new(storage: &WidgetStorage) -> Self {
        Self {
            storage: storage.clone(),
            children: HashMap::default(),
        }
    }

    pub fn with<W: Widget>(self, key: K, widget: StyledWidget<W>, layout: Layout) -> Self {
        let widget = self.storage.register(widget);
        self.with_registration(key, widget, layout)
    }

    pub fn with_registration(
        mut self,
        key: K,
        registration: WidgetRegistration,
        layout: Layout,
    ) -> Self {
        self.children.insert(
            key,
            LayoutChild {
                registration,
                layout,
            },
        );
        self
    }

    pub fn finish(self) -> StyledWidget<CustomLayout> {
        StyledWidget::default_for(CustomLayout {
            children: Box::new(self.children),
        })
    }
}

pub trait LayoutKey: Hash + Debug + Eq + PartialEq + Send + Sync + 'static {}

impl<T> LayoutKey for T where T: Hash + Debug + Eq + PartialEq + Send + Sync + 'static {}

#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub left: Dimension,
    pub top: Dimension,
    pub right: Dimension,
    pub bottom: Dimension,
    pub width: Dimension,
    pub height: Dimension,
}

impl Layout {
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
}

pub trait LayoutChildren: AnySendSync {
    fn layout_children(&self) -> Vec<LayoutChild>;
}

#[derive(Debug)]
pub struct CustomLayoutTransmogrifier;

impl<K: LayoutKey> LayoutChildren for ChildrenMap<K> {
    fn layout_children(&self) -> Vec<LayoutChild> {
        self.values().cloned().collect()
    }
}

impl LayoutChildren for CustomLayout {
    fn layout_children(&self) -> Vec<LayoutChild> {
        self.children.layout_children()
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
        Dimension::Auto
    }
}
