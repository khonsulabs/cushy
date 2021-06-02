use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, RwLock},
};

use gooey_core::{
    AnyWidget, Callback, CallbackFn, Channels, Context, Frontend, StyledWidget, Transmogrifier,
    TransmogrifierContext, WeakWidgetRegistration, Widget, WidgetId, WidgetRef, WidgetRegistration,
    WidgetStorage,
};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug)]
pub struct Component<B: Behavior> {
    pub behavior: B,
    content: WidgetRegistration,
    content_widget: Option<WidgetRef<B::Content>>,
    registered_widgets: HashMap<B::Widgets, WeakWidgetRegistration>,
    callback_widget: SettableWidgetRef<B>,
}

impl<B: Behavior> Component<B> {
    pub fn new(mut behavior: B, storage: &WidgetStorage) -> StyledWidget<Self> {
        let mut builder = ComponentBuilder::new(storage);
        let content = behavior.create_content(&mut builder);
        let content = builder.register(content);
        StyledWidget::default_for(Component {
            content,
            behavior,
            callback_widget: builder.widget,
            registered_widgets: builder.registered_widgets,
            content_widget: None,
        })
    }

    pub fn default_for(storage: &WidgetStorage) -> StyledWidget<Self>
    where
        B: Default,
    {
        Self::new(B::default(), storage)
    }

    pub fn content(&self) -> Option<&'_ WidgetRef<B::Content>> {
        self.content_widget.as_ref()
    }

    pub fn registered_widget(&self, id: &B::Widgets) -> Option<WidgetRegistration> {
        self.registered_widgets.get(id).and_then(|id| id.upgrade())
    }

    pub fn send_command_to<W: Widget>(
        &self,
        id: &B::Widgets,
        command: W::Command,
        storage: &WidgetStorage,
    ) -> bool {
        if let Some(widget) = self.registered_widget(id) {
            if let Some(state) = storage.widget_state(widget.id().id) {
                let channels = state.channels::<W>().expect("incorrect widget type");
                channels.post_command(command);
                return true;
            }
        }
        false
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
    type Content: Widget;
    type Widgets: Hash + Eq + Debug + Send + Sync;

    fn create_content(
        &mut self,
        builder: &mut ComponentBuilder<Self>,
    ) -> StyledWidget<Self::Content>;

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    );
}

impl<B: Behavior> Widget for Component<B> {
    type Command = <B::Content as Widget>::Command;
    type TransmogrifierCommand = <B::Content as Widget>::Command;
    type TransmogrifierEvent = InternalEvent<B>;

    const CLASS: &'static str = "gooey-component";

    fn receive_event(&mut self, event: Self::TransmogrifierEvent, context: &Context<Self>) {
        match event {
            InternalEvent::ReceiveWidget(widget) => {
                self.content_widget = Some(widget);
            }
            InternalEvent::Content(event) => B::receive_event(self, event, context),
        }
    }

    fn receive_command(&mut self, command: Self::Command, context: &Context<Self>) {
        context.send_command(command);
    }
}

#[derive(Debug)]
pub enum InternalEvent<B: Behavior> {
    ReceiveWidget(WidgetRef<B::Content>),
    Content(B::Event),
}

#[derive(Debug)]
pub struct ComponentBuilder<B: Behavior> {
    widget: SettableWidgetRef<B>,
    storage: WidgetStorage,
    registered_widgets: HashMap<B::Widgets, WeakWidgetRegistration>,
    _phantom: PhantomData<B>,
}

impl<B: Behavior> ComponentBuilder<B> {
    pub fn new(storage: &WidgetStorage) -> Self {
        Self {
            storage: storage.clone(),
            widget: SettableWidgetRef::default(),
            registered_widgets: HashMap::default(),
            _phantom: PhantomData::default(),
        }
    }

    pub fn map_event<I: 'static, C: CallbackFn<I, <B as Behavior>::Event> + 'static>(
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

    /// Register a widget with storage.
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // The unwrap is unreachable
    pub fn register_widget<W: Widget + AnyWidget>(
        &mut self,
        id: B::Widgets,
        widget: StyledWidget<W>,
    ) -> WidgetRegistration {
        let registration = self.storage.register(widget);
        self.registered_widgets
            .insert(id, WeakWidgetRegistration::from(&registration));
        registration
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
        let _ = self.channels.post_event(InternalEvent::Content(event));
        self.frontend
            .gooey()
            .set_widget_has_messages(self.widget.clone());
        self.frontend
            .gooey()
            .process_widget_messages(&self.frontend);
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
        let poster = self.widget.read().unwrap();
        let poster = poster.as_ref().unwrap();
        poster.post_event(self.mapper.invoke(info));
    }
}

impl<B: Behavior> ComponentTransmogrifier<B> {
    pub fn initialize_component<F: Frontend>(
        &self,
        component: &Component<B>,
        widget: &WidgetRef<Component<B>>,
        frontend: &F,
    ) {
        let widget = widget.registration().unwrap().id().clone();
        let widget_state = frontend.gooey().widget_state(widget.id).unwrap();
        let channels = widget_state.channels::<Component<B>>().unwrap();
        let mut callback_widget = component.callback_widget.write().unwrap();
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
        &self,
        command: <Component<B> as Widget>::TransmogrifierCommand,
        context: &mut TransmogrifierContext<Self, F>,
    ) where
        Self: Transmogrifier<F, Widget = Component<B>>,
    {
        context
            .widget
            .content_widget
            .as_ref()
            .unwrap()
            .post_command::<F>(command);
    }
}
