//! Types for styling widgets.

use std::any::Any;
use std::borrow::Cow;
use std::collections::{hash_map, HashMap};
use std::fmt::Debug;
use std::ops::{
    Add, Bound, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::Arc;

use kludgine::figures::units::{Lp, Px, UPx};
use kludgine::figures::{Fraction, IntoUnsigned, ScreenScale, Size};
use kludgine::Color;
use palette::{IntoColor, Okhsl, OklabHue, Srgb};

use crate::animation::{EasingFunction, ZeroToOne};
use crate::context::WidgetContext;
use crate::names::Name;
use crate::styles::components::{FocusableWidgets, VisualOrder};
use crate::utils::Lazy;
use crate::value::{Dynamic, IntoValue, Value};

#[macro_use]
pub mod components;

/// A collection of style components organized by their name.
#[derive(Clone, Debug, Default)]
pub struct Styles(Arc<HashMap<Group, HashMap<Name, Value<Component>>>>);

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
    pub fn insert_named(&mut self, name: ComponentName, component: impl IntoComponentValue) {
        Arc::make_mut(&mut self.0)
            .entry(name.group)
            .or_default()
            .insert(name.name, component.into_component_value());
    }

    /// Inserts a [`Component`] using then name provided.
    pub fn insert(&mut self, name: &impl NamedComponent, component: impl IntoComponentValue) {
        let name = name.name().into_owned();
        self.insert_named(name, component);
    }

    /// Adds a [`Component`] for the name provided and returns self.
    #[must_use]
    pub fn with(mut self, name: &impl NamedComponent, component: impl IntoComponentValue) -> Self {
        self.insert(name, component);
        self
    }

    /// Returns the associated component for the given name, if found.
    #[must_use]
    pub fn get_named<Named>(&self, component: &Named) -> Option<&Value<Component>>
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
    pub fn get<Named>(
        &self,
        component: &Named,
        context: &WidgetContext<'_, '_>,
    ) -> Named::ComponentType
    where
        Named: ComponentDefinition + ?Sized,
    {
        let name = component.name();
        self.0
            .get(&name.group)
            .and_then(|group| group.get(&name.name))
            .and_then(|component| {
                component.redraw_when_changed(context);
                <Named::ComponentType>::try_from_component(component.get()).ok()
            })
            .unwrap_or_else(|| component.default_value(context))
    }
}

/// A value that can be converted into a `Value<Component>`.
pub trait IntoComponentValue {
    /// Returns `self` stored in a component value.
    fn into_component_value(self) -> Value<Component>;
}

impl<T> IntoComponentValue for T
where
    T: Into<Component>,
{
    fn into_component_value(self) -> Value<Component> {
        Value::Constant(self.into())
    }
}

impl IntoComponentValue for Value<Component> {
    fn into_component_value(self) -> Value<Component> {
        self
    }
}

