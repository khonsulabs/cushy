//! Types for styling widgets.

use std::borrow::Cow;
use std::collections::{hash_map, HashMap};
use std::sync::Arc;

use crate::names::Name;
use crate::utils::Lazy;

pub mod components;

/// A collection of style components organized by their name.
#[derive(Clone, Debug, Default)]
pub struct Styles(Arc<HashMap<Group, HashMap<Name, Component>>>);

impl Styles {
    /// Returns an empty collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a collection with the capacity to hold up to `capacity` elements
    /// without reallocating.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Arc::new(HashMap::with_capacity(capacity)))
    }

    /// Inserts a [`Component`] with a given name.
    pub fn insert_named(&mut self, name: ComponentName, component: impl Into<Component>) {
        Arc::make_mut(&mut self.0)
            .entry(name.group)
            .or_default()
            .insert(name.name, component.into());
    }

    /// Inserts a [`Component`] using then name provided.
    pub fn insert(&mut self, name: &impl NamedComponent, component: impl Into<Component>) {
        let name = name.name().into_owned();
        self.insert_named(name, component);
    }

    /// Adds a [`Component`] for the name provided and returns self.
    #[must_use]
    pub fn with(mut self, name: &impl NamedComponent, component: impl Into<Component>) -> Self {
        self.insert(name, component);
        self
    }

    /// Returns the associated component for the given name, if found.
    #[must_use]
    pub fn get<Named>(&self, component: &Named) -> Option<&Component>
    where
        Named: NamedComponent + ?Sized,
    {
        let name = component.name();
        self.0
            .get(&name.group)
            .and_then(|group| group.get(&name.name))
    }

    /// Returns the component associated with the given name, or if not found,
    /// returns the default value provided by the definition.
    #[must_use]
    pub fn get_or_default<Named>(&self, component: &Named) -> Named::ComponentType
    where
        Named: ComponentDefinition + ?Sized,
    {
        let name = component.name();
        self.0
            .get(&name.group)
            .and_then(|group| group.get(&name.name))
            .and_then(|component| {
                <Named::ComponentType>::try_from_component(component.clone()).ok()
            })
            .unwrap_or_else(|| component.default_value())
    }
}

impl FromIterator<(ComponentName, Component)> for Styles {
    fn from_iter<T: IntoIterator<Item = (ComponentName, Component)>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut styles = Self::with_capacity(iter.size_hint().0);
        for (name, component) in iter {
            styles.insert_named(name, component);
        }
        styles
    }
}

impl IntoIterator for Styles {
    type IntoIter = StylesIntoIter;
    type Item = (ComponentName, Component);

    fn into_iter(self) -> Self::IntoIter {
        StylesIntoIter {
            main: Arc::try_unwrap(self.0)
                .unwrap_or_else(|err| err.as_ref().clone())
                .into_iter(),
            names: None,
        }
    }
}

/// An iterator over the owned contents of a [`Styles`] instance.
pub struct StylesIntoIter {
    main: hash_map::IntoIter<Group, HashMap<Name, Component>>,
    names: Option<(Group, hash_map::IntoIter<Name, Component>)>,
}

impl Iterator for StylesIntoIter {
    type Item = (ComponentName, Component);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((group, names)) = &mut self.names {
                if let Some((name, component)) = names.next() {
                    return Some((ComponentName::new(group.clone(), name), component));
                }
                self.names = None;
            }

            let (group, names) = self.main.next()?;
            self.names = Some((group, names.into_iter()));
        }
    }
}

use std::any::Any;
use std::fmt::Debug;
use std::panic::{RefUnwindSafe, UnwindSafe};

use kludgine::figures::units::{Lp, Px};
use kludgine::figures::ScreenScale;
use kludgine::Color;

/// A value of a style component.
#[derive(Debug, Clone)]
pub enum Component {
    /// A color.
    Color(Color),
    /// A single-dimension measurement.
    Dimension(Dimension),
    /// A percentage between 0.0 and 1.0.
    Percent(f32),
    /// A custom component type.
    Custom(CustomComponent),
}

impl From<Color> for Component {
    fn from(value: Color) -> Self {
        Self::Color(value)
    }
}

impl TryFrom<Component> for Color {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::Color(color) => Ok(color),
            other => Err(other),
        }
    }
}

impl From<Dimension> for Component {
    fn from(value: Dimension) -> Self {
        Self::Dimension(value)
    }
}

impl TryFrom<Component> for Dimension {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::Dimension(color) => Ok(color),
            other => Err(other),
        }
    }
}

impl From<Px> for Component {
    fn from(value: Px) -> Self {
        Self::from(Dimension::from(value))
    }
}

impl TryFrom<Component> for Px {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::Dimension(Dimension::Px(px)) => Ok(px),
            other => Err(other),
        }
    }
}

impl From<Lp> for Component {
    fn from(value: Lp) -> Self {
        Self::from(Dimension::from(value))
    }
}

impl TryFrom<Component> for Lp {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::Dimension(Dimension::Lp(px)) => Ok(px),
            other => Err(other),
        }
    }
}

/// A 1-dimensional measurement.
#[derive(Debug, Clone, Copy)]
pub enum Dimension {
    /// Physical Pixels
    Px(Px),
    /// Logical Pixels
    Lp(Lp),
}

impl Default for Dimension {
    fn default() -> Self {
        Self::Px(Px(0))
    }
}

