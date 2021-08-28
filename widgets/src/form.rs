use std::{collections::HashMap, fmt::Debug, marker::PhantomData, ops::Deref, sync::Arc};

use gooey_core::{
    styles::style_sheet::Classes, AnySendSync, AppContext, Builder as _, Callback, Context, Key,
    StyledWidget, WidgetRegistration, WidgetStorage,
};

mod text_field;

use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
pub use text_field::TextField;

use crate::{
    component::{Behavior, Component, Content, EventMapper},
    label::Label,
    list::List,
};

pub static LABEL_CLASS: &str = "gooey-form-label";

#[derive(Debug)]
pub struct Form<M: Model> {
    pub model: Arc<Mutex<M>>,
    fields: HashMap<M::Fields, InstantiatedField<M>>,
    order: Vec<M::Fields>,
    changed: Callback<ChangeEvent>,
}

#[derive(Debug)]
struct InstantiatedField<M: Model> {
    field: Box<dyn AnyField<M>>,
    valid: bool,
}

#[derive(Debug, Clone)]
pub enum ChangeEvent {
    Valid,
    Invalid,
}

impl<M: Model> Form<M> {
    pub fn build(model: M, storage: &WidgetStorage) -> Builder<'_, M> {
        Builder::new(model, storage)
    }
}

pub trait AnyField<M: Model>: AnySendSync {
    fn receive_event(
        &mut self,
        event: &dyn AnySendSync,
        context: &Context<Component<Form<M>>>,
    ) -> FormEventStatus;
    fn build(
        &mut self,
        key: M::Fields,
        model: Arc<Mutex<M>>,
        storage: &WidgetStorage,
        events: &EventMapper<Form<M>>,
    ) -> WidgetRegistration;
}

impl<M: Model, K: Send + Sync + 'static, W: FormWidget<M, Kind = K>> AnyField<M>
    for FormField<M, K, W>
{
    fn receive_event(
        &mut self,
        event: &dyn AnySendSync,
        context: &Context<Component<Form<M>>>,
    ) -> FormEventStatus {
        let event = event.as_any().downcast_ref();
        self.widget.receive_event(event.unwrap(), context)
    }

    fn build(
        &mut self,
        key: M::Fields,
        model: Arc<Mutex<M>>,
        storage: &WidgetStorage,
        events: &EventMapper<Form<M>>,
    ) -> WidgetRegistration {
        self.widget.initialize(key, model, storage, events)
    }
}

pub trait Accessor<M: Model, K: Send + Sync + 'static>: Send + Sync + 'static {
    fn get<'a>(&self, model: &'a mut M) -> &'a mut K;
    fn set(&self, model: &mut M, new_value: K);
}

impl<
        F: for<'a> Fn(&'a mut M) -> &'a mut K + Send + Sync + 'static,
        M: Model,
        K: Clone + Send + Sync + 'static,
    > Accessor<M, K> for F
{
    fn get<'a>(&self, model: &'a mut M) -> &'a mut K {
        self(model)
    }

    fn set(&self, model: &mut M, new_value: K) {
        *self(model) = new_value;
    }
}

#[derive(Debug)]
#[must_use]
pub struct Builder<'a, M: Model> {
    storage: &'a WidgetStorage,
    form: Form<M>,
}

impl<'a, M: Model> Builder<'a, M> {
    pub fn new(model: M, storage: &'a WidgetStorage) -> Self {
        Self {
            storage,
            form: Form {
                model: Arc::new(Mutex::new(model)),
                fields: HashMap::default(),
                order: Vec::default(),
                changed: Callback::default(),
            },
        }
    }

    pub fn field<W: FormWidget<M, Kind = K>, K: Key>(mut self, key: M::Fields, widget: W) -> Self {
        self.form.fields.insert(
            key.clone(),
            InstantiatedField {
                field: Box::new(FormField::new(key.clone(), widget)),
                valid: true,
            },
        );
        self.form.order.push(key);
        self
    }

    pub fn on_changed(mut self, on_changed: Callback<ChangeEvent>) -> Self {
        self.form.changed = on_changed;
        self
    }

    pub fn finish(self) -> StyledWidget<Component<Form<M>>> {
        Component::new(self.form, self.storage)
    }
}

pub trait Model: Sized + Debug + Send + Sync + 'static {
    type Fields: FormKey;
}

impl<M: Model> Behavior for Form<M> {
    type Content = List;
    type Event = FormEvent<M::Fields>;
    type Widgets = ();

    fn build_content(
        &mut self,
        mut builder: <Self::Content as Content<Self>>::Builder,
        events: &EventMapper<Self>,
    ) -> StyledWidget<Self::Content> {
        builder = builder.unadorned();
        for key in &self.order {
            let instance = self.fields.get_mut(key).unwrap();
            if let Some(label) = key.label(builder.storage().app()) {
                builder = builder.with(Label::new(label).with(Classes::from(LABEL_CLASS)));
            }
            let widget =
                instance
                    .field
                    .build(key.clone(), self.model.clone(), builder.storage(), events);
            builder = builder.with_registration(widget);
        }

        builder.finish()
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &gooey_core::Context<Component<Self>>,
    ) {
        let FormEvent::Field { key, event } = event;
        if let Some(instance) = component.fields.get_mut(&key) {
            match instance.field.receive_event(event.as_ref(), context) {
                FormEventStatus::Changed(change) => {
                    instance.valid = matches!(change, ChangeEvent::Valid);
                }
                FormEventStatus::Unchanged => {}
            }
        }

        let all_valid = component.fields.values().all(|field| field.valid);
        component.behavior.changed.invoke(if all_valid {
            ChangeEvent::Valid
        } else {
            ChangeEvent::Invalid
        });
    }
}