impl<T> IntoComponentValue for Dynamic<T>
where
    T: Clone,
    Component: From<T>,
{
    fn into_component_value(self) -> Value<Component> {
        Value::Dynamic(self.map_each_into())
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
    type Item = (ComponentName, Value<Component>);

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
    main: hash_map::IntoIter<Group, HashMap<Name, Value<Component>>>,
    names: Option<(Group, hash_map::IntoIter<Name, Value<Component>>)>,
}

impl Iterator for StylesIntoIter {
    type Item = (ComponentName, Value<Component>);

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

/// A value of a style component.
#[derive(Debug, Clone)]
pub enum Component {
    /// A color.
    Color(Color),
    /// A single-dimension measurement.
    Dimension(Dimension),
    /// A single-dimension measurement.
    DimensionRange(DimensionRange),
    /// A percentage between 0.0 and 1.0.
    Percent(ZeroToOne),
    /// A custom component type.
    Custom(CustomComponent),
    /// An easing function for animations.
    Easing(EasingFunction),
    /// A visual ordering to use for layout.
    VisualOrder(VisualOrder),
    /// A description of what widgets should be focusable.
    FocusableWidgets(FocusableWidgets),
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

/// A 1-dimensional measurement that may be automatically calculated.
#[derive(Debug, Clone, Copy)]
pub enum FlexibleDimension {
    /// Automatically calculate this dimension.
    Auto,
    /// Use this dimension.
    Dimension(Dimension),
}

impl FlexibleDimension {
    /// A dimension of 0 pixels.
    pub const ZERO: Self = Self::Dimension(Dimension::ZERO);
}

impl Default for FlexibleDimension {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<Dimension> for FlexibleDimension {
    fn from(dimension: Dimension) -> Self {
        Self::Dimension(dimension)
    }
}

impl From<Px> for FlexibleDimension {
    fn from(value: Px) -> Self {
        Self::from(Dimension::from(value))
    }
}

impl From<Lp> for FlexibleDimension {
    fn from(value: Lp) -> Self {
        Self::from(Dimension::from(value))
    }
}

/// A 1-dimensional measurement.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Dimension {
    /// Physical Pixels
    Px(Px),
    /// Logical Pixels
    Lp(Lp),
}

impl Dimension {
    /// A dimension of 0 pixels.
    pub const ZERO: Self = Self::Px(Px(0));
}

impl Default for Dimension {
    fn default() -> Self {
        Self::ZERO
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

/// A range of [`Dimension`]s.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DimensionRange {
    /// The start bound of the range.
    pub start: Bound<Dimension>,
    /// The end bound of the range.
    pub end: Bound<Dimension>,
}

impl DimensionRange {
    /// Returns this range's dimension if the range represents a single
    /// dimension.
    #[must_use]
    pub fn exact_dimension(&self) -> Option<Dimension> {
        match (self.start, self.end) {
            (Bound::Excluded(start), Bound::Included(end)) if start == end => Some(start),
            _ => None,
        }
    }

    /// Clamps `size` to the dimensions of this range, converting to unsigned
    /// pixels in the process.
    #[must_use]
    pub fn clamp(&self, mut size: UPx, scale: Fraction) -> UPx {
        if let Some(min) = self.minimum() {
            size = size.max(min.into_px(scale).into_unsigned());
        }
        if let Some(max) = self.maximum() {
            size = size.min(max.into_px(scale).into_unsigned());
        }
        size
    }

    /// Returns the minimum measurement, if the start is bounded.
    #[must_use]
    pub fn minimum(&self) -> Option<Dimension> {
        match self.start {
            Bound::Unbounded => None,
            Bound::Excluded(Dimension::Lp(lp)) => Some(Dimension::Lp(lp + 1)),
            Bound::Excluded(Dimension::Px(px)) => Some(Dimension::Px(px + 1)),
            Bound::Included(value) => Some(value),
        }
    }

    /// Returns the maximum measurement, if the end is bounded.
    #[must_use]
    pub fn maximum(&self) -> Option<Dimension> {
        match self.end {
            Bound::Unbounded => None,
            Bound::Excluded(Dimension::Lp(lp)) => Some(Dimension::Lp(lp - 1)),
            Bound::Excluded(Dimension::Px(px)) => Some(Dimension::Px(px - 1)),
            Bound::Included(value) => Some(value),
        }
    }
}

impl<T> From<T> for DimensionRange
where
    T: Into<Dimension>,
{
    fn from(value: T) -> Self {
        let dimension = value.into();
        Self::from(dimension..=dimension)
    }
}

impl<T> From<Range<T>> for DimensionRange
where
    T: Into<Dimension>,
{
    fn from(value: Range<T>) -> Self {
        Self {
            start: Bound::Included(value.start.into()),
            end: Bound::Excluded(value.end.into()),
        }
    }
}

impl From<RangeFull> for DimensionRange {
    fn from(_: RangeFull) -> Self {
        Self {
            start: Bound::Unbounded,
            end: Bound::Unbounded,
        }
    }
}

impl<T> From<RangeInclusive<T>> for DimensionRange
where
    T: Into<Dimension> + Clone,
{
    fn from(value: RangeInclusive<T>) -> Self {
        Self {
            start: Bound::Included(value.start().clone().into()),
            end: Bound::Included(value.end().clone().into()),
        }
    }
}

impl<T> From<RangeFrom<T>> for DimensionRange
where
    T: Into<Dimension>,
{
    fn from(value: RangeFrom<T>) -> Self {
        Self {
            start: Bound::Included(value.start.into()),
            end: Bound::Unbounded,
        }
    }
}

impl<T> From<RangeTo<T>> for DimensionRange
where
    T: Into<Dimension>,
{
    fn from(value: RangeTo<T>) -> Self {
        Self {
            start: Bound::Unbounded,
            end: Bound::Excluded(value.end.into()),
        }
    }
}

impl<T> From<RangeToInclusive<T>> for DimensionRange
where
    T: Into<Dimension>,
{
    fn from(value: RangeToInclusive<T>) -> Self {
        Self {
            start: Bound::Unbounded,
            end: Bound::Included(value.end.into()),
        }
    }
}

impl From<DimensionRange> for Component {
    fn from(value: DimensionRange) -> Self {
        Component::DimensionRange(value)
    }
}

impl TryFrom<Component> for DimensionRange {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::DimensionRange(value) => Ok(value),
            other => Err(other),
        }
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
    /// Returns a new group with `name`.
    #[must_use]
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self(Name::new(name))
    }

    /// Returns a new instance using the group name of `T`.
    #[must_use]
    pub fn from_group<T>() -> Self
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
        Self::new(Group::from_group::<G>(), name)
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
    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Self::ComponentType;
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
    fn default_component_value(&self, context: &WidgetContext<'_, '_>) -> Component;
}

impl<T> ComponentDefaultvalue for T
where
    T: ComponentDefinition,
{
    fn default_component_value(&self, context: &WidgetContext<'_, '_>) -> Component {
        self.default_value(context).into_component()
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

/// A type describing characteristics about the edges of a rectangle.
#[derive(Clone, Copy, Debug)]
pub struct Edges<T = FlexibleDimension> {
    /// The left edge
    pub left: T,
    /// The right edge
    pub right: T,
    /// The top edge
    pub top: T,
    /// The bottom edge
    pub bottom: T,
}

impl<T> Edges<T> {
    /// Returns the sum of the parts as a [`Size`].
    pub fn size(&self) -> Size<T>
    where
        T: Add<Output = T> + Copy,
    {
        Size::new(self.left + self.right, self.top + self.bottom)
    }
}

impl<T> Default for Edges<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            left: T::default(),
            right: T::default(),
            top: T::default(),
            bottom: Default::default(),
        }
    }
}

impl<T> Edges<T> {
    /// Updates `top` and returns self.
    #[must_use]
    pub fn with_top(mut self, top: impl Into<T>) -> Self {
        self.top = top.into();
        self
    }

    /// Updates `bottom` and returns self.
    #[must_use]
    pub fn with_bottom(mut self, bottom: impl Into<T>) -> Self {
        self.bottom = bottom.into();
        self
    }

    /// Updates `right` and returns self.
    #[must_use]
    pub fn with_right(mut self, right: impl Into<T>) -> Self {
        self.right = right.into();
        self
    }

    /// Updates `left` and returns self.
    #[must_use]
    pub fn with_left(mut self, left: impl Into<T>) -> Self {
        self.left = left.into();
        self
    }

    /// Updates left and right to be `horizontal` and returns self.
    #[must_use]
    pub fn with_horizontal(mut self, horizontal: impl Into<T>) -> Self
    where
        T: Clone,
    {
        self.left = horizontal.into();
        self.right = self.left.clone();
        self
    }

    /// Updates top and bottom to be `vertical` and returns self.
    #[must_use]
    pub fn with_vertical(mut self, vertical: impl Into<T>) -> Self
    where
        T: Clone,
    {
        self.top = vertical.into();
        self.bottom = self.top.clone();
        self
    }
}

impl Edges<Dimension> {
    /// Returns a new instance with `dimension` for every edge.
    #[must_use]
    pub fn uniform<D>(dimension: D) -> Self
    where
        D: Into<Dimension>,
    {
        let dimension = dimension.into();
        Self::from(dimension)
    }
}

impl<T> From<T> for Edges<T>
where
    T: Clone,
{
    fn from(value: T) -> Self {
        Self {
            left: value.clone(),
            right: value.clone(),
            top: value.clone(),
            bottom: value,
        }
    }
}

impl IntoValue<Edges<FlexibleDimension>> for FlexibleDimension {
    fn into_value(self) -> Value<Edges<FlexibleDimension>> {
        Value::Constant(Edges::from(self))
    }
}

impl IntoValue<Edges<Dimension>> for Dimension {
    fn into_value(self) -> Value<Edges<Dimension>> {
        Value::Constant(Edges::from(self))
    }
}

/// A set of light and dark [`Theme`]s.
#[derive(Clone, Debug)]
pub struct ThemePair {
    /// The theme to use when the user interface is in light mode.
    pub light: Theme,
    /// The theme to use when the user interface is in dark mode.
    pub dark: Theme,
    /// A theme of the primary color that remains consistent between dark and
    /// light theme variants.
    pub primary_fixed: FixedTheme,
    /// A theme of the secondary color that remains consistent between dark and
    /// light theme variants.
    pub secondary_fixed: FixedTheme,
    /// A theme of the tertiary color that remains consistent between dark and
    /// light theme variants.
    pub tertiary_fixed: FixedTheme,

    /// A color to apply to scrims, a term sometimes used to refer to the
    /// translucent backdrop placed behind a modal popup.
    pub scrim: Color,

    /// A color to apply to shadows.
    pub shadow: Color,
}

impl ThemePair {
    /// Returns a new theme generated from the provided color sources.
    #[must_use]
    pub fn from_sources(
        primary: ColorSource,
        secondary: ColorSource,
        tertiary: ColorSource,
        error: ColorSource,
        neutral: ColorSource,
        neutral_variant: ColorSource,
    ) -> Self {
        Self {
            light: Theme::light_from_sources(
                primary,
                secondary,
                tertiary,
                error,
                neutral,
                neutral_variant,
            ),
            dark: Theme::dark_from_sources(
                primary,
                secondary,
                tertiary,
                error,
                neutral,
                neutral_variant,
            ),
            primary_fixed: FixedTheme::from_source(primary),
            secondary_fixed: FixedTheme::from_source(secondary),
            tertiary_fixed: FixedTheme::from_source(tertiary),
            scrim: neutral.color(1),
            shadow: neutral.color(1),
        }
    }
}

impl Default for ThemePair {
    fn default() -> Self {
        const PRIMARY_HUE: f32 = -120.;
        const SECONDARY_HUE: f32 = 0.;
        const TERTIARY_HUE: f32 = -30.;
        const ERROR_HUE: f32 = 30.;
        Self::from_sources(
            ColorSource::new(PRIMARY_HUE, 0.8),
            ColorSource::new(SECONDARY_HUE, 0.3),
            ColorSource::new(TERTIARY_HUE, 0.3),
            ColorSource::new(ERROR_HUE, 0.8),
            ColorSource::new(0., 0.001),
            ColorSource::new(30., 0.),
        )
    }
}

/// A Gooey Color theme.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Theme {
    /// The primary color theme.
    pub primary: ColorTheme,
    /// The secondary color theme.
    pub secondary: ColorTheme,
    /// The tertiary color theme.
    pub tertiary: ColorTheme,
    /// The color theme for errors.
    pub error: ColorTheme,

    /// The theme to color surfaces.
    pub surface: SurfaceTheme,
}

