use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub use gooey_reactor as reactor;
use gooey_reactor::{Reactor, Scope, ScopeGuard, Value};

pub trait Widget: Send + Sync + Debug + 'static {}

pub trait AnyWidget: Send + Sync + Debug + 'static {
    fn as_any(&self) -> &dyn Any;
    fn widget_type_id(&self) -> TypeId;
}

impl<T> AnyWidget for T
where
    T: Widget,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

pub struct BoxedWidget(Box<dyn AnyWidget>);

impl BoxedWidget {
    pub fn new<Widget>(widget: Widget) -> Self
    where
        Widget: AnyWidget,
    {
        Self(Box::new(widget))
    }
}

impl AsRef<dyn AnyWidget> for BoxedWidget {
    fn as_ref(&self) -> &dyn AnyWidget {
        self.0.as_ref()
    }
}

impl Debug for BoxedWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.as_ref().fmt(f)
    }
}

#[derive(Debug, Clone)]
pub struct Runtime {
    reactor: Reactor,
    root: Arc<ScopeGuard>,
    shutdown: Value<bool>,
}

impl Default for Runtime {
    fn default() -> Self {
        let reactor = Reactor::default();
        let root = reactor.new_scope();
        let shutdown = root.new_value(false);

        Self {
            reactor,
            root: Arc::new(root),
            shutdown,
        }
    }
}

impl Runtime {
    pub const fn root_scope(&self) -> &Arc<ScopeGuard> {
        &self.root
    }
}

pub trait Frontend: Send + Sync + Debug + Sized + 'static {
    // type AnyTransmogrifier: Transmogrify<Self>;
    type Context;
    type Instance;
    // type AnyWidget;
    // fn instantiate<W>(&mut self, )
}

pub trait AnyFrontend: Send + Sync + Debug + 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T> AnyFrontend for T
where
    T: Frontend,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
pub struct ActiveContext {
    pub frontend: Arc<dyn AnyFrontend>,
    pub scope: Scope,
}

impl ActiveContext {
    pub fn new_widget<Template, WidgetFn>(&self, widget_fn: WidgetFn) -> WidgetInstance<Template>
    where
        WidgetFn: FnOnce(ActiveContext) -> Template,
    {
        let scope = self.scope.new_scope();
        let widget = widget_fn(ActiveContext {
            frontend: self.frontend.clone(),
            scope: *scope,
        });

        WidgetInstance { widget, scope }
    }

    pub const fn scope(&self) -> Scope {
        self.scope
    }

    pub fn frontend<F>(&self) -> Option<&F>
    where
        F: Frontend,
    {
        self.frontend.as_any().downcast_ref()
    }

    pub fn new_value<T>(&self, value: T) -> Value<T>
    where
        T: Send + Sync,
    {
        self.scope.new_value(value)
    }
}

impl Debug for ActiveContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveContext")
            .field("frontend", &self.frontend)
            .field("scope", &self.scope)
            .finish()
    }
}

#[derive(Debug)]
pub struct WidgetInstance<W> {
    pub widget: W,
    pub scope: ScopeGuard,
}

impl<W> WidgetInstance<W> {
    pub fn map<R>(self, map: impl FnOnce(W) -> R) -> WidgetInstance<R> {
        WidgetInstance {
            widget: map(self.widget),
            scope: self.scope,
        }
    }

    pub fn boxed(self) -> WidgetInstance<BoxedWidget>
    where
        W: Widget,
    {
        WidgetInstance {
            widget: BoxedWidget::new(self.widget),
            scope: self.scope,
        }
    }
}

impl<W> Deref for WidgetInstance<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<W> DerefMut for WidgetInstance<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

#[derive(Clone, Debug)]
pub enum WidgetValue<T>
where
    T: 'static,
{
    Static(T),
    Value(Value<T>),
}

impl<T> Default for WidgetValue<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::Static(T::default())
    }
}

impl<T> WidgetValue<T> {
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        match self {
            WidgetValue::Static(value) => value.clone(),
            WidgetValue::Value(value) => value.get().expect("invalid value reference"),
        }
    }

    pub fn map_ref<R>(&self, map: impl FnOnce(&T) -> R) -> R {
        match self {
            WidgetValue::Static(value) => map(value),
            WidgetValue::Value(value) => value.map_ref(map).expect("invalid value reference"),
        }
    }
}

