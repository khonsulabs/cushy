use std::{collections::HashMap, fmt::Debug, ops::Deref};

use gooey_core::{
    figures::{Figure, Size},
    styles::Surround,
    AnySendSync, Context, Key, KeyedStorage, RelatedStorage, Scaled, StyledWidget, Widget,
    WidgetId, WidgetRegistration, WidgetStorage,
};

use crate::component::{Behavior, ComponentBuilder, Content, ContentBuilder};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug)]
pub struct Layout {
    children: Box<dyn LayoutChildren>,
    related_storage: Option<Box<dyn AnySendSync>>,
}

impl Layout {
    #[must_use]
    pub fn build<K: Key>(storage: &WidgetStorage) -> Builder<K, WidgetStorage> {
        Builder::new(storage.clone())
    }

    pub fn remove_child<K: Key>(&mut self, layout_key: &K, context: &Context<Self>) -> bool {
        let children = self
            .children
            .as_mut()
            .as_mut_any()
            .downcast_mut::<ChildrenMap<K>>()
            .unwrap();
        children.remove(layout_key).map_or(false, |removed_child| {
            if let Some(related_storage) = &mut self.related_storage {
                let related_storage = related_storage
                    .as_mut()
                    .as_mut_any()
                    .downcast_mut::<Box<dyn RelatedStorage<K>>>()
                    .unwrap();
                related_storage.remove(layout_key);
            }

            context.send_command(LayoutCommand::ChildRemoved(
                removed_child.registration.id().clone(),
            ));
            true
        })
    }

    pub fn insert<K: Key, W: Widget>(
        &mut self,
        layout_key: Option<K>,
        widget: StyledWidget<W>,
        layout: WidgetLayout,
        context: &Context<Self>,
    ) {
        let registration = context.register(widget);
        self.insert_registration(layout_key, registration, layout, context);
    }

    pub fn insert_registration<K: Key>(
        &mut self,
        layout_key: Option<K>,
        registration: WidgetRegistration,
        layout: WidgetLayout,
        context: &Context<Self>,
    ) {
        let children = self
            .children
            .as_mut()
            .as_mut_any()
            .downcast_mut::<ChildrenMap<K>>()
            .unwrap();
        if let Some(layout_key) = &layout_key {
            if let Some(related_storage) = &mut self.related_storage {
                let related_storage = related_storage
                    .as_mut()
                    .as_mut_any()
                    .downcast_mut::<Box<dyn RelatedStorage<K>>>()
                    .unwrap();
                related_storage.register(layout_key.clone(), &registration);
            }
        }
        if let Some(old_child) = children.insert(
            layout_key,
            LayoutChild {
                registration: registration.clone(),
                layout,
            },
        ) {
            context.send_command(LayoutCommand::ChildRemoved(
                old_child.registration.id().clone(),
            ));
        }

        context.send_command(LayoutCommand::ChildAdded(registration));
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
    const FOCUSABLE: bool = false;
}

#[derive(Debug)]
pub struct Builder<K: Key, S: KeyedStorage<K>> {
    storage: S,
    children: ChildrenMap<K>,
}

#[derive(Debug)]
struct ChildrenMap<K> {
    children: HashMap<u32, LayoutChild>,
    keys_to_id: HashMap<K, u32>,
    order: Vec<u32>,
}

impl<K> Default for ChildrenMap<K> {
    fn default() -> Self {
        Self {
            children: HashMap::default(),
            keys_to_id: HashMap::default(),
            order: Vec::default(),
        }
    }
}

impl<K: Key> ChildrenMap<K> {
    fn remove(&mut self, key: &K) -> Option<LayoutChild> {
        self.keys_to_id.remove(key).and_then(|id| {
            self.order.retain(|order| !order == id);
            self.children.remove(&id)
        })
    }

    fn insert(&mut self, key: Option<K>, child: LayoutChild) -> Option<LayoutChild> {
        let mut old_child = None;
        if let Some(key) = key {
            if let Some(removed_widget) = self.keys_to_id.insert(key, child.registration.id().id) {
                old_child = self.children.remove(&removed_widget);
                if let Some(old_child) = &old_child {
                    self.order
                        .retain(|id| *id != old_child.registration.id().id);
                }
            }
        }
        self.order.push(child.registration.id().id);
        self.children.insert(child.registration.id().id, child);
        old_child
    }
}

#[derive(Clone, Debug)]
pub struct LayoutChild {
    pub registration: WidgetRegistration,
    pub layout: WidgetLayout,
}

impl Deref for LayoutChild {
    type Target = WidgetLayout;