impl Theme {
    /// Returns a new light theme generated from the provided color sources.
    #[must_use]
    pub fn light_from_sources(
        primary: ColorSource,
        secondary: ColorSource,
        tertiary: ColorSource,
        error: ColorSource,
        neutral: ColorSource,
        neutral_variant: ColorSource,
    ) -> Self {
        Self {
            primary: ColorTheme::light_from_source(primary),
            secondary: ColorTheme::light_from_source(secondary),
            tertiary: ColorTheme::light_from_source(tertiary),
            error: ColorTheme::light_from_source(error),
            surface: SurfaceTheme::light_from_sources(neutral, neutral_variant),
        }
    }

    /// Returns a new dark theme generated from the provided color sources.
    #[must_use]
    pub fn dark_from_sources(
        primary: ColorSource,
        secondary: ColorSource,
        tertiary: ColorSource,
        error: ColorSource,
        neutral: ColorSource,
        neutral_variant: ColorSource,
    ) -> Self {
        Self {
            primary: ColorTheme::dark_from_source(primary),
            secondary: ColorTheme::dark_from_source(secondary),
            tertiary: ColorTheme::dark_from_source(tertiary),
            error: ColorTheme::dark_from_source(error),
            surface: SurfaceTheme::dark_from_sources(neutral, neutral_variant),
        }
    }
}