impl<T> From<T> for WidgetValue<T> {
    fn from(value: T) -> Self {
        Self::Static(value)
    }
}

impl<'a, T> From<&'a T> for WidgetValue<T>
where
    T: Clone,
{
    fn from(value: &'a T) -> Self {
        Self::Static(value.clone())
    }
}

impl<T> From<Value<T>> for WidgetValue<T> {
    fn from(value: Value<T>) -> Self {
        Self::Value(value)
    }
}

impl<'a> From<&'a str> for WidgetValue<String> {
    fn from(value: &'a str) -> Self {
        Self::Static(value.to_string())
    }
}

pub struct Callback<T>(Box<dyn AnyCallback<T>>);

impl<T> Clone for Callback<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        Callback(self.0.cloned())
    }
}

impl<T> Debug for Callback<T>
where
    T: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.type_name())
    }
}

impl<T> Callback<T>
where
    T: 'static,
{
    pub fn new<F>(function: F) -> Self
    where
        F: AnyCallback<T>,
    {
        Self(Box::new(function))
    }

    pub fn invoke(&mut self, value: T) {
        self.0.invoke(value);
    }
}

pub trait AnyCallback<T>: Send + Sync + 'static {
    fn cloned(&self) -> Box<dyn AnyCallback<T>>;
    fn invoke(&mut self, value: T);
    fn type_name(&self) -> &'static str;
}

impl<T, F> AnyCallback<T> for F
where
    F: FnMut(T) + Clone + Send + Sync + 'static,
{
    fn invoke(&mut self, value: T) {
        self(value)
    }

    fn cloned(&self) -> Box<dyn AnyCallback<T>> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        type_name::<F>()
    }
}

#[macro_export]
macro_rules! define_widget_factories {
    ($name:ident, ) => {};
}

pub struct Widgets<F>
where
    F: Frontend,
{
    by_id: HashMap<TypeId, Box<dyn Transmogrify<F>>>,
    frontend: PhantomData<&'static F>,
}

impl<F> Default for Widgets<F>
where
    F: Frontend,
{
    fn default() -> Self {
        Self {
            by_id: HashMap::default(),
            frontend: PhantomData,
        }
    }
}

impl<F> Widgets<F>
where
    F: Frontend,
{
    pub fn add<T>(&mut self)
    where
        T: WidgetTransmogrifier<F> + Transmogrify<F> + Default,
    {
        #[allow(clippy::box_default)] // This breaks the dyn cast.
        self.by_id
            .insert(TypeId::of::<T::Widget>(), Box::new(T::default()));
    }

    pub fn with<T>(mut self) -> Self
    where
        T: WidgetTransmogrifier<F> + Transmogrify<F> + Default,
    {
        self.add::<T>();
        self
    }

    pub fn instantiate(&self, widget: &dyn AnyWidget, context: &F::Context) -> F::Instance {
        let Some(transmogrifier) = self.by_id.get(&widget.widget_type_id()) else { unreachable!("WebWidget not registered") };
        transmogrifier.transmogrify(widget, context)
    }
}

impl<F> Debug for Widgets<F>
where
    F: Frontend,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut tuple = f.debug_tuple("WebWidgets");
        for transmogrifier in self.by_id.values() {
            tuple.field(&transmogrifier.widget_type_name());
        }
        tuple.finish()
    }
}

pub trait WidgetTransmogrifier<F>: Transmogrify<F>
where
    F: Frontend,
{
    type Widget: Widget;
    fn transmogrify(&self, widget: &Self::Widget, context: &F::Context) -> F::Instance;
}

pub trait Transmogrify<F>: Send + Sync + 'static
where
    F: Frontend,
{
    fn transmogrify(&self, widget: &dyn AnyWidget, context: &F::Context) -> F::Instance;
    fn widget_type_name(&self) -> &'static str {
        type_name::<Self>()
    }
}

impl<F, T> Transmogrify<F> for T
where
    T: WidgetTransmogrifier<F>,
    F: Frontend,
{
    fn transmogrify(
        &self,
        widget: &dyn AnyWidget,
        context: &<F as Frontend>::Context,
    ) -> <F as Frontend>::Instance {
        let widget = widget
            .as_any()
            .downcast_ref::<T::Widget>()
            .expect("type mismatch");
        WidgetTransmogrifier::transmogrify(self, widget, context)
    }
}
