use std::{fmt::Debug, marker::PhantomData};

use gooey_core::{
    styles::Surround, Frontend, Key, KeyedStorage, Points, RelatedStorage, StyledWidget, Widget,
    WidgetRef, WidgetRegistration, WidgetStorage,
};

use crate::component::{Behavior, ComponentBuilder, Content, ContentBuilder};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug)]
pub struct Container {
    pub child: WidgetRegistration,
}

impl From<WidgetRegistration> for Container {
    fn from(child: WidgetRegistration) -> Self {
        Self { child }
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

impl<B: Behavior> Content<B> for Container {
    type Builder = Builder<B::Widgets, ComponentBuilder<B>>;
}

#[derive(Debug)]
pub struct Builder<K: Key, S: KeyedStorage<K>> {
    storage: S,
    child: Option<WidgetRegistration>,
    padding: Surround<Points>,
    _phantom: PhantomData<K>,
}

impl<K: Key, S: KeyedStorage<K>> Builder<K, S> {
    pub fn child<W: Widget>(mut self, key: impl Into<Option<K>>, widget: StyledWidget<W>) -> Self {
        self.child = Some(self.storage.register(key.into(), widget));
        self
    }

    pub fn finish(self) -> StyledWidget<Container> {
        StyledWidget::from(Container {
            child: self.child.expect("no child in container"),
        })
    }
}

impl<K: Key, S: KeyedStorage<K>> ContentBuilder<K, S> for Builder<K, S> {
    fn storage(&self) -> &WidgetStorage {
        self.storage.storage()
    }

    fn related_storage(&self) -> Option<Box<dyn RelatedStorage<K>>> {
        self.storage.related_storage()
    }

    fn new(storage: S) -> Self {
        Self {
            storage,
            child: None,
            padding: Surround::default(),
            _phantom: PhantomData::default(),
        }
    }
}

#[derive(Debug)]
pub struct ContainerTransmogrifier;