/// A theme of surface colors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceTheme {
    /// The default background color.
    pub color: Color,
    /// A dimmer variant of the default background color.
    pub dim_color: Color,
    /// A brighter variant of the default background color.
    pub bright_color: Color,

    /// The background color to use for the lowest level container widget.
    pub lowest_container: Color,
    /// The background color to use for the low level container widgets.
    pub low_container: Color,
    /// The background color for middle-level container widgets.
    pub container: Color,
    /// The background color for high-level container widgets.
    pub high_container: Color,
    /// The background color for highest-level container widgets.
    pub highest_container: Color,

    /// The default background color for widgets that are opaque.
    pub opaque_widget: Color,

    /// The default text/content color.
    pub on_color: Color,
    /// A variation of the text/content color that is de-emphasized.
    pub on_color_variant: Color,
    /// The color to draw important outlines.
    pub outline: Color,
    /// The color to use for decorative outlines.
    pub outline_variant: Color,
}

impl SurfaceTheme {
    /// Returns a new light surface theme generated from the two neutral color
    /// sources.
    #[must_use]
    pub fn light_from_sources(neutral: ColorSource, neutral_variant: ColorSource) -> Self {
        Self {
            color: neutral.color(97),
            dim_color: neutral.color(70),
            bright_color: neutral.color(99),
            opaque_widget: neutral_variant.color(75),
            lowest_container: neutral.color(95),
            low_container: neutral.color(92),
            container: neutral.color(90),
            high_container: neutral.color(85),
            highest_container: neutral.color(80),
            on_color: neutral.color(10),
            on_color_variant: neutral_variant.color(30),
            outline: neutral_variant.color(50),
            outline_variant: neutral.color(60),
        }
    }

