use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use crate::names::Name;
use crate::utils::Lazy;

#[derive(Clone, Debug, Default)]
pub struct Styles(Arc<HashMap<Group, HashMap<Name, Component>>>);

impl Styles {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, name: &impl NamedComponent, component: impl Into<Component>) {
        let name = name.name().into_owned();
        Arc::make_mut(&mut self.0)
            .entry(name.group)
            .or_default()
            .insert(name.name, component.into());
    }

    #[must_use]
    pub fn with(mut self, name: &impl NamedComponent, component: impl Into<Component>) -> Self {
        self.push(name, component);
        self
    }

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

    #[must_use]
    pub fn get_or_default<Named>(&self, component: &Named) -> Named::ComponentType
    where
        Named: ComponentDefinition + ?Sized,
    {
        let name = component.name();
        self.0
            .get(&name.group)
            .and_then(|group| group.get(&name.name))
            .and_then(|component| component.clone().try_into().ok())
            .unwrap_or_else(|| component.default_value())
    }
}

pub type StyleQuery = Vec<ComponentName>;
use std::any::Any;
use std::fmt::Debug;
use std::panic::{RefUnwindSafe, UnwindSafe};

use kludgine::figures::units::{Lp, Px};
use kludgine::figures::ScreenScale;
use kludgine::Color;

#[derive(Debug, Clone)]
pub enum Component {
    Color(Color),
    Dimension(Dimension),
    Percent(f32),
    Boxed(BoxedComponent),
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

#[derive(Debug, Clone, Copy)]
pub enum Dimension {
    Px(Px),
    Lp(Lp),
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

#[derive(Debug, Clone)]
pub struct BoxedComponent(Arc<dyn AnyComponent>);

impl BoxedComponent {
    pub fn new<T>(value: T) -> Self
    where
        T: RefUnwindSafe + UnwindSafe + Debug + Send + Sync + 'static,
    {
        Self(Arc::new(value))
    }

    #[must_use]
    pub fn downcast<T>(&self) -> Option<&T>
    where
        T: Debug + Send + Sync + 'static,
    {
        self.0.as_ref().as_any().downcast_ref()
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Group(Name);

impl Group {
    #[must_use]
    pub fn new<T>() -> Self
    where
        T: ComponentGroup,
    {
        Self(T::name())
    }

    #[must_use]
    pub fn matches<T>(&self) -> bool
    where
        T: ComponentGroup,
    {
        self.0 == T::name()
    }
}

pub trait ComponentGroup {
    fn name() -> Name;
}

pub enum Global {}

impl ComponentGroup for Global {
    fn name() -> Name {
        Name::new("global")
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ComponentName {
    pub group: Group,
    pub name: Name,
}

impl ComponentName {
    pub fn new(group: Group, name: impl Into<Name>) -> Self {
        Self {
            group,
            name: name.into(),
        }
    }

    pub fn named<G: ComponentGroup>(name: impl Into<Name>) -> Self {
        Self::new(Group::new::<G>(), name)
    }
}

impl From<&'static Lazy<ComponentName>> for ComponentName {
    fn from(value: &'static Lazy<ComponentName>) -> Self {
        (**value).clone()
    }
}
pub trait NamedComponent {
    fn name(&self) -> Cow<'_, ComponentName>;
}

pub trait ComponentDefinition: NamedComponent {
    type ComponentType: Into<Component> + TryFrom<Component, Error = Component>;

    fn default_value(&self) -> Self::ComponentType;
}

pub trait ComponentDefaultvalue: NamedComponent {
    fn default_component_value(&self) -> Component;
}

impl<T> ComponentDefaultvalue for T
where
    T: ComponentDefinition,
{
    fn default_component_value(&self) -> Component {
        self.default_value().into()
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

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct TextSize;

impl NamedComponent for TextSize {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("text_size"))
    }
}

impl ComponentDefinition for TextSize {
    type ComponentType = Dimension;

    fn default_value(&self) -> Dimension {
        Dimension::Lp(Lp::points(12))
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct LineHeight;

impl NamedComponent for LineHeight {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("line_height"))
    }
}

impl ComponentDefinition for LineHeight {
    type ComponentType = Dimension;

    fn default_value(&self) -> Dimension {
        Dimension::Lp(Lp::points(14))
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct TextColor;

impl NamedComponent for TextColor {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("text_color"))
    }
}

impl ComponentDefinition for TextColor {
    type ComponentType = Color;

    fn default_value(&self) -> Color {
        Color::WHITE
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct HighlightColor;

impl NamedComponent for HighlightColor {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("highlight_color"))
    }
}

impl ComponentDefinition for HighlightColor {
    type ComponentType = Color;

    fn default_value(&self) -> Color {
        Color::AQUA
    }
}
