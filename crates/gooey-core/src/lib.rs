//! Core types and functionality for Gooey applications.
#![forbid(unsafe_code)]
#![warn(
    //missing_docs,
    clippy::pedantic,
)]
#![allow(clippy::module_name_repetitions)]

use std::any::{type_name, Any};
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::Arc;

use alot::OrderedLots;
pub use {figures as math, gooey_reactor as reactor};
pub mod graphics;
pub mod style;
pub mod window;
// mod tree;
pub use gooey_macros::Widget;
use gooey_reactor::{Dynamic, Reactor, Scope, ScopeGuard};
use stylecs::{Identifier, Name, Style};

use crate::style::{DynamicStyle, Library, WidgetStyle};

/// A Gooey widget.
// TODO document derive macro
pub trait Widget: BaseStyle + RefUnwindSafe + UnwindSafe + Send + Sync + Debug + 'static {
    /// The unique name of this widget.
    fn name(&self) -> Name;
}

/// Base style definitions for a widget.
pub trait BaseStyle {
    /// Returns the base style to apply to instances of this widget.
    ///
    /// Components that the user specifies in themes or inline on the widget
    /// itself will override styles returned from this fuction.
    fn base_style(&self, library: &Library) -> WidgetStyle;
}

/// A Rust-native widget implementation.
///
/// This trait allows identifying Rust widgets by their [`Name`] without having
/// an instance present. This is useful for creating style selectors.
pub trait StaticWidget: Widget {
    /// Returns the name of this widget.
    ///
    /// This function is similar to [`Widget::name()`] but does not require an
    /// instance of the widget to call.
    ///
    /// This function and [`Widget::name()`] should return identical values.
    fn static_name() -> Name;
}

/// A [`Widget`] that is trait object-safe.
pub trait AnyWidget: RefUnwindSafe + UnwindSafe + Send + Sync + Debug + 'static {
    /// Returns this widget as an [`Any`] type.
    fn as_any(&self) -> &dyn Any;
    /// Returns the result of [`Widget::name()`].
    fn name(&self) -> Name;
    /// Returns the result of [`BaseStyle::base_style()`].
    fn base_style(&self, library: &Library) -> WidgetStyle;
}

impl<T> AnyWidget for T
where
    T: Widget,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> Name {
        self.name()
    }

    fn base_style(&self, library: &Library) -> WidgetStyle {
        T::base_style(self, library)
    }
}

/// A widget that has been boxed to "erase" the Widget's type.
///
/// This abstraction allows one widget to own another widget without needing to
/// know the type of widget it owns.
pub struct BoxedWidget(Box<dyn AnyWidget>);

impl BoxedWidget {
    /// Returns a new boxed widget containing `widget`.
    pub fn new<Widget>(widget: Widget) -> Self
    where
        Widget: AnyWidget,
    {
        Self(Box::new(widget))
    }
}

impl Deref for BoxedWidget {
    type Target = dyn AnyWidget;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl AsRef<dyn AnyWidget> for BoxedWidget {
    fn as_ref(&self) -> &dyn AnyWidget {
        self.0.as_ref()
    }
}

impl Debug for BoxedWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

/// A Gooey runtime.
///
/// The runtime owns all of the widget values and structures that drive the
/// reactive system. A runtime can be used to power more than one window.
#[derive(Debug, Clone)]
pub struct Runtime {
    root: Arc<ScopeGuard>,
}

impl Default for Runtime {
    fn default() -> Self {
        let reactor = Reactor::default();
        let root = reactor.new_scope();

        Self {
            root: Arc::new(root),
        }
    }
}

impl Runtime {
    /// Returns a reference to the root scope.
    #[must_use]
    pub const fn root_scope(&self) -> &Arc<ScopeGuard> {
        &self.root
    }
}

/// A frontend that displays a Gooey appliation.
pub trait Frontend: RefUnwindSafe + UnwindSafe + Send + Sync + Debug + Sized + 'static {
    /// A frontend-specific type that is provided as a context to
    /// [`WidgetTransmogrifier::transmogrify`].
    type Context;
    /// The type that a [`Widget`] is transmogrified into by this frontend.
    ///
    /// For example:
    ///
    /// - In `gooey-web`, this is `web_sys::Node`
    /// - In `gooey-raster`, this is `gooey_raster::Rasterizable`
    type Instance;
}

/// A trait-object-safe interface for a [`Frontend`].
pub trait AnyFrontend: RefUnwindSafe + UnwindSafe + Send + Sync + Debug + 'static {
    /// Returns the underlying type as an Any instance for downcasting.
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

/// A context within a Gooey application.
///
/// Each context is associated with the scope of a specific widget. When new
/// widgets or values are created using this context, they are associated with
/// this context's scope.
#[derive(Clone)]
pub struct Context {
    /// The current frontend of the application.
    pub frontend: Arc<dyn AnyFrontend>,
    /// The scope of this context.
    pub scope: Scope,
}