    /// Returns a new dark surface theme generated from the two neutral color
    /// sources.
    #[must_use]
    pub fn dark_from_sources(neutral: ColorSource, neutral_variant: ColorSource) -> Self {
        Self {
            color: neutral.color(10),
            dim_color: neutral.color(2),
            bright_color: neutral.color(11),
            opaque_widget: neutral_variant.color(40),
            lowest_container: neutral.color(15),
            low_container: neutral.color(20),
            container: neutral.color(25),
            high_container: neutral.color(30),
            highest_container: neutral.color(35),
            on_color: neutral.color(90),
            on_color_variant: neutral_variant.color(70),
            outline: neutral_variant.color(60),
            outline_variant: neutral.color(50),
        }
    }
}

/// A pallete of a shared [`ColorSource`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ColorTheme {
    /// The primary color, used for high-emphasis content.
    pub color: Color,
    /// The primary color, dimmed for de-emphasized or disabled content.
    pub color_dim: Color,
    /// The color for content that sits atop the primary color.
    pub on_color: Color,
    /// The backgrond color for containers.
    pub container: Color,
    /// The color for content that is inside of a container.
    pub on_container: Color,
}

impl ColorTheme {
    /// Returns a new light color theme for `source`.
    #[must_use]
    pub fn light_from_source(source: ColorSource) -> Self {
        Self {
            color: source.color(40),
            color_dim: source.color(30),
            on_color: source.color(100),
            container: source.color(90),
            on_container: source.color(10),
        }
    }