    fn deref(&self) -> &WidgetLayout {
        &self.layout
    }
}

impl<K: Key, S: KeyedStorage<K>> Builder<K, S> {
    pub fn storage(&self) -> &WidgetStorage {
        self.storage.storage()
    }

    pub fn with<W: Widget>(
        mut self,
        key: impl Into<Option<K>>,
        widget: StyledWidget<W>,
        layout: WidgetLayout,
    ) -> Self {
        let key = key.into();
        let widget = self.storage.register(key.clone(), widget);
        self.with_registration(key, widget, layout)
    }

    pub fn with_registration(
        mut self,
        key: impl Into<Option<K>>,
        registration: WidgetRegistration,
        layout: WidgetLayout,
    ) -> Self {
        self.children.insert(
            key.into(),
            LayoutChild {
                registration,
                layout,
            },
        );
        self
    }

    pub fn finish(self) -> StyledWidget<Layout> {
        StyledWidget::from(Layout {
            children: Box::new(self.children),
            related_storage: self
                .storage
                .related_storage()
                .map(|storage| Box::new(storage) as Box<dyn AnySendSync>),
        })
    }
}

#[derive(Clone, Debug, Default)]
#[must_use]
pub struct WidgetLayout {
    pub left: Dimension,
    pub top: Dimension,
    pub right: Dimension,
    pub bottom: Dimension,
    pub width: Dimension,
    pub height: Dimension,
}

#[derive(Clone, Debug, Default)]
#[must_use]
pub struct WidgetLayoutBuilder {
    layout: WidgetLayout,
}

impl WidgetLayout {
    pub fn build() -> WidgetLayoutBuilder {
        WidgetLayoutBuilder::default()
    }

