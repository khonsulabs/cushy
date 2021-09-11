use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use gooey_core::{
    styles::{style_sheet::Classes, Style},
    AnyWidget, Builder, Callback, CallbackFn, Channels, Context, DefaultWidget, Frontend, Key,
    KeyedStorage, LocalizationParameters, RelatedStorage, StyledWidget, Transmogrifier,
    TransmogrifierContext, WeakWidgetRegistration, Widget, WidgetId, WidgetRef, WidgetRegistration,
    WidgetState, WidgetStorage,
};
use parking_lot::{Mutex, RwLock};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug)]
pub struct Component<B: Behavior> {
    pub behavior: B,
    content: WidgetRegistration,
    content_widget: Option<WidgetRef<B::Content>>,
    registered_widgets: Arc<Mutex<HashMap<B::Widgets, WeakWidgetRegistration>>>,
    callback_widget: SettableWidgetRef<B>,
}

impl<B: Behavior + Default> DefaultWidget for Component<B> {
    fn default_for(storage: &WidgetStorage) -> StyledWidget<Self> {
        Self::new(B::default(), storage)
    }
}

impl<B: Behavior> Component<B> {
    pub fn new(mut behavior: B, storage: &WidgetStorage) -> StyledWidget<Self> {
        let own_registration = storage.allocate::<Self>();
        let registered_widgets =
            Arc::<Mutex<HashMap<B::Widgets, WeakWidgetRegistration>>>::default();
        let builder =
            ComponentBuilder::<B>::new(registered_widgets.clone(), storage, &own_registration);
        let content_builder = <B::Content as Content<B>>::build(builder);
        let event_mapper = EventMapper::default();
        let content = behavior.build_content(content_builder, &event_mapper);
        let content = storage.register(content);
        StyledWidget::new(
            Self {
                content,
                behavior,
                callback_widget: event_mapper.widget,
                registered_widgets,
                content_widget: None,
            },
            Style::default(),
            Some(own_registration),
        )
    }

    pub fn content(&self) -> Option<&'_ WidgetRef<B::Content>> {
        self.content_widget.as_ref()
    }

    pub fn registered_widget(&self, id: &B::Widgets) -> Option<WidgetRegistration> {
        let registered_widgets = self.registered_widgets.lock();
        registered_widgets
            .get(id)
            .and_then(WeakWidgetRegistration::upgrade)
    }

    pub fn register_widget(&mut self, id: B::Widgets, registration: &WidgetRegistration) {
        let mut registered_widgets = self.registered_widgets.lock();
        registered_widgets.insert(id, WeakWidgetRegistration::from(registration));
    }

    pub fn remove(&mut self, id: &B::Widgets) {
        let mut registered_widgets = self.registered_widgets.lock();
        registered_widgets.remove(id);
    }

    pub fn map_content<F: FnOnce(&B::Content, &Context<B::Content>) -> R, R>(
        &self,
        context: &Context<Self>,
        with_fn: F,
    ) -> Option<R> {
        context.map_widget(self.content.id(), with_fn)
    }

    pub fn map_content_mut<F: FnOnce(&mut B::Content, &Context<B::Content>) -> R, R>(
        &self,
        context: &Context<Self>,
        with_fn: F,
    ) -> Option<R> {
        context.map_widget_mut(self.content.id(), with_fn)
    }

    pub fn map_widget<OW: Widget, F: FnOnce(&OW, &Context<OW>) -> R, R>(
        &self,
        id: &B::Widgets,
        context: &Context<Self>,
        with_fn: F,
    ) -> Option<R> {
        self.registered_widget(id)
            .and_then(|widget| context.map_widget(widget.id(), with_fn))
    }

    pub fn map_widget_mut<OW: Widget, F: FnOnce(&mut OW, &Context<OW>) -> R, R>(
        &self,
        id: &B::Widgets,
        context: &Context<Self>,
        with_fn: F,
    ) -> Option<R> {
        self.registered_widget(id)
            .and_then(|widget| context.map_widget_mut(widget.id(), with_fn))
    }

    pub fn widget_state(&self, id: &B::Widgets, context: &Context<Self>) -> Option<WidgetState> {
        self.registered_widget(id)
            .and_then(|widget| context.widget_state(widget.id()))
    }

    pub fn map_event<I: 'static, C: CallbackFn<I, <B as Behavior>::Event> + 'static>(
        &self,
        mapper: C,
    ) -> Callback<I, ()> {
        let mapped_callback = MappedCallback::<B, I> {
            mapper: Box::new(mapper),
            widget: self.callback_widget.clone(),
            _phantom: PhantomData::default(),
        };
        Callback::new(mapped_callback)
    }

    pub fn event_mapper(&self) -> EventMapper<B> {
        EventMapper {
            widget: self.callback_widget.clone(),
        }
    }
}

#[derive(Debug)]
pub struct ComponentTransmogrifier<B: Behavior>(PhantomData<B>);