impl Context {
    /// Returns a new context for the root scope of a runtime.
    pub fn root<F>(frontend: F, runtime: &Runtime) -> Self
    where
        F: Frontend,
    {
        Self {
            frontend: Arc::new(frontend),
            scope: ***runtime.root_scope(),
        }
    }

    /// Creates a new widget, calling `widget_fn` with a new [`Context`] for
    /// with its own scope.
    pub fn new_widget<NewWidget, WidgetFn>(
        &self,
        widget_fn: WidgetFn,
    ) -> WidgetInstance<NewWidget::Widget>
    where
        WidgetFn: FnOnce(Context) -> NewWidget,
        NewWidget: IntoNewWidget,
    {
        let scope = self.scope.new_scope();
        let widget = widget_fn(Context {
            frontend: self.frontend.clone(),
            scope: *scope,
        })
        .into_new(self);

        WidgetInstance {
            id: widget.id,
            style: widget.style,
            widget: widget.widget,
            scope,
        }
    }

    /// Returns a reference to the frontend, if the current frontend is of type
    /// `F`.
    #[must_use]
    pub fn frontend<F>(&self) -> Option<&F>
    where
        F: Frontend,
    {
        self.frontend.as_any().downcast_ref()
    }

    /// Creates a new dynamic value containing `initial_value`.
    #[must_use]
    pub fn new_dynamic<T>(&self, initial_value: T) -> Dynamic<T>
    where
        T: Send + Sync,
    {
        self.scope.new_dynamic(initial_value)
    }
}

impl Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveContext")
            .field("frontend", &self.frontend)
            .field("scope", &self.scope)
            .finish()
    }
}

/// An instance of a [`Widget`].
pub struct WidgetInstance<W> {
    /// The widget instance.
    pub widget: W,
    /// The unique identifier of this widget instance.
    pub id: Option<Identifier>,
    // TODO classes: WidgetValue<Classes>, needs a Set type.
    /// The effective style of this widget instance.
    pub style: DynamicStyle,
    /// The scope guard that keeps this widget instance's scope alive.
    pub scope: ScopeGuard,
}

impl<W> WidgetInstance<W> {
    /// Maps the widget using `map` and returns a new [`WidgetInstance`]
    /// containing the result.
    pub fn map<R>(self, map: impl FnOnce(W) -> R) -> WidgetInstance<R> {
        WidgetInstance {
            widget: map(self.widget),
            id: self.id,
            style: self.style,
            scope: self.scope,
        }
    }

    /// Returns a boxed widget instance.
    pub fn boxed(self) -> WidgetInstance<BoxedWidget>
    where
        W: Widget,
    {
        self.map(BoxedWidget::new)
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

impl<W> Debug for WidgetInstance<W>
where
    W: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.widget.fmt(f)
    }
}

pub struct NewWidget<W> {
    pub id: Option<Identifier>,
    // TODO classes: WidgetValue<Classes>, needs a Set type.
    pub style: DynamicStyle,
    pub widget: W,
}

impl<W> NewWidget<W> {
    pub fn new(widget: W, context: &Context) -> Self {
        Self {
            id: None,
            style: DynamicStyle::new(context),
            widget,
        }
    }
}

pub trait IntoNewWidget {
    type Widget: Widget;

    fn into_new(self, context: &Context) -> NewWidget<Self::Widget>;
}

impl<W> IntoNewWidget for W
where
    W: Widget,
{
    type Widget = W;

    fn into_new(self, context: &Context) -> NewWidget<Self::Widget> {
        NewWidget::new(self, context)
    }
}

impl<W> IntoNewWidget for NewWidget<W>
where
    W: Widget,
{
    type Widget = W;

    fn into_new(self, _context: &Context) -> NewWidget<Self::Widget> {
        self
    }
}

pub trait WidgetExt {
    type Widget: Widget;
    fn id(self, id: Identifier) -> NewWidget<Self::Widget>;
}

impl<W> WidgetExt for NewWidget<W>
where
    W: Widget,
{
    type Widget = W;

    fn id(mut self, id: Identifier) -> Self {
        self.id = Some(id);
        self
    }
}

#[derive(Clone, Debug)]
pub enum Value<T>
where
    T: 'static,
{
    Static(T),
    Dynamic(Dynamic<T>),
}

impl<T> Default for Value<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::Static(T::default())
    }
}