    /// Returns a new dark color theme for `source`.
    #[must_use]
    pub fn dark_from_source(source: ColorSource) -> Self {
        Self {
            color: source.color(70),
            color_dim: source.color(60),
            on_color: source.color(10),
            container: source.color(30),
            on_container: source.color(90),
        }
    }
}

/// A theme of colors that is shared between light and dark theme variants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedTheme {
    /// An accent background color.
    pub color: Color,
    /// An alternate background color, for less emphasized content.
    pub dim_color: Color,
    /// The primary color for content on either background color in this theme.
    pub on_color: Color,
    /// The color for de-emphasized content on either background color in this
    /// theme.
    pub on_color_variant: Color,
}

impl FixedTheme {
    /// Returns a new color theme from `source` whose colors are safe in both
    /// light and dark themes.
    #[must_use]
    pub fn from_source(source: ColorSource) -> Self {
        Self {
            color: source.color(90),
            dim_color: source.color(80),
            on_color: source.color(10),
            on_color_variant: source.color(40),
        }
    }
}

/// A source for [`Color`]s.
///
/// This type is a combination of an [`OklabHue`] and a saturation ranging from
/// 0.0 to 1.0. When combined with a luminance value, a [`Color`] can be
/// generated.
///
/// The goal of this type is to allow various tones of a given hue/saturation to
/// be generated easily.
#[derive(Clone, Copy, Debug)]
pub struct ColorSource {
    /// A measurement of hue, in degees, from -180 to 180.
    ///
    /// For fully saturated bright colors:
    ///
    /// - 0° corresponds to a kind of magenta-pink (RBG #ff0188),
    /// - 90° to a kind of yellow (RBG RGB #ffcb00)
    /// - 180° to a kind of cyan (RBG #00ffe1) and
    /// - 240° to a kind of blue (RBG #00aefe).
    pub hue: OklabHue,
    /// A measurement of saturation.
    ///
    /// A saturation of 0.0 corresponds to shades of gray, while a saturation of
    /// 1.0 corresponds to fully saturated colors.
    pub saturation: ZeroToOne,
}

impl ColorSource {
    /// Returns a new source with the given hue (in degrees) and saturation (0.0
    /// - 1.0).
    #[must_use]
    pub fn new(hue: impl Into<OklabHue>, saturation: impl Into<ZeroToOne>) -> Self {
        Self {
            hue: hue.into(),
            saturation: saturation.into(),
        }
    }

    /// Generates a new color by combing the hue, saturation, and lightness.
    #[must_use]
    pub fn color(self, lightness: impl Lightness) -> Color {
        let rgb: palette::Srgb =
            Okhsl::new(self.hue, *self.saturation, *lightness.into_lightness()).into_color();
        Color::new_f32(rgb.red, rgb.blue, rgb.green, 1.0)
    }

    /// Calculates an approximate ratio between 0.0 and 1.0 of how contrasting
    /// these colors are, with perfect constrast being two clors that are
    /// opposite of each other on the hue circle and one fully desaturated and
    /// the other fully saturated.
    #[must_use]
    pub fn contrast_between(self, other: Self) -> ZeroToOne {
        let saturation_delta = self.saturation.difference_between(other.saturation);

        let self_hue = self.hue.into_positive_degrees();
        let other_hue = other.hue.into_positive_degrees();
        // Calculate the shortest distance between the hues, taking into account
        // that 0 and 359 are one degree apart.
        let hue_delta = ZeroToOne::new(
            if self_hue < other_hue {
                let hue_delta_a = other_hue - self_hue;
                let hue_delta_b = self_hue + 360. - other_hue;
                hue_delta_a.min(hue_delta_b)
            } else {
                let hue_delta_a = self_hue - other_hue;
                let hue_delta_b = other_hue + 360. - self_hue;
                hue_delta_a.min(hue_delta_b)
            } / 180.,
        );

        saturation_delta * hue_delta
    }
}