impl<B: Behavior> Default for ComponentTransmogrifier<B> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

pub trait Behavior: Debug + Send + Sync + Sized + 'static {
    type Event: Debug + Send + Sync;
    type Content: Content<Self>;
    type Widgets: Key;

    #[must_use]
    fn classes() -> Option<Classes> {
        None
    }

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        events: &EventMapper<Self>,
    ) -> StyledWidget<Self::Content>;

    #[allow(unused_variables)]
    fn initialize(component: &mut Component<Self>, context: &Context<Component<Self>>) {}

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    );
}

pub trait Content<B: Behavior>: Widget {
    type Builder: ContentBuilder<Self, B::Widgets, ComponentBuilder<B>>;

    #[must_use]
    fn build(builder: ComponentBuilder<B>) -> Self::Builder {
        Self::Builder::new(builder)
    }
}

pub trait ContentBuilder<W: Widget, K: Key, S: KeyedStorage<K> + 'static>:
    Builder<Output = StyledWidget<W>> + Debug + Send + Sync
{
    fn storage(&self) -> &WidgetStorage;
    fn related_storage(&self) -> Option<Box<dyn RelatedStorage<K>>>;
    #[must_use]
    fn new(storage: S) -> Self;

    /// Localizes `key` with `parameters`.
    #[must_use]
    fn localize<'a>(
        &self,
        key: &str,
        parameters: impl Into<Option<LocalizationParameters<'a>>>,
    ) -> String {
        self.storage().localize(key, parameters)
    }
}

#[derive(Debug)]
pub enum ComponentCommand<W, B> {
    Widget(W),
    Behavior(B),
}

impl<B: Behavior> Widget for Component<B> {
    type Command = ComponentCommand<<B::Content as Widget>::Command, B::Event>;
    type Event = InternalEvent<B>;

    const CLASS: &'static str = "gooey-component";
    const FOCUSABLE: bool = false;

    fn classes() -> Classes {
        B::classes().map_or_else(
            || Classes::from(Self::CLASS),
            |mut classes| {
                classes.insert(Cow::from(Self::CLASS));
                classes
            },
        )
    }

    fn receive_event(&mut self, event: Self::Event, context: &Context<Self>) {
        match event {
            InternalEvent::ReceiveWidget(widget) => {
                self.content_widget = Some(widget);
            }
            InternalEvent::Content(event) => B::receive_event(self, event, context),
        }
    }
}

impl<B: Behavior> Deref for Component<B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        &self.behavior
    }
}

impl<B: Behavior> DerefMut for Component<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.behavior
    }
}

#[derive(Debug)]
pub enum InternalEvent<B: Behavior> {
    ReceiveWidget(WidgetRef<B::Content>),
    Content(B::Event),
}

#[derive(Debug)]
pub struct ComponentBuilder<B: Behavior> {
    component: WidgetRegistration,
    storage: WidgetStorage,
    registered_widgets: Arc<Mutex<HashMap<B::Widgets, WeakWidgetRegistration>>>,
    _phantom: PhantomData<B>,
}

#[derive(Debug)]
pub struct EventMapper<B: Behavior> {
    widget: SettableWidgetRef<B>,
}

#[derive(Debug)]
pub struct ComponentUpdater<B: Behavior> {
    component: WeakWidgetRegistration,
    registered_widgets: Arc<Mutex<HashMap<B::Widgets, WeakWidgetRegistration>>>,
    _behavior: PhantomData<B>,
}

impl<B: Behavior> RelatedStorage<B::Widgets> for ComponentUpdater<B> {
    fn widget(&self) -> WeakWidgetRegistration {
        self.component.clone()
    }

    fn remove(&self, key: &B::Widgets) -> Option<WeakWidgetRegistration> {
        let mut registered_widgets = self.registered_widgets.lock();
        registered_widgets.remove(key)
    }

    fn register(&self, key: B::Widgets, registration: &WidgetRegistration) {
        let mut registered_widgets = self.registered_widgets.lock();
        registered_widgets.insert(key, WeakWidgetRegistration::from(registration));
    }
}

impl<B: Behavior> Default for EventMapper<B> {
    fn default() -> Self {
        Self {
            widget: SettableWidgetRef::default(),
        }
    }
}

impl<B: Behavior> EventMapper<B> {
    pub fn map<I: 'static, C: CallbackFn<I, <B as Behavior>::Event> + 'static>(
        &self,
        mapper: C,
    ) -> Callback<I, ()> {
        let mapped_callback = MappedCallback::<B, I> {
            mapper: Box::new(mapper),
            widget: self.widget.clone(),
            _phantom: PhantomData::default(),
        };
        Callback::new(mapped_callback)
    }
}