impl<T> Value<T> {
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        match self {
            Value::Static(value) => value.clone(),
            Value::Dynamic(value) => value.get().expect("invalid value reference"),
        }
    }

    pub fn map_ref<R>(&self, map: impl FnOnce(&T) -> R) -> R {
        match self {
            Value::Static(value) => map(value),
            Value::Dynamic(value) => value.map_ref(map).expect("invalid value reference"),
        }
    }

    pub fn map_each<R, F>(&self, mut map: F) -> Option<Value<R>>
    where
        F: for<'a> FnMut(&'a T) -> R + Send + 'static,
        R: Send + Sync,
    {
        match self {
            Value::Static(value) => Some(Value::Static(map(value))),
            Value::Dynamic(value) => value.map_each(map).map(Value::Dynamic),
        }
    }

    pub fn for_each<F>(&self, mut map: F)
    where
        F: for<'a> FnMut(&'a T) + Send + 'static,
    {
        match self {
            Value::Static(value) => map(value),
            Value::Dynamic(value) => value.for_each(map),
        }
    }
}

impl<T> From<T> for Value<T> {
    fn from(value: T) -> Self {
        Self::Static(value)
    }
}

impl<'a, T> From<&'a T> for Value<T>
where
    T: Clone,
{
    fn from(value: &'a T) -> Self {
        Self::Static(value.clone())
    }
}

impl<T> From<Dynamic<T>> for Value<T> {
    fn from(value: Dynamic<T>) -> Self {
        Self::Dynamic(value)
    }
}

impl<'a> From<&'a str> for Value<String> {
    fn from(value: &'a str) -> Self {
        Self::Static(value.to_string())
    }
}

#[repr(transparent)]
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

pub trait AnyCallback<T>: RefUnwindSafe + UnwindSafe + Send + Sync + 'static {
    fn cloned(&self) -> Box<dyn AnyCallback<T>>;
    fn invoke(&mut self, value: T);
    fn type_name(&self) -> &'static str;
}

impl<T, F> AnyCallback<T> for F
where
    F: FnMut(T) + Clone + RefUnwindSafe + UnwindSafe + Send + Sync + 'static,
{
    fn invoke(&mut self, value: T) {
        self(value);
    }

    fn cloned(&self) -> Box<dyn AnyCallback<T>> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        type_name::<F>()
    }
}

pub struct Widgets<F>
where
    F: Frontend,
{
    by_name: HashMap<Name, AnyTransmogrifier<F>>,
}

impl<F> Default for Widgets<F>
where
    F: Frontend,
{
    fn default() -> Self {
        Self {
            by_name: HashMap::default(),
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
        T::Widget: StaticWidget,
    {
        #[allow(clippy::box_default)] // This breaks the dyn cast.
        self.by_name.insert(
            <T::Widget as StaticWidget>::static_name(),
            AnyTransmogrifier::new(T::default()),
        );
    }

    #[must_use]
    pub fn with<T>(mut self) -> Self
    where
        T: WidgetTransmogrifier<F> + Transmogrify<F> + Default,
        T::Widget: StaticWidget,
    {
        self.add::<T>();
        self
    }

    #[must_use]
    pub fn instantiate(
        &self,
        widget: &dyn AnyWidget,
        style: Dynamic<Style>,
        context: &F::Context,
    ) -> F::Instance {
        let Some(transmogrifier) = self.by_name.get(&widget.name()) else {
            unreachable!("{} not registered", widget.name())
        };
        transmogrifier.0.transmogrify(widget, style, context)
    }

    #[must_use]
    pub fn get<T>(&self) -> Option<&AnyTransmogrifier<F>>
    where
        T: StaticWidget,
    {
        self.by_name.get(&T::static_name())
    }
}

impl<F> Debug for Widgets<F>
where
    F: Frontend,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut tuple = f.debug_tuple("WebWidgets");
        for transmogrifier in self.by_name.values() {
            tuple.field(&transmogrifier.0.widget_type_name());
        }
        tuple.finish()
    }
}

pub struct AnyTransmogrifier<F>(Box<dyn Transmogrify<F>>);

impl<F> AnyTransmogrifier<F>
where
    F: Frontend,
{
    pub fn new<T>(transmogrifier: T) -> Self
    where
        T: WidgetTransmogrifier<F>,
    {
        Self(Box::new(transmogrifier))
    }
}

pub trait WidgetTransmogrifier<F>: Transmogrify<F>
where
    F: Frontend,
{
    type Widget: Widget;
    fn transmogrify(
        &self,
        widget: &Self::Widget,
        style: Dynamic<Style>,
        context: &F::Context,
    ) -> F::Instance;
}

pub trait Transmogrify<F>: RefUnwindSafe + UnwindSafe + Send + Sync + 'static
where
    F: Frontend,
{
    fn transmogrify(
        &self,
        widget: &dyn AnyWidget,
        style: Dynamic<Style>,
        context: &F::Context,
    ) -> F::Instance;

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
        style: Dynamic<Style>,
        context: &<F as Frontend>::Context,
    ) -> <F as Frontend>::Instance {
        let widget = widget
            .as_any()
            .downcast_ref::<T::Widget>()
            .expect("type mismatch");
        WidgetTransmogrifier::transmogrify(self, widget, style, context)
    }
}

