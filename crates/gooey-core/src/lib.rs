use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::Arc;

use alot::OrderedLots;
pub use {figures as math, gooey_reactor as reactor};
pub mod graphics;
pub mod style;
mod tree;
pub use gooey_macros::Widget;
use gooey_reactor::{Reactor, Scope, ScopeGuard, Value};
use stylecs::{Identifier, Name, Style};

use crate::style::{DynamicStyle, Library, WidgetStyle};

pub trait Widget: BaseStyle + RefUnwindSafe + UnwindSafe + Send + Sync + Debug + 'static {
    fn name(&self) -> Name;
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

pub trait BaseStyle {
    fn base_style(&self, library: &Library) -> WidgetStyle;
}

/// A Rust-native widget implementation.
///
/// This trait allows identifying Rust widgets by their [`Name`] without having
/// an instance present. This is useful for creating style selectors.
pub trait StaticWidget: Widget {
    fn static_name() -> Name;
}

pub trait AnyWidget: RefUnwindSafe + UnwindSafe + Send + Sync + Debug + 'static {
    fn as_any(&self) -> &dyn Any;
    fn widget_type_id(&self) -> TypeId;
    fn name(&self) -> Name;
    fn style(&self, library: &Library) -> WidgetStyle;
}

impl<T> AnyWidget for T
where
    T: Widget,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_type_id(&self) -> TypeId {
        self.type_id()
    }

    fn name(&self) -> Name {
        self.name()
    }

    fn style(&self, library: &Library) -> WidgetStyle {
        T::base_style(self, library)
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

pub trait Frontend: RefUnwindSafe + UnwindSafe + Send + Sync + Debug + Sized + 'static {
    type Context;
    type Instance;
}