struct FormField<M: Model, K, W: FormWidget<M, Kind = K>> {
    name: M::Fields,
    widget: W,
    _phantom: PhantomData<(M, K)>,
}

impl<M: Model, K: Send + Sync + 'static, W: FormWidget<M, Kind = K>> FormField<M, K, W> {
    pub fn new(name: M::Fields, widget: W) -> Self {
        Self {
            name,
            widget,
            _phantom: PhantomData,
        }
    }
}

impl<M: Model, K, W: FormWidget<M, Kind = K>> Debug for FormField<M, K, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormField")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
#[must_use]
pub enum FormEventStatus {
    Changed(ChangeEvent),
    Unchanged,
}

pub trait FormWidget<M: Model>: Sized + Send + Sync + 'static {
    type Kind: Send + Sync + 'static;
    type Event;
    type Builder: gooey_core::Builder<Output = Self>;

    fn initialize(
        &mut self,
        key: M::Fields,
        model: Arc<Mutex<M>>,
        storage: &WidgetStorage,
        events: &EventMapper<Form<M>>,
    ) -> WidgetRegistration;

    fn receive_event(
        &mut self,
        event: &Self::Event,
        context: &Context<Component<Form<M>>>,
    ) -> FormEventStatus;

    fn build<A: Accessor<M, Self::Kind>>(accessor: A) -> Self::Builder;

    fn build_simple<F: for<'a> Fn(&'a mut M) -> &'a mut Self::Kind + Send + Sync + 'static>(
        accessor: F,
    ) -> Self::Builder
    where
        Self::Kind: Clone,
    {
        Self::build(accessor)
    }

    fn simple<F: for<'a> Fn(&'a mut M) -> &'a mut Self::Kind + Send + Sync + 'static>(
        accessor: F,
    ) -> Self
    where
        Self::Kind: Clone,
    {
        Self::build(accessor).finish()
    }
}

pub struct FieldAccessor<M: Model, K> {
    model: Arc<Mutex<M>>,
    accessor: Arc<dyn Accessor<M, K>>,
}

pub struct FieldGuard<'a, K> {
    model: MappedMutexGuard<'a, K>,
}

impl<'a, K: Send + Sync + 'static> Deref for FieldGuard<'a, K> {
    type Target = K;

    fn deref(&self) -> &Self::Target {
        &*self.model
    }
}

impl<M: Model, K> Clone for FieldAccessor<M, K> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            accessor: self.accessor.clone(),
        }
    }
}

impl<M: Model, K: Send + Sync + 'static> FieldAccessor<M, K> {
    pub fn set(&self, new_value: K) {
        let mut model = self.model.lock();
        self.accessor.set(&mut model, new_value);
    }
}

impl<M: Model, K: Send + Sync + 'static> FieldAccessor<M, K> {
    #[must_use]
    pub fn get(&self) -> FieldGuard<'_, K> {
        let model = self.model.lock();
        FieldGuard {
            model: MutexGuard::map(model, |model| self.accessor.get(model)),
        }
    }
}

#[derive(Debug)]
pub enum FormEvent<K: Key> {
    Field { key: K, event: Box<dyn AnySendSync> },
}

impl<K: Key> FormEvent<K> {
    pub fn field<E: AnySendSync>(key: K, event: E) -> Self {
        Self::Field {
            key,
            event: Box::new(event),
        }
    }
}

enum UpgradeableAccessor<M: Model, K> {
    Accessor(Arc<dyn Accessor<M, K>>),
    FieldAccessor(FieldAccessor<M, K>),
}

impl<M: Model, K: Send + Sync + 'static> UpgradeableAccessor<M, K> {
    pub fn new<A: Accessor<M, K>>(accessor: A) -> Self {
        Self::Accessor(Arc::new(accessor))
    }

    pub fn upgrade(&mut self, model: Arc<Mutex<M>>) {
        match self {
            Self::Accessor(accessor) => {
                *self = Self::FieldAccessor(FieldAccessor {
                    model,
                    accessor: accessor.clone(),
                });
            }
            Self::FieldAccessor(_) => panic!("upgrade called a second time"),
        }
    }

    pub fn as_field(&self) -> &FieldAccessor<M, K> {
        match self {
            UpgradeableAccessor::FieldAccessor(field) => field,
            UpgradeableAccessor::Accessor(_) => panic!("accessor was not upgraded"),
        }
    }
}

pub struct SimpleAccessor<M: Model, K> {
    accessor: Arc<dyn Accessor<M, K>>,
}

impl<M: Model, K: Clone + Send + Sync + 'static> SimpleAccessor<M, K> {
    pub fn new<F: for<'a> Fn(&'a mut M) -> &'a mut K + Send + Sync + 'static>(function: F) -> Self {
        Self {
            accessor: Arc::new(function),
        }
    }
}

impl<M: Model, K: Send + Sync + 'static> Accessor<M, K> for SimpleAccessor<M, K> {
    fn get<'a>(&self, model: &'a mut M) -> &'a mut K {
        self.accessor.get(model)
    }

    fn set(&self, model: &mut M, new_value: K) {
        self.accessor.set(model, new_value);
    }
}

pub trait FormKey: Key {
    fn label(&self, context: &AppContext) -> Option<String>;
}