pub struct Children {
    context: Context,
    children: OrderedLots<Child>,
}

impl Children {
    #[must_use]
    pub fn new(context: &Context) -> Self {
        Self {
            context: context.clone(),
            children: OrderedLots::new(),
        }
    }

    #[must_use]
    pub fn with_widget<NewWidget>(self, widget: NewWidget) -> Self
    where
        NewWidget: IntoNewWidget,
    {
        self.with(|_| widget)
    }

    #[must_use]
    pub fn with_named_widget<NewWidget>(self, name: Name, widget: NewWidget) -> Self
    where
        NewWidget: IntoNewWidget,
    {
        self.with_named(name, |_| widget)
    }

    #[must_use]
    pub fn with<NewWidget, WidgetFn>(mut self, widget_fn: WidgetFn) -> Self
    where
        WidgetFn: FnOnce(Context) -> NewWidget,
        NewWidget: IntoNewWidget,
    {
        self.push(widget_fn);
        self
    }

    #[must_use]
    pub fn with_named<NewWidget, WidgetFn>(mut self, name: Name, widget_fn: WidgetFn) -> Self
    where
        WidgetFn: FnOnce(Context) -> NewWidget,
        NewWidget: IntoNewWidget,
    {
        self.push_named(name, widget_fn);
        self
    }

    pub fn push<NewWidget, WidgetFn>(&mut self, widget_fn: WidgetFn)
    where
        WidgetFn: FnOnce(Context) -> NewWidget,
        NewWidget: IntoNewWidget,
    {
        let widget = self.context.new_widget(widget_fn);
        self.children.push(Child {
            name: None,
            widget: widget.boxed(),
        });
    }

    pub fn push_named<NewWidget, WidgetFn>(&mut self, name: Name, widget_fn: WidgetFn)
    where
        WidgetFn: FnOnce(Context) -> NewWidget,
        NewWidget: IntoNewWidget,
    {
        let widget = self.context.new_widget(widget_fn);

        self.children.push(Child {
            name: Some(name),
            widget: widget.boxed(),
        });
    }

    pub fn insert<NewWidget, WidgetFn>(&mut self, index: usize, widget_fn: WidgetFn)
    where
        WidgetFn: FnOnce(Context) -> NewWidget,
        NewWidget: IntoNewWidget,
    {
        let widget = self.context.new_widget(widget_fn);
        self.children.insert(
            index,
            Child {
                name: None,
                widget: widget.boxed(),
            },
        );
    }

    pub fn insert_named<NewWidget, WidgetFn>(
        &mut self,
        index: usize,
        name: Name,
        widget_fn: WidgetFn,
    ) where
        WidgetFn: FnOnce(Context) -> NewWidget,
        NewWidget: IntoNewWidget,
    {
        let widget = self.context.new_widget(widget_fn);

        self.children.insert(
            index,
            Child {
                name: Some(name),
                widget: widget.boxed(),
            },
        );
    }

    #[must_use]
    pub fn entries(&self) -> alot::ordered::EntryIter<'_, Child> {
        self.children.entries()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.children.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl Debug for Children {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for child in &self.children {
            list.entry(child);
        }
        list.finish()
    }
}

pub struct Child {
    name: Option<Name>,
    widget: WidgetInstance<BoxedWidget>,
}

impl Child {
    #[must_use]
    pub const fn name(&self) -> Option<&Name> {
        self.name.as_ref()
    }
}

impl Deref for Child {
    type Target = WidgetInstance<BoxedWidget>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl Debug for Child {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            f.debug_map().entry(&name, &self.widget).finish()
        } else {
            self.widget.fmt(f)
        }
    }
}

#[test]
fn children_debug() {
    #[derive(Debug)]
    struct NullFrontend;
    impl Frontend for NullFrontend {
        type Context = ();
        type Instance = ();
    }

    #[derive(Debug, Widget)]
    #[widget(name = test_widget,  core = crate)]
    struct TestWidget(u32);

    let reactor = Reactor::default();
    let guard = reactor.new_scope();
    let context = Context {
        frontend: Arc::new(NullFrontend),
        scope: guard.scope(),
    };

    let debug = format!(
        "{:?}",
        Children::new(&context)
            .with(|_| TestWidget(1))
            .with_named(Name::private("second").unwrap(), |_| TestWidget(1))
    );
    assert_eq!(
        debug,
        "[TestWidget(1), {Name { authority: \"_\", name: \"second\" }: TestWidget(1)}]"
    );
}