impl<B: Behavior> ComponentBuilder<B> {
    #[must_use]
    pub fn new(
        registered_widgets: Arc<Mutex<HashMap<B::Widgets, WeakWidgetRegistration>>>,
        storage: &WidgetStorage,
        component: &WidgetRegistration,
    ) -> Self {
        Self {
            registered_widgets,
            storage: storage.clone(),
            component: component.clone(),
            _phantom: PhantomData::default(),
        }
    }

    /// Register a widget with storage.
    pub fn register<W: Widget + AnyWidget>(
        &mut self,
        id: B::Widgets,
        widget: StyledWidget<W>,
    ) -> WidgetRegistration {
        let registration = self.storage.register(widget);
        let mut registered_widgets = self.registered_widgets.lock();
        registered_widgets.insert(id, WeakWidgetRegistration::from(&registration));
        registration
    }

    pub fn component(&self) -> &WidgetRegistration {
        &self.component
    }
}

impl<B: Behavior> KeyedStorage<B::Widgets> for ComponentBuilder<B> {
    fn register<W: Widget + AnyWidget>(
        &mut self,
        key: impl Into<Option<B::Widgets>>,
        styled_widget: StyledWidget<W>,
    ) -> WidgetRegistration {
        match key.into() {
            Some(key) => Self::register(self, key, styled_widget),
            None => self.storage.register(styled_widget),
        }
    }

    fn storage(&self) -> &WidgetStorage {
        &self.storage
    }

    fn related_storage(&self) -> Option<Box<dyn RelatedStorage<B::Widgets>>> {
        Some(Box::new(ComponentUpdater::<B> {
            component: WeakWidgetRegistration::from(&self.component),
            registered_widgets: self.registered_widgets.clone(),
            _behavior: PhantomData::default(),
        }))
    }
}

impl<B: Behavior> Deref for ComponentBuilder<B> {
    type Target = WidgetStorage;

    fn deref(&self) -> &Self::Target {
        &self.storage
    }
}

pub struct MappedCallback<B: Behavior, I> {
    widget: SettableWidgetRef<B>,
    mapper: Box<dyn CallbackFn<I, <B as Behavior>::Event>>,
    _phantom: PhantomData<B>,
}

type SettableWidgetRef<B> = Arc<RwLock<Option<Box<dyn AnyEventPoster<B>>>>>;

#[derive(Debug)]
pub struct EventPoster<B: Behavior, F: Frontend> {
    widget: WidgetId,
    channels: Channels<Component<B>>,
    frontend: F,
}

impl<B: Behavior, F: Frontend> AnyEventPoster<B> for EventPoster<B, F> {
    fn post_event(&self, event: B::Event) {
        self.channels.post_event(InternalEvent::Content(event));
        self.frontend.set_widget_has_messages(self.widget.clone());
        if !self.frontend.gooey().is_managed_code() {
            self.frontend
                .gooey()
                .process_widget_messages(&self.frontend);
        }
    }
}

pub trait AnyEventPoster<B: Behavior>: Debug + Send + Sync + 'static {
    fn post_event(&self, event: B::Event);
}

impl<B: Behavior, I> Debug for MappedCallback<B, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "MappedCallback {{ widget: {:?} }}",
            &self.widget
        ))
    }
}

impl<B: Behavior, I> CallbackFn<I, ()> for MappedCallback<B, I> {
    fn invoke(&self, info: I) {
        let poster = self.widget.read();
        let poster = poster.as_ref().unwrap();
        poster.post_event(self.mapper.invoke(info));
    }
}

impl<B: Behavior> ComponentTransmogrifier<B> {
    pub fn initialize_component<F: Frontend>(
        component: &mut Component<B>,
        widget: &WidgetRef<Component<B>>,
        frontend: &F,
    ) {
        let widget = widget.registration().unwrap().id().clone();
        let widget_state = frontend.gooey().widget_state(&widget).unwrap();
        let channels = widget_state.channels::<Component<B>>().unwrap();
        B::initialize(component, &Context::new(channels, frontend));

        let mut callback_widget = component.callback_widget.write();
        *callback_widget = Some(Box::new(EventPoster {
            widget,
            channels: channels.clone(),
            frontend: frontend.clone(),
        }));
        channels.post_event(InternalEvent::ReceiveWidget(
            WidgetRef::new(&component.content, frontend.clone()).expect(
                "type mismatch: Behavior::Widget type doesn't match initialized widget type",
            ),
        ));
    }

    pub fn forward_command_to_content<F: Frontend>(
        command: <Component<B> as Widget>::Command,
        context: &mut TransmogrifierContext<'_, Self, F>,
    ) where
        Self: Transmogrifier<F, Widget = Component<B>>,
    {
        match command {
            ComponentCommand::Widget(command) => {
                context
                    .widget
                    .content_widget
                    .as_ref()
                    .unwrap()
                    .post_command::<F>(command);
            }
            ComponentCommand::Behavior(event) => context
                .widget
                .receive_event(InternalEvent::Content(event), &Context::from(&*context)),
        }
    }
}