pub trait AnyFrontend: RefUnwindSafe + UnwindSafe + Send + Sync + Debug + 'static {
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
    pub fn root<F>(frontend: F, runtime: &Runtime) -> Self
    where
        F: Frontend,
    {
        Self {
            frontend: Arc::new(frontend),
            scope: ***runtime.root_scope(),
        }
    }

    pub fn new_widget<NewWidget, WidgetFn>(
        &self,
        widget_fn: WidgetFn,
    ) -> WidgetInstance<NewWidget::Widget>
    where
        WidgetFn: FnOnce(ActiveContext) -> NewWidget,
        NewWidget: IntoNewWidget,
    {
        let scope = self.scope.new_scope();
        let widget = widget_fn(ActiveContext {
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

pub struct WidgetInstance<W> {
    pub id: Option<Identifier>,
    // TODO classes: WidgetValue<Classes>, needs a Set type.
    pub style: DynamicStyle,
    pub widget: W,
    pub scope: ScopeGuard,
}

impl<W> WidgetInstance<W> {
    pub fn map<R>(self, map: impl FnOnce(W) -> R) -> WidgetInstance<R> {
        WidgetInstance {
            widget: map(self.widget),
            id: self.id,
            style: self.style,
            scope: self.scope,
        }
    }

    pub fn boxed(self) -> WidgetInstance<BoxedWidget>
    where
        W: Widget,
    {
        WidgetInstance {
            widget: BoxedWidget::new(self.widget),
            id: self.id,
            style: self.style,
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
    pub fn new(widget: W, context: &ActiveContext) -> Self {
        Self {
            id: None,
            style: DynamicStyle::new(context),
            widget,
        }
    }
}

pub trait IntoNewWidget {
    type Widget: Widget;

    fn into_new(self, context: &ActiveContext) -> NewWidget<Self::Widget>;
}

impl<W> IntoNewWidget for W
where
    W: Widget,
{
    type Widget = W;

    fn into_new(self, context: &ActiveContext) -> NewWidget<Self::Widget> {
        NewWidget::new(self, context)
    }
}

impl<W> IntoNewWidget for NewWidget<W>
where
    W: Widget,
{
    type Widget = W;

    fn into_new(self, _context: &ActiveContext) -> NewWidget<Self::Widget> {
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

    pub fn map_each<R, F>(&self, mut map: F) -> Option<WidgetValue<R>>
    where
        F: for<'a> FnMut(&'a T) -> R + Send + 'static,
        R: Send + Sync,
    {
        match self {
            WidgetValue::Static(value) => Some(WidgetValue::Static(map(value))),
            WidgetValue::Value(value) => value.map_each(map).map(WidgetValue::Value),
        }
    }

    pub fn for_each<F>(&self, mut map: F)
    where
        F: for<'a> FnMut(&'a T) + Send + 'static,
    {
        match self {
            WidgetValue::Static(value) => map(value),
            WidgetValue::Value(value) => value.for_each(map),
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
    by_id: HashMap<TypeId, AnyTransmogrifier<F>>,
}

impl<F> Default for Widgets<F>
where
    F: Frontend,
{
    fn default() -> Self {
        Self {
            by_id: HashMap::default(),
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
        self.by_id.insert(
            TypeId::of::<T::Widget>(),
            AnyTransmogrifier::new(T::default()),
        );
    }

    pub fn with<T>(mut self) -> Self
    where
        T: WidgetTransmogrifier<F> + Transmogrify<F> + Default,
    {
        self.add::<T>();
        self
    }

    pub fn instantiate(
        &self,
        widget: &dyn AnyWidget,
        style: Value<Style>,
        context: &F::Context,
    ) -> F::Instance {
        let Some(transmogrifier) = self.by_id.get(&widget.widget_type_id()) else { unreachable!("{} not registered", widget.name()) };
        transmogrifier.0.transmogrify(widget, style, context)
    }

    pub fn get<T>(&self) -> Option<&AnyTransmogrifier<F>>
    where
        T: Widget,
    {
        self.by_id.get(&TypeId::of::<T>())
    }
}

impl<F> Debug for Widgets<F>
where
    F: Frontend,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut tuple = f.debug_tuple("WebWidgets");
        for transmogrifier in self.by_id.values() {
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
        style: Value<Style>,
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
        style: Value<Style>,
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
        style: Value<Style>,
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
    context: ActiveContext,
    children: OrderedLots<Child>,
}

impl Children {
    pub fn new(context: &ActiveContext) -> Self {
        Self {
            context: context.clone(),
            children: OrderedLots::new(),
        }
    }

    pub fn with_widget<NewWidget>(self, widget: NewWidget) -> Self
    where
        NewWidget: IntoNewWidget,
    {
        self.with(|_| widget)
    }

    pub fn with_named_widget<NewWidget>(self, name: Name, widget: NewWidget) -> Self
    where
        NewWidget: IntoNewWidget,
    {
        self.with_named(name, |_| widget)
    }

    pub fn with<NewWidget, WidgetFn>(mut self, widget_fn: WidgetFn) -> Self
    where
        WidgetFn: FnOnce(ActiveContext) -> NewWidget,
        NewWidget: IntoNewWidget,
    {
        self.push(widget_fn);
        self
    }

    pub fn with_named<NewWidget, WidgetFn>(mut self, name: Name, widget_fn: WidgetFn) -> Self
    where
        WidgetFn: FnOnce(ActiveContext) -> NewWidget,
        NewWidget: IntoNewWidget,
    {
        self.push_named(name, widget_fn);
        self
    }

    pub fn push<NewWidget, WidgetFn>(&mut self, widget_fn: WidgetFn)
    where
        WidgetFn: FnOnce(ActiveContext) -> NewWidget,
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
        WidgetFn: FnOnce(ActiveContext) -> NewWidget,
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
        WidgetFn: FnOnce(ActiveContext) -> NewWidget,
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
        WidgetFn: FnOnce(ActiveContext) -> NewWidget,
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

    pub fn entries(&self) -> alot::ordered::EntryIter<'_, Child> {
        self.children.entries()
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

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
    let context = ActiveContext {
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
