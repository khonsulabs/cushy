use std::{convert::TryFrom, marker::PhantomData, sync::Arc};

use gooey_core::{WidgetRegistration, WidgetStorage};
use parking_lot::Mutex;

use super::{
    Accessor, ChangeEvent, Form, FormEvent, FormEventStatus, FormWidget, LocalizableError, Model,
    UpgradeableAccessor,
};
use crate::{
    component::{Component, EventMapper},
    input::Input,
};

pub struct TextField<M: Model, S: TryFrom<String, Error = E> + ToString, E> {
    password: bool,
    accessor: UpgradeableAccessor<M, S>,
    key: Option<M::Fields>,
    input: Option<WidgetRegistration>,
    _model: PhantomData<M>,
}

impl<
        M: Model,
        S: TryFrom<String, Error = E> + ToString + Send + Sync + 'static,
        E: LocalizableError,
    > FormWidget<M> for TextField<M, S, E>
{
    type Builder = Builder<M, S, E>;
    type Event = Event;
    type Kind = S;

    fn initialize(
        &mut self,
        key: M::Fields,
        model: Arc<Mutex<M>>,
        storage: &WidgetStorage,
        events: &EventMapper<Form<M>>,
    ) -> WidgetRegistration {
        self.accessor.upgrade(model);
        self.key = Some(key.clone());
        let mut input = Input::build()
            .value(self.accessor.as_field().get().to_string())
            .on_changed(events.map(move |_| FormEvent::field(key.clone(), Event::InputChanged)));
        if self.password {
            input = input.password();
        }
        self.input = Some(storage.register(input.finish()));
        self.input.clone().unwrap()
    }

    fn receive_event(
        &mut self,
        event: &Self::Event,
        context: &gooey_core::Context<Component<Form<M>>>,
    ) -> FormEventStatus {
        let Event::InputChanged = event;
        let value = context
            .map_widget::<Input, _, _>(self.input.as_ref().unwrap().id(), |input, _context| {
                input.value().to_string()
            });

        value.map_or(FormEventStatus::Unchanged, |value| {
            match S::try_from(value) {
                Ok(value) => {
                    self.accessor.as_field().set(value);
                    FormEventStatus::Changed(ChangeEvent::Valid)
                }
                Err(_) => FormEventStatus::Changed(ChangeEvent::Invalid),
            }
        })
    }

    fn build<A: Accessor<M, Self::Kind>>(accessor: A) -> Self::Builder {
        Builder::new(accessor)
    }
}

#[derive(Debug)]
pub enum Event {
    InputChanged,
}

pub struct Builder<M: Model, S: TryFrom<String, Error = E> + ToString, E> {
    field: TextField<M, S, E>,
}

impl<M: Model, S: TryFrom<String, Error = E> + ToString + Send + Sync + 'static, E>
    Builder<M, S, E>
{
    pub fn new<A: Accessor<M, S>>(accessor: A) -> Self {
        Self {
            field: TextField {
                password: false,
                accessor: UpgradeableAccessor::new(accessor),
                key: None,
                input: None,
                _model: PhantomData,
            },
        }
    }

    pub fn password(mut self) -> Self {
        self.field.password = true;
        self
    }

    pub fn finish(self) -> TextField<M, S, E> {
        self.field
    }
}

impl<
        M: Model,
        S: TryFrom<String, Error = E> + ToString + Send + Sync + 'static,
        E: LocalizableError,
    > gooey_core::Builder for Builder<M, S, E>
{
    type Output = TextField<M, S, E>;

    fn finish(self) -> Self::Output {
        Builder::finish(self)
    }
}