impl From<Px> for Dimension {
    fn from(value: Px) -> Self {
        Self::Px(value)
    }
}

impl From<Lp> for Dimension {
    fn from(value: Lp) -> Self {
        Self::Lp(value)
    }
}

impl ScreenScale for Dimension {
    type Lp = Lp;
    type Px = Px;

    fn into_px(self, scale: kludgine::figures::Fraction) -> Px {
        match self {
            Dimension::Px(px) => px,
            Dimension::Lp(lp) => lp.into_px(scale),
        }
    }

    fn from_px(px: Px, _scale: kludgine::figures::Fraction) -> Self {
        Self::from(px)
    }

    fn into_lp(self, scale: kludgine::figures::Fraction) -> Lp {
        match self {
            Dimension::Px(px) => px.into_lp(scale),
            Dimension::Lp(lp) => lp,
        }
    }

    fn from_lp(lp: Lp, _scale: kludgine::figures::Fraction) -> Self {
        Self::from(lp)
    }
}

/// A custom component value.
#[derive(Debug, Clone)]
pub struct CustomComponent(Arc<dyn AnyComponent>);

impl CustomComponent {
    /// Wraps an arbitrary value so that it can be used as a [`Component`].
    pub fn new<T>(value: T) -> Self
    where
        T: RefUnwindSafe + UnwindSafe + Debug + Send + Sync + 'static,
    {
        Self(Arc::new(value))
    }

    /// Return the contained value cast as `T`. Returns `None` if `T` does is
    /// not the same type that was provided when this component was created.
    #[must_use]
    pub fn downcast<T>(&self) -> Option<&T>
    where
        T: Debug + Send + Sync + 'static,
    {
        self.0.as_ref().as_any().downcast_ref()
    }
}

impl ComponentType for CustomComponent {
    fn into_component(self) -> Component {
        Component::Custom(self)
    }

    fn try_from_component(component: Component) -> Result<Self, Component> {
        match component {
            Component::Custom(custom) => Ok(custom),
            other => Err(other),
        }
    }
}

trait AnyComponent: Send + Sync + RefUnwindSafe + UnwindSafe + Debug {
    fn as_any(&self) -> &dyn Any;
}

impl<T> AnyComponent for T
where
    T: RefUnwindSafe + UnwindSafe + Debug + Send + Sync + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A style component group.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Group(Name);

impl Group {
    /// Returns a new instance using the group name of `T`.
    #[must_use]
    pub fn new<T>() -> Self
    where
        T: ComponentGroup,
    {
        Self(T::name())
    }

    /// Returns true if this instance matches the group name of `T`.
    #[must_use]
    pub fn matches<T>(&self) -> bool
    where
        T: ComponentGroup,
    {
        self.0 == T::name()
    }
}

/// A type that represents a group of style components.
pub trait ComponentGroup {
    /// Returns the name of the group.
    fn name() -> Name;
}

/// The Global style components group.
pub enum Global {}

impl ComponentGroup for Global {
    fn name() -> Name {
        Name::new("global")
    }
}

/// A fully-qualified style component name.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ComponentName {
    /// The group name.
    pub group: Group,
    /// The name of the component within the group.
    pub name: Name,
}

impl ComponentName {
    /// Returns a new instance using `group` and `name`.
    pub fn new(group: Group, name: impl Into<Name>) -> Self {
        Self {
            group,
            name: name.into(),
        }
    }

    /// Returns a new instance using `G` and `name`.
    pub fn named<G: ComponentGroup>(name: impl Into<Name>) -> Self {
        Self::new(Group::new::<G>(), name)
    }
}

impl From<&'static Lazy<ComponentName>> for ComponentName {
    fn from(value: &'static Lazy<ComponentName>) -> Self {
        (**value).clone()
    }
}

/// A type that represents a named style component.
pub trait NamedComponent {
    /// Returns the name of the style component.
    fn name(&self) -> Cow<'_, ComponentName>;
}

/// A type that represents a named component with a default value of a specific
/// Rust type.
pub trait ComponentDefinition: NamedComponent {
    /// The type that will be contained in the [`Component`].
    type ComponentType: ComponentType;

    /// Returns the default value to use for this component.
    fn default_value(&self) -> Self::ComponentType;
}

/// A type that can be converted to and from [`Component`].
pub trait ComponentType: Sized {
    /// Returns this type, wrapped in a [`Component`].
    fn into_component(self) -> Component;
    /// Attempts to extract this type from `component`. If `component` does not
    /// contain this type, `Err(component)` is returned.
    fn try_from_component(component: Component) -> Result<Self, Component>;
}

impl<T> ComponentType for T
where
    T: Into<Component> + TryFrom<Component, Error = Component>,
{
    fn into_component(self) -> Component {
        self.into()
    }

    fn try_from_component(component: Component) -> Result<Self, Component> {
        Self::try_from(component)
    }
}

/// A type that represents a named component with a default value.
pub trait ComponentDefaultvalue: NamedComponent {
    /// Returns the default value for this component.
    fn default_component_value(&self) -> Component;
}

impl<T> ComponentDefaultvalue for T
where
    T: ComponentDefinition,
{
    fn default_component_value(&self) -> Component {
        self.default_value().into_component()
    }
}

impl NamedComponent for ComponentName {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Borrowed(self)
    }
}

impl NamedComponent for Cow<'_, ComponentName> {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Borrowed(self)
    }
}
