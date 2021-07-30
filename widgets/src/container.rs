use std::fmt::Debug;

use gooey_core::{
    euclid::Length, styles::Surround, Frontend, Key, KeyedStorage, KeyedWidgetStorage, Points,
    StyledWidget, WeakWidgetRegistration, Widget, WidgetRef, WidgetRegistration, WidgetStorage,
};

use crate::component::{Behavior, ComponentBuilder, Content, ContentBuilder};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug)]
pub struct Container {
    pub child: WidgetRegistration,
    pub padding: Surround<Points>,
}

impl From<WidgetRegistration> for Container {
    fn from(child: WidgetRegistration) -> Self {
        Self {
            child,
            padding: Surround::default(),
        }
    }
}

impl Container {
    pub fn new<W: Widget>(child: StyledWidget<W>, storage: &WidgetStorage) -> StyledWidget<Self> {
        StyledWidget::from(storage.register(child))
    }

    pub fn child<W: Widget, F: Frontend>(&self, frontend: F) -> Option<WidgetRef<W>> {
        WidgetRef::new(&self.child, frontend)
    }
}

impl Widget for Container {
    type Command = ();
    type Event = ();

    const CLASS: &'static str = "gooey-container";
}

impl<'a, B: Behavior> Content<'a, B> for Container {
    type Builder = Builder<'a, B::Widgets, B::Event, ComponentBuilder<B>>;
}

#[derive(Debug)]
pub struct Builder<'a, K: Key, E, S: KeyedStorage<K, E>> {
    storage: KeyedWidgetStorage<'a, K, E, S>,
    child: Option<WidgetRegistration>,
    padding: Surround<Points>,
}

impl<'a, K: Key, E, S: KeyedStorage<K, E>> Builder<'a, K, E, S> {
    pub fn child<W: Widget>(mut self, key: impl Into<Option<K>>, widget: StyledWidget<W>) -> Self {
        self.child = Some(self.storage.register(key.into(), widget));
        self
    }

    pub fn pad_left<F: Into<Length<f32, Points>>>(mut self, padding: F) -> Self {
        self.padding.left = Some(padding.into());
        self
    }

    pub fn pad_right<F: Into<Length<f32, Points>>>(mut self, padding: F) -> Self {
        self.padding.right = Some(padding.into());
        self
    }

    pub fn pad_top<F: Into<Length<f32, Points>>>(mut self, padding: F) -> Self {
        self.padding.top = Some(padding.into());
        self
    }

    pub fn pad_bottom<F: Into<Length<f32, Points>>>(mut self, padding: F) -> Self {
        self.padding.bottom = Some(padding.into());
        self
    }

    pub fn finish(self) -> StyledWidget<Container> {
        StyledWidget::from(Container {
            child: self.child.expect("no child in container"),
            padding: self.padding,
        })
    }
}

impl<'a, K: Key, E: Debug + Send + Sync, S: KeyedStorage<K, E> + 'static>
    ContentBuilder<'a, K, E, S> for Builder<'a, K, E, S>
{
    fn storage(&self) -> &WidgetStorage {
        self.storage.storage()
    }

    fn component(&self) -> Option<WeakWidgetRegistration> {
        self.storage.component()
    }

    fn new(storage: impl Into<KeyedWidgetStorage<'a, K, E, S>>) -> Self {
        Self {
            storage: storage.into(),
            child: None,
            padding: Surround::default(),
        }
    }
}

#[derive(Debug)]
pub struct ContainerTransmogrifier;