    pub fn fill() -> Self {
        Self {
            left: Dimension::zero(),
            top: Dimension::zero(),
            right: Dimension::zero(),
            bottom: Dimension::zero(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn left_in_points(&self, content_size: Size<f32, Scaled>) -> Option<Figure<f32, Scaled>> {
        self.left.length(Figure::new(content_size.width))
    }

    #[must_use]
    pub fn right_in_points(&self, content_size: Size<f32, Scaled>) -> Option<Figure<f32, Scaled>> {
        self.right.length(Figure::new(content_size.width))
    }

    #[must_use]
    pub fn top_in_points(&self, content_size: Size<f32, Scaled>) -> Option<Figure<f32, Scaled>> {
        self.top.length(Figure::new(content_size.height))
    }

    #[must_use]
    pub fn bottom_in_points(&self, content_size: Size<f32, Scaled>) -> Option<Figure<f32, Scaled>> {
        self.bottom.length(Figure::new(content_size.height))
    }

    #[must_use]
    pub fn width_in_points(&self, content_size: Size<f32, Scaled>) -> Figure<f32, Scaled> {
        self.width
            .length(Figure::new(content_size.width))
            .unwrap_or_else(|| Figure::new(content_size.width))
    }

    #[must_use]
    pub fn height_in_points(&self, content_size: Size<f32, Scaled>) -> Figure<f32, Scaled> {
        self.height
            .length(Figure::new(content_size.height))
            .unwrap_or_else(|| Figure::new(content_size.height))
    }

    #[must_use]
    pub fn surround_in_points(
        &self,
        content_size: Size<f32, Scaled>,
    ) -> Surround<Figure<f32, Scaled>> {
        Surround {
            left: self.left_in_points(content_size),
            top: self.top_in_points(content_size),
            right: self.right_in_points(content_size),
            bottom: self.bottom_in_points(content_size),
        }
    }

    #[must_use]
    pub fn size_in_points(&self, content_size: Size<f32, Scaled>) -> Size<f32, Scaled> {
        Size::from_figures(
            self.width_in_points(content_size),
            self.height_in_points(content_size),
        )
    }
}

impl WidgetLayoutBuilder {
    pub fn left<D: Into<Dimension>>(mut self, left: D) -> Self {
        self.layout.left = left.into();
        self
    }

    pub fn right<D: Into<Dimension>>(mut self, right: D) -> Self {
        self.layout.right = right.into();
        self
    }

    pub fn top<D: Into<Dimension>>(mut self, top: D) -> Self {
        self.layout.top = top.into();
        self
    }

    pub fn bottom<D: Into<Dimension>>(mut self, bottom: D) -> Self {
        self.layout.bottom = bottom.into();
        self
    }

    pub fn width<D: Into<Dimension>>(mut self, width: D) -> Self {
        self.layout.width = width.into();
        self
    }

    pub fn height<D: Into<Dimension>>(mut self, height: D) -> Self {
        self.layout.height = height.into();
        self
    }

    pub const fn fill_width(mut self) -> Self {
        self.layout.left = Dimension::zero();
        self.layout.right = Dimension::zero();
        self
    }

    pub fn horizontal_margins<D: Into<Dimension>>(mut self, margin: D) -> Self {
        let margin = margin.into();
        self.layout.left = margin;
        self.layout.right = margin;
        self
    }

    pub fn vertical_margins<D: Into<Dimension>>(mut self, margin: D) -> Self {
        let margin = margin.into();
        self.layout.top = margin;
        self.layout.bottom = margin;
        self
    }

    pub const fn fill_height(mut self) -> Self {
        self.layout.top = Dimension::zero();
        self.layout.bottom = Dimension::zero();
        self
    }

    pub const fn finish(self) -> WidgetLayout {
        self.layout
    }
}

pub trait LayoutChildren: AnySendSync {
    fn layout_children(&self) -> Vec<LayoutChild>;
    fn child_by_widget_id(&self, widget_id: &WidgetId) -> Option<&LayoutChild>;
}

#[derive(Debug)]
pub struct LayoutTransmogrifier;

impl<K: Key> LayoutChildren for ChildrenMap<K> {
    fn layout_children(&self) -> Vec<LayoutChild> {
        self.order
            .iter()
            .map(|child_id| self.children.get(child_id).unwrap().clone())
            .collect()
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
    Exact(Figure<f32, Scaled>),
    Percent(f32),
}

impl Default for Dimension {
    fn default() -> Self {
        Self::Auto
    }
}

impl Dimension {
    #[must_use]
    pub const fn zero() -> Self {
        Self::Exact(Figure::new(0.))
    }

    #[must_use]
    pub const fn exact(length: f32) -> Self {
        Self::Exact(Figure::new(length))
    }

    #[must_use]
    pub const fn percent(percent: f32) -> Self {
        Self::Percent(percent)
    }

    #[must_use]
    pub fn length(self, content_length: Figure<f32, Scaled>) -> Option<Figure<f32, Scaled>> {
        match self {
            Dimension::Auto => None,
            Dimension::Exact(measurement) => Some(measurement),
            Dimension::Percent(percent) => Some(content_length * percent),
        }
    }
}

impl From<Figure<f32, Scaled>> for Dimension {
    fn from(length: Figure<f32, Scaled>) -> Self {
        Self::Exact(length)
    }
}

impl From<f32> for Dimension {
    fn from(f: f32) -> Self {
        Figure::new(f).into()
    }
}

#[allow(clippy::cast_precision_loss)]
impl From<i32> for Dimension {
    fn from(i: i32) -> Self {
        Figure::new(i as f32).into()
    }
}

impl<B: Behavior> Content<B> for Layout {
    type Builder = Builder<B::Widgets, ComponentBuilder<B>>;

    fn build(storage: ComponentBuilder<B>) -> Self::Builder {
        Builder::new(storage)
    }
}

impl<'a, K: Key, S: KeyedStorage<K>> ContentBuilder<Layout, K, S> for Builder<K, S> {
    fn new(storage: S) -> Self {
        Self {
            storage,
            children: ChildrenMap::default(),
        }
    }

    fn storage(&self) -> &WidgetStorage {
        self.storage.storage()
    }

    fn related_storage(&self) -> Option<Box<dyn RelatedStorage<K>>> {
        self.storage.related_storage()
    }
}

impl<K: Key, S: KeyedStorage<K>> gooey_core::Builder for Builder<K, S> {
    type Output = StyledWidget<Layout>;

    fn finish(self) -> Self::Output {
        Builder::finish(self)
    }
}