/// A value that can represent the lightness of a color.
///
/// This is implemented for these types:
///
/// - [`ZeroToOne`]: A range of 0.0 to 1.0.
/// - `f32`: Values are clamped to 0.0 and 1.0. Panics if NaN.
/// - `u8`: A range of 0 to 100. Values above 100 are clamped.
pub trait Lightness {
    /// Returns this value as a floating point clamped between 0 and 1.
    fn into_lightness(self) -> ZeroToOne;
}

impl Lightness for ZeroToOne {
    fn into_lightness(self) -> ZeroToOne {
        self
    }
}
impl Lightness for f32 {
    fn into_lightness(self) -> ZeroToOne {
        ZeroToOne::new(self)
    }
}

impl Lightness for u8 {
    fn into_lightness(self) -> ZeroToOne {
        ZeroToOne::new(f32::from(self) / 100.)
    }
}

/// Extra functionality added to the [`Color`] type from Kludgine.
pub trait ColorExt: Copy {
    /// Converts this color into its hue/saturation and lightness components.
    fn into_source_and_lightness(self) -> (ColorSource, ZeroToOne);

    /// Returns the hue and saturation of this color.
    fn source(self) -> ColorSource {
        self.into_source_and_lightness().0
    }

    /// Returns the perceived lightness of this color.
    #[must_use]
    fn lightness(self) -> ZeroToOne {
        self.into_source_and_lightness().1
    }

    /// Returns the contrast between this color and the components provided.
    ///
    /// To achieve a contrast of 1.0:
    ///
    /// - `self`'s hue and `check_source.hue` must be 180 degrees apart.
    /// - `self`'s saturation and `check_source.saturation` must be different by
    ///   1.0.
    /// - `self`'s lightness and `check_lightness` must be different by 1.0.
    /// - `self`'s alpha and `check_alpha` must be different by 1.0.
    ///
    /// The algorithm currently used is purposely left undocumented as it will
    /// likely change. It should be a reasonable heuristic until someone smarter
    /// than @ecton comes along.
    fn contrast_between(
        self,
        check_source: ColorSource,
        check_lightness: ZeroToOne,
        check_alpha: ZeroToOne,
    ) -> ZeroToOne;

    /// Returns the color in `others` that contrasts the most from `self`.
    #[must_use]
    fn most_contrasting(self, others: &[Self]) -> Self
    where
        Self: Copy;
}

impl ColorExt for Color {
    fn into_source_and_lightness(self) -> (ColorSource, ZeroToOne) {
        let hsl: palette::Okhsl =
            Srgb::new(self.red_f32(), self.green_f32(), self.blue_f32()).into_color();
        (
            ColorSource {
                hue: hsl.hue,
                saturation: ZeroToOne::new(hsl.saturation),
            },
            ZeroToOne::new(hsl.lightness * self.alpha_f32()),
        )
    }

    fn contrast_between(
        self,
        check_source: ColorSource,
        check_lightness: ZeroToOne,
        check_alpha: ZeroToOne,
    ) -> ZeroToOne {
        let (other_source, other_lightness) = self.into_source_and_lightness();
        let lightness_delta = other_lightness.difference_between(check_lightness);

        let source_change = check_source.contrast_between(other_source);

        let other_alpha = ZeroToOne::new(self.alpha_f32());
        let alpha_delta = check_alpha.difference_between(other_alpha);

        ZeroToOne::new((*lightness_delta + *source_change + *alpha_delta) / 3.)
    }

    fn most_contrasting(self, others: &[Self]) -> Self
    where
        Self: Copy,
    {
        let (check_source, check_lightness) = self.into_source_and_lightness();
        let check_alpha = ZeroToOne::new(self.alpha_f32());

        let mut others = others.iter().copied();
        let mut most_contrasting = others.next().expect("at least one comparison");
        let mut most_contrast_amount =
            most_contrasting.contrast_between(check_source, check_lightness, check_alpha);
        for other in others {
            let contrast_amount =
                other.contrast_between(check_source, check_lightness, check_alpha);
            if contrast_amount > most_contrast_amount {
                most_contrasting = other;
                most_contrast_amount = contrast_amount;
            }
        }

        most_contrasting
    }
}
