//! Types for styling widgets.

use std::any::Any;
use std::borrow::Cow;
use std::collections::hash_map;
use std::fmt::{Debug, Write};
use std::ops::{
    Add, AddAssign, Bound, Deref, Div, Mul, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo,
    RangeToInclusive,
};
use std::sync::Arc;

use ahash::AHashMap;
use figures::units::{Lp, Px, UPx};
use figures::{Fraction, IntoSigned, IntoUnsigned, Rect, Round, ScreenScale, Size, Zero};
use intentional::Cast;
pub use kludgine::cosmic_text::{FamilyOwned, Style, Weight};
pub use kludgine::shapes::CornerRadii;
pub use kludgine::Color;
pub use palette::OklabHue;
use palette::{IntoColor, Okhsl, Srgb};

use crate::animation::{EasingFunction, ZeroToOne};
use crate::context::{Trackable, WidgetContext};
use crate::names::Name;
use crate::utils::Lazy;
use crate::value::{Dynamic, IntoValue, Source, Value};
use crate::widget::MakeWidget;
use crate::widgets::input::CowString;
use crate::widgets::ComponentProbe;

#[macro_use]
pub mod components;

/// A collection of style components organized by their name.
#[derive(Clone, Default)]
pub struct Styles(Arc<StyleData>);

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
        Self(Arc::new(StyleData {
            components: AHashMap::with_capacity(capacity),
        }))
    }

    /// Inserts a [`Component`] with a given name.
    pub fn insert_named(&mut self, name: ComponentName, component: impl IntoStoredComponent) {
        Arc::make_mut(&mut self.0)
            .components
            .insert(name, component.into_stored_component());
    }

    /// Inserts a [`Component`] using then name provided.
    pub fn insert(&mut self, name: &impl NamedComponent, component: impl IntoComponentValue) {
        let name = name.name().into_owned();
        self.insert_named(name, component);
    }

    /// Inserts a [`Component`] using then name provided, resolving the value
    /// through `dynamic`.
    pub fn insert_dynamic(
        &mut self,
        name: &impl NamedComponent,
        dynamic: impl IntoDynamicComponentValue,
    ) {
        let component = match dynamic.into_dynamic_component() {
            Value::Constant(dynamic) => Value::Constant(Component::Dynamic(dynamic)),
            Value::Dynamic(dynamic) => Value::Dynamic(dynamic.map_each_cloned(Component::Dynamic)),
        };
        self.insert(name, component);
    }

    /// Adds a [`Component`] for the name provided and returns self.
    #[must_use]
    pub fn with<C: ComponentDefinition>(
        mut self,
        name: &C,
        component: impl IntoValue<C::ComponentType>,
    ) -> Self
    where
        Value<C::ComponentType>: IntoComponentValue,
    {
        self.insert_named(
            name.name().into_owned(),
            StoredComponent {
                inherited: false,
                inheritable: true,
                component: component.into_value().into_component_value(),
            },
        );
        self
    }

    /// Adds a [`Component`] using then name provided, resolving the value
    /// through `dynamic`. This function returns self.
    #[must_use]
    pub fn with_dynamic<C: ComponentDefinition>(
        mut self,
        name: &C,
        dynamic: impl IntoDynamicComponentValue,
    ) -> Self {
        self.insert_dynamic(name, dynamic);
        self
    }

    /// Returns the associated component for the given name, if found.
    #[must_use]
    pub fn get_with_fallback<Fallback>(
        &self,
        component: &impl NamedComponent,
        fallback: &Fallback,
        context: &WidgetContext<'_>,
    ) -> Fallback::ComponentType
    where
        Fallback: ComponentDefinition,
    {
        self.0
            .components
            .get(&component.name())
            .or_else(|| self.0.components.get(&fallback.name()))
            .and_then(|stored| Self::resolve_component(&stored.component, context))
            .unwrap_or_else(|| fallback.default_value(context))
    }

    fn resolve_component<T>(component: &Value<Component>, context: &WidgetContext<'_>) -> Option<T>
    where
        T: ComponentType,
    {
        let mut resolved = component.get();
        loop {
            match T::try_from_component(resolved) {
                Ok(value) => {
                    if value.requires_invalidation() {
                        component.invalidate_when_changed(context);
                    } else {
                        component.redraw_when_changed(context);
                    }
                    break Some(value);
                }
                Err(Component::Dynamic(dynamic)) => {
                    let Some(new_component) = dynamic.resolve(context) else {
                        break None;
                    };
                    resolved = new_component;
                }
                Err(_) => break None,
            }
        }
    }

    /// Returns the component associated with the given name, if a value is
    /// specified.
    #[must_use]
    pub fn try_get<Named>(
        &self,
        component: &Named,
        context: &WidgetContext<'_>,
    ) -> Option<Named::ComponentType>
    where
        Named: ComponentDefinition,
    {
        self.0
            .components
            .get(&component.name())
            .and_then(|stored| Self::resolve_component(&stored.component, context))
    }

    /// Returns the component associated with the given name, or if not found,
    /// returns the default value provided by the definition.
    #[must_use]
    pub fn get<Named>(&self, component: &Named, context: &WidgetContext<'_>) -> Named::ComponentType
    where
        Named: ComponentDefinition,
    {
        self.try_get(component, context)
            .unwrap_or_else(|| component.default_value(context))
    }

    /// Inserts all components from `other`, overwriting any existing entries
    /// with the same [`ComponentName`].
    pub fn inherit_from(&mut self, other: Styles) {
        for (name, mut value) in Arc::try_unwrap(other.0)
            .unwrap_or_else(|err| err.as_ref().clone())
            .components
        {
            if !value.inheritable || self.0.components.contains_key(&name) {
                continue;
            }

            value.inherited = true;
            self.insert_named(name, value);
        }
    }

    /// Returns this collection of styles without any local style definitions.
    #[must_use]
    pub fn into_inherited(self) -> Self {
        if self.0.components.values().any(|stored| !stored.inheritable) {
            Self(Arc::new(StyleData {
                components: Arc::try_unwrap(self.0)
                    .unwrap_or_else(|err| err.as_ref().clone())
                    .components
                    .into_iter()
                    .filter(|(_, stored)| stored.inheritable)
                    .collect(),
            }))
        } else {
            self
        }
    }
}

impl Debug for Styles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut map = f.debug_struct("Styles");
        let mut component_name = String::new();
        for (name, stored) in &self.0.components {
            component_name.clear();
            write!(&mut component_name, "{name:?}")?;

            map.field(&component_name, &stored.component);
        }
        map.finish()
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
            into_iter: Arc::try_unwrap(self.0)
                .unwrap_or_else(|err| err.as_ref().clone())
                .components
                .into_iter(),
        }
    }
}

/// An iterator that returns the contents of a [`Styles`] collection.
pub struct StylesIntoIter {
    into_iter: hash_map::IntoIter<ComponentName, StoredComponent>,
}

impl Iterator for StylesIntoIter {
    type Item = (ComponentName, Value<Component>);

    fn next(&mut self) -> Option<Self::Item> {
        self.into_iter
            .next()
            .map(|(name, stored)| (name, stored.component))
    }
}

#[derive(Default, Clone)]
struct StyleData {
    components: AHashMap<ComponentName, StoredComponent>,
}

/// A [`Component`] that is stored within a [`Styles`] collection.
#[derive(Clone)]
pub struct StoredComponent {
    inherited: bool,
    inheritable: bool,
    component: Value<Component>,
}

impl StoredComponent {
    /// Returns a new component that will not be inherited to children widgets.
    pub fn local(component: impl IntoComponentValue) -> Self {
        Self {
            inherited: false,
            inheritable: false,
            component: component.into_component_value(),
        }
    }
}

/// A value that can be converted into a `Value<Component>`.
pub trait IntoComponentValue {
    /// Returns `self` stored in a component value.
    fn into_component_value(self) -> Value<Component>;
}

/// A type that can be converted into a [`StoredComponent`].
pub trait IntoStoredComponent {
    /// Returns this value as a stored component.
    fn into_stored_component(self) -> StoredComponent;
}

impl<T> IntoStoredComponent for T
where
    T: IntoComponentValue,
{
    fn into_stored_component(self) -> StoredComponent {
        StoredComponent {
            inherited: false,
            inheritable: true,
            component: self.into_component_value(),
        }
    }
}

impl IntoStoredComponent for StoredComponent {
    fn into_stored_component(self) -> StoredComponent {
        self
    }
}

impl<T> IntoComponentValue for T
where
    T: Into<Component>,
{
    fn into_component_value(self) -> Value<Component> {
        Value::Constant(self.into())
    }
}

impl<T> IntoComponentValue for Value<T>
where
    T: Clone + Send + 'static,
    Component: From<T>,
{
    fn into_component_value(self) -> Value<Component> {
        self.map_each(|v| Component::from(v.clone()))
    }
}

impl<T> IntoComponentValue for Dynamic<T>
where
    T: Clone + Send + 'static,
    Component: From<T>,
{
    fn into_component_value(self) -> Value<Component> {
        Value::Dynamic(self.map_each_into())
    }
}

/// A type that can convert into a [`Value`] containing a [`DynamicComponent`].
pub trait IntoDynamicComponentValue {
    /// Returns this type converted into a dynamic component value.
    fn into_dynamic_component(self) -> Value<DynamicComponent>;
}

impl IntoDynamicComponentValue for DynamicComponent {
    fn into_dynamic_component(self) -> Value<DynamicComponent> {
        Value::Constant(self)
    }
}

impl<T> IntoDynamicComponentValue for T
where
    T: ComponentDefinition + Clone + Send + Sync + 'static,
{
    fn into_dynamic_component(self) -> Value<DynamicComponent> {
        Value::Constant(DynamicComponent::from(self))
    }
}

impl<T> IntoDynamicComponentValue for Dynamic<T>
where
    T: ComponentDefinition + Clone + Send + Sync + 'static,
{
    fn into_dynamic_component(self) -> Value<DynamicComponent> {
        Value::Dynamic(self.map_each_into())
    }
}

/// A value of a style component.
#[derive(Debug, Clone, PartialEq)]
pub enum Component {
    /// A color.
    Color(Color),
    /// A single-dimension measurement.
    Dimension(Dimension),
    /// A single-dimension measurement.
    DimensionRange(DimensionRange),
    /// A percentage between 0.0 and 1.0.
    Percent(ZeroToOne),
    /// An easing function for animations.
    Easing(EasingFunction),
    /// A visual ordering to use for layout.
    VisualOrder(VisualOrder),
    /// A description of what widgets should be focusable.
    FocusableWidgets(FocusableWidgets),
    /// A description of the depth of a
    /// [`Container`](crate::widgets::Container).
    ContainerLevel(ContainerLevel),
    /// A font family.
    FontFamily(FamilyOwned),
    /// The weight (boldness) of a font.
    FontWeight(Weight),
    /// The style of a font.
    FontStyle(Style),
    /// A string value.
    String(CowString),

    /// A custom component type.
    Custom(CustomComponent),

    /// This component should use the associated value in the named class.
    Dynamic(DynamicComponent),
}

impl Component {
    /// Returns a [`CustomComponent`] created from `component`.
    ///
    /// Custom components allow storing nearly any type in the style system.
    pub fn custom<T>(component: T) -> Self
    where
        T: RequireInvalidation + Debug + Send + Sync + 'static,
    {
        Self::Custom(CustomComponent::new(component))
    }

    /// Returns a new [`DynamicComponent`] which allows resolving a component at
    /// runtime.
    #[must_use]
    pub fn dynamic<T, Func>(resolve: Func) -> Self
    where
        Func:
            for<'a, 'context> Fn(&'a WidgetContext<'context>) -> Option<T> + Send + Sync + 'static,
        T: ComponentType,
    {
        Self::Dynamic(DynamicComponent::new(move |context| {
            resolve(context).map(T::into_component)
        }))
    }
}

macro_rules! impl_component_from_string {
    ($type:ty) => {
        impl From<$type> for Component {
            fn from(s: $type) -> Self {
                Self::String(s.into())
            }
        }
    };
}

impl_component_from_string!(String);
impl_component_from_string!(CowString);
impl_component_from_string!(&'_ str);

macro_rules! impl_component_try_from_string {
    ($type:ty) => {
        impl TryFrom<Component> for $type {
            type Error = Component;

            fn try_from(s: Component) -> Result<Self, Self::Error> {
                match s {
                    Component::String(s) => Ok(s.into()),
                    other => Err(other),
                }
            }
        }
    };
}

impl_component_try_from_string!(String);
impl_component_try_from_string!(CowString);

impl From<FamilyOwned> for Component {
    fn from(value: FamilyOwned) -> Self {
        Self::FontFamily(value)
    }
}

impl TryFrom<Component> for FamilyOwned {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::FontFamily(family) => Ok(family),
            other => Err(other),
        }
    }
}

impl RequireInvalidation for FamilyOwned {
    fn requires_invalidation(&self) -> bool {
        true
    }
}

impl From<Weight> for Component {
    fn from(value: Weight) -> Self {
        Self::FontWeight(value)
    }
}

impl TryFrom<Component> for Weight {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::FontWeight(weight) => Ok(weight),
            other => Err(other),
        }
    }
}

impl RequireInvalidation for Weight {
    fn requires_invalidation(&self) -> bool {
        true
    }
}

impl From<Style> for Component {
    fn from(value: Style) -> Self {
        Self::FontStyle(value)
    }
}

impl TryFrom<Component> for Style {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::FontStyle(style) => Ok(style),
            other => Err(other),
        }
    }
}

impl RequireInvalidation for Style {
    fn requires_invalidation(&self) -> bool {
        true
    }
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

impl RequireInvalidation for Color {
    fn requires_invalidation(&self) -> bool {
        false
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

impl RequireInvalidation for Dimension {
    fn requires_invalidation(&self) -> bool {
        true
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

impl RequireInvalidation for Px {
    fn requires_invalidation(&self) -> bool {
        true
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

impl RequireInvalidation for Lp {
    fn requires_invalidation(&self) -> bool {
        true
    }
}

impl<Unit> From<CornerRadii<Unit>> for Component
where
    Dimension: From<Unit>,
    Unit: Debug + Send + Sync + 'static,
{
    fn from(radii: CornerRadii<Unit>) -> Self {
        let radii = CornerRadii {
            top_left: Dimension::from(radii.top_left),
            top_right: Dimension::from(radii.top_right),
            bottom_right: Dimension::from(radii.bottom_right),
            bottom_left: Dimension::from(radii.bottom_left),
        };
        Component::custom(radii)
    }
}

impl<Unit> RequireInvalidation for CornerRadii<Unit> {
    fn requires_invalidation(&self) -> bool {
        true
    }
}

impl TryFrom<Component> for CornerRadii<Dimension> {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::Custom(custom) => custom
                .downcast()
                .copied()
                .ok_or_else(|| Component::Custom(custom)),
            other => Err(other),
        }
    }
}

/// A 1-dimensional measurement that may be automatically calculated.
#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FlexibleDimension {
    /// Automatically calculate this dimension.
    Auto,
    /// Use this dimension.
    Dimension(Dimension),
}

impl Zero for FlexibleDimension {
    const ZERO: Self = Self::Dimension(Dimension::ZERO);

    fn is_zero(&self) -> bool {
        match self {
            FlexibleDimension::Auto => false,
            FlexibleDimension::Dimension(dim) => dim.is_zero(),
        }
    }
}

impl Debug for FlexibleDimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => f.write_str("Auto"),
            Self::Dimension(arg0) => Debug::fmt(arg0, f),
        }
    }
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
#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Dimension {
    /// Physical Pixels
    Px(Px),
    /// Logical Pixels
    Lp(Lp),
}

impl Debug for Dimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Px(arg0) => Debug::fmt(arg0, f),
            Self::Lp(arg0) => Debug::fmt(arg0, f),
        }
    }
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

impl Zero for Dimension {
    const ZERO: Self = Dimension::Px(Px::ZERO);

    fn is_zero(&self) -> bool {
        match self {
            Dimension::Px(x) => x.is_zero(),
            Dimension::Lp(x) => x.is_zero(),
        }
    }
}

impl ScreenScale for Dimension {
    type Lp = Lp;
    type Px = Px;
    type UPx = UPx;

    fn into_px(self, scale: figures::Fraction) -> Px {
        match self {
            Dimension::Px(px) => px,
            Dimension::Lp(lp) => lp.into_px(scale),
        }
    }

    fn from_px(px: Px, _scale: figures::Fraction) -> Self {
        Self::from(px)
    }

    fn into_lp(self, scale: figures::Fraction) -> Lp {
        match self {
            Dimension::Px(px) => px.into_lp(scale),
            Dimension::Lp(lp) => lp,
        }
    }

    fn from_lp(lp: Lp, _scale: figures::Fraction) -> Self {
        Self::from(lp)
    }

    fn into_upx(self, scale: Fraction) -> Self::UPx {
        match self {
            Dimension::Px(px) => px.into_unsigned(),
            Dimension::Lp(lp) => lp.into_upx(scale),
        }
    }

    fn from_upx(px: Self::UPx, _scale: Fraction) -> Self {
        Self::from(px.into_signed())
    }
}

impl Mul<i32> for Dimension {
    type Output = Dimension;

    fn mul(self, rhs: i32) -> Self::Output {
        match self {
            Self::Px(val) => Self::Px(val * rhs),
            Self::Lp(val) => Self::Lp(val * rhs),
        }
    }
}

impl Mul<f32> for Dimension {
    type Output = Dimension;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            Self::Px(val) => Self::Px(val * rhs),
            Self::Lp(val) => Self::Lp(val * rhs),
        }
    }
}

impl Div<i32> for Dimension {
    type Output = Dimension;

    fn div(self, rhs: i32) -> Self::Output {
        match self {
            Self::Px(val) => Self::Px(val / rhs),
            Self::Lp(val) => Self::Lp(val / rhs),
        }
    }
}

impl Div<f32> for Dimension {
    type Output = Dimension;

    fn div(self, rhs: f32) -> Self::Output {
        match self {
            Self::Px(val) => Self::Px(val / rhs),
            Self::Lp(val) => Self::Lp(val / rhs),
        }
    }
}

/// A range of [`Dimension`]s.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DimensionRange {
    /// The start bound of the range.
    pub start: Bound<Dimension>,
    /// The end bound of the range.
    pub end: Bound<Dimension>,
}

impl Default for DimensionRange {
    fn default() -> Self {
        Self {
            start: Bound::Unbounded,
            end: Bound::Unbounded,
        }
    }
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
            size = size.max(min.into_upx(scale));
        }
        if let Some(max) = self.maximum() {
            size = size.min(max.into_upx(scale));
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

    /// Returns true if this range has no bounds.
    #[must_use]
    pub const fn is_unbounded(&self) -> bool {
        matches!(&self.start, Bound::Unbounded) && matches!(&self.end, Bound::Unbounded)
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

impl RequireInvalidation for DimensionRange {
    fn requires_invalidation(&self) -> bool {
        true
    }
}

/// A custom component value.
#[derive(Debug, Clone)]
pub struct CustomComponent(Arc<dyn AnyComponent>);

impl CustomComponent {
    /// Wraps an arbitrary value so that it can be used as a [`Component`].
    pub fn new<T>(value: T) -> Self
    where
        T: RequireInvalidation + Debug + Send + Sync + 'static,
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

impl PartialEq for CustomComponent {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl RequireInvalidation for CustomComponent {
    fn requires_invalidation(&self) -> bool {
        self.0.requires_invalidation()
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

trait AnyComponent: RequireInvalidation + Send + Sync + Debug {
    fn as_any(&self) -> &dyn Any;
}

impl<T> AnyComponent for T
where
    T: RequireInvalidation + Debug + Send + Sync + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A fully-qualified style component name.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ComponentName {
    /// The group name.
    pub group: Name,
    /// The name of the component within the group.
    pub name: Name,
}

impl ComponentName {
    /// Returns a new instance using `group` and `name`.
    pub fn new(group: impl Into<Name>, name: impl Into<Name>) -> Self {
        Self {
            group: group.into(),
            name: name.into(),
        }
    }
}

impl Debug for ComponentName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}.{:?}", &self.group, &self.name)
    }
}

impl From<&'static Lazy<ComponentName>> for ComponentName {
    fn from(value: &'static Lazy<ComponentName>) -> Self {
        (**value).clone()
    }
}

/// A type that represents a named style component.
pub trait NamedComponent: Sized {
    /// Returns the name of the style component.
    fn name(&self) -> Cow<'_, ComponentName>;
}

/// A type that represents a named component with a default value of a specific
/// Rust type.
pub trait ComponentDefinition: NamedComponent {
    /// The type that will be contained in the [`Component`].
    type ComponentType: ComponentType;

    /// Returns the default value to use for this component.
    fn default_value(&self, context: &WidgetContext<'_>) -> Self::ComponentType;
}

/// A [`ComponentDefinition`] that can provide a default value without access to
/// a runtime context.
pub trait ContextFreeComponent: ComponentDefinition {
    /// Returns the default value for this component.
    fn default(&self) -> Self::ComponentType;

    /// Returns a new probe that provides access to the runtime value of this
    /// component.
    fn probe(self) -> ComponentProbe<Self> {
        ComponentProbe::default_for(self)
    }

    /// Returns a new probe wrapping `child` that provides access to the runtime
    /// value of this component.
    fn probe_wrapping(self, child: impl MakeWidget) -> ComponentProbe<Self> {
        ComponentProbe::default_wrapping(self, child)
    }
}

/// Describes whether a type should invalidate a widget.
pub trait RequireInvalidation {
    /// Cushy tracks two different states:
    ///
    /// - Whether to repaint the window
    /// - Whether to relayout a widget
    ///
    /// If a value change of `self` may require a relayout, this should return
    /// true.
    fn requires_invalidation(&self) -> bool;
}

/// A type that can be converted to and from [`Component`].
pub trait ComponentType: RequireInvalidation + Sized {
    /// Returns this type, wrapped in a [`Component`].
    fn into_component(self) -> Component;
    /// Attempts to extract this type from `component`. If `component` does not
    /// contain this type, `Err(component)` is returned.
    fn try_from_component(component: Component) -> Result<Self, Component>;
}

impl<T> ComponentType for T
where
    T: RequireInvalidation + Into<Component> + TryFrom<Component, Error = Component>,
{
    fn into_component(self) -> Component {
        self.into()
    }

    fn try_from_component(component: Component) -> Result<Self, Component> {
        Self::try_from(component)
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
    /// The top edge
    pub top: T,
    /// The right edge
    pub right: T,
    /// The bottom edge
    pub bottom: T,
}

impl<T> Edges<T> {
    /// Returns the sum of the parts as a [`Size`].
    pub fn size(self) -> Size<T>
    where
        T: Add<Output = T> + Copy,
    {
        Size::new(self.width(), self.height())
    }

    /// Returns a new set of edges produced by calling `map` with each of the
    /// edges.
    pub fn map<U>(self, mut map: impl FnMut(T) -> U) -> Edges<U> {
        Edges {
            left: map(self.left),
            top: map(self.top),
            right: map(self.right),
            bottom: map(self.bottom),
        }
    }

    /// Returns the sum of the left and right edges.
    pub fn width(self) -> T
    where
        T: Add<Output = T>,
    {
        self.left + self.right
    }

    /// Returns the sum of the top and bottom edges.
    pub fn height(self) -> T
    where
        T: Add<Output = T>,
    {
        self.top + self.bottom
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

impl<T> Round for Edges<T>
where
    T: Round,
{
    fn round(self) -> Self {
        self.map(Round::round)
    }

    fn ceil(self) -> Self {
        self.map(Round::ceil)
    }

    fn floor(self) -> Self {
        self.map(Round::floor)
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

impl<Unit> Zero for Edges<Unit>
where
    Unit: Zero,
{
    const ZERO: Self = Self {
        left: Unit::ZERO,
        top: Unit::ZERO,
        right: Unit::ZERO,
        bottom: Unit::ZERO,
    };

    fn is_zero(&self) -> bool {
        self.left.is_zero() && self.top.is_zero() && self.right.is_zero() && self.bottom.is_zero()
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

impl IntoValue<Edges<FlexibleDimension>> for Dimension {
    fn into_value(self) -> Value<Edges<FlexibleDimension>> {
        FlexibleDimension::Dimension(self).into_value()
    }
}

impl IntoValue<Edges<FlexibleDimension>> for Px {
    fn into_value(self) -> Value<Edges<FlexibleDimension>> {
        Dimension::from(self).into_value()
    }
}

impl IntoValue<Edges<FlexibleDimension>> for Lp {
    fn into_value(self) -> Value<Edges<FlexibleDimension>> {
        Dimension::from(self).into_value()
    }
}

impl IntoValue<Edges<Dimension>> for Dimension {
    fn into_value(self) -> Value<Edges<Dimension>> {
        Value::Constant(Edges::from(self))
    }
}

impl IntoValue<Edges<Dimension>> for Px {
    fn into_value(self) -> Value<Edges<Dimension>> {
        Dimension::from(self).into_value()
    }
}

impl IntoValue<Edges<Dimension>> for Lp {
    fn into_value(self) -> Value<Edges<Dimension>> {
        Dimension::from(self).into_value()
    }
}

impl IntoValue<Dimension> for Px {
    fn into_value(self) -> Value<Dimension> {
        Dimension::from(self).into_value()
    }
}

impl IntoValue<Dimension> for Lp {
    fn into_value(self) -> Value<Dimension> {
        Dimension::from(self).into_value()
    }
}

impl IntoValue<FlexibleDimension> for Px {
    fn into_value(self) -> Value<FlexibleDimension> {
        Dimension::from(self).into_value()
    }
}

impl IntoValue<FlexibleDimension> for Lp {
    fn into_value(self) -> Value<FlexibleDimension> {
        Dimension::from(self).into_value()
    }
}

impl IntoValue<FlexibleDimension> for Dimension {
    fn into_value(self) -> Value<FlexibleDimension> {
        FlexibleDimension::from(self).into_value()
    }
}

impl IntoValue<CornerRadii<Dimension>> for Dimension {
    fn into_value(self) -> Value<CornerRadii<Dimension>> {
        Value::Constant(CornerRadii {
            top_left: self,
            top_right: self,
            bottom_right: self,
            bottom_left: self,
        })
    }
}

impl IntoValue<CornerRadii<Dimension>> for Lp {
    fn into_value(self) -> Value<CornerRadii<Dimension>> {
        Dimension::Lp(self).into_value()
    }
}

impl IntoValue<CornerRadii<Dimension>> for Px {
    fn into_value(self) -> Value<CornerRadii<Dimension>> {
        Dimension::Px(self).into_value()
    }
}

impl<U> ScreenScale for Edges<U>
where
    U: ScreenScale<Px = Px, UPx = UPx, Lp = Lp>,
{
    type Lp = Edges<Lp>;
    type Px = Edges<Px>;
    type UPx = Edges<UPx>;

    fn into_px(self, scale: Fraction) -> Self::Px {
        Edges {
            left: self.left.into_px(scale),
            top: self.top.into_px(scale),
            right: self.right.into_px(scale),
            bottom: self.bottom.into_px(scale),
        }
    }

    fn from_px(px: Self::Px, scale: Fraction) -> Self {
        Self {
            left: U::from_px(px.left, scale),
            top: U::from_px(px.top, scale),
            right: U::from_px(px.right, scale),
            bottom: U::from_px(px.bottom, scale),
        }
    }

    fn into_upx(self, scale: Fraction) -> Self::UPx {
        Edges {
            left: self.left.into_upx(scale),
            top: self.top.into_upx(scale),
            right: self.right.into_upx(scale),
            bottom: self.bottom.into_upx(scale),
        }
    }

    fn from_upx(px: Self::UPx, scale: Fraction) -> Self {
        Self {
            left: U::from_upx(px.left, scale),
            top: U::from_upx(px.top, scale),
            right: U::from_upx(px.right, scale),
            bottom: U::from_upx(px.bottom, scale),
        }
    }

    fn into_lp(self, scale: Fraction) -> Self::Lp {
        Edges {
            left: self.left.into_lp(scale),
            top: self.top.into_lp(scale),
            right: self.right.into_lp(scale),
            bottom: self.bottom.into_lp(scale),
        }
    }

    fn from_lp(lp: Self::Lp, scale: Fraction) -> Self {
        Self {
            left: U::from_lp(lp.left, scale),
            top: U::from_lp(lp.top, scale),
            right: U::from_lp(lp.right, scale),
            bottom: U::from_lp(lp.bottom, scale),
        }
    }
}

impl<U, R> Add for Edges<U>
where
    U: Add<Output = R>,
{
    type Output = Edges<R>;

    fn add(self, rhs: Self) -> Self::Output {
        Edges {
            left: self.left + rhs.left,
            top: self.top + rhs.top,
            right: self.right + rhs.right,
            bottom: self.bottom + rhs.bottom,
        }
    }
}

impl<U, R> AddAssign<Edges<R>> for Edges<U>
where
    U: AddAssign<R>,
{
    fn add_assign(&mut self, rhs: Edges<R>) {
        self.left += rhs.left;
        self.top += rhs.top;
        self.right += rhs.right;
        self.bottom += rhs.bottom;
    }
}

/// A set of light and dark [`Theme`]s.
#[derive(Clone, Debug, PartialEq)]
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
    pub fn from_scheme(scheme: &ColorScheme) -> Self {
        Self {
            light: Theme::light_from_sources(
                scheme.primary,
                scheme.secondary,
                scheme.tertiary,
                scheme.error,
                scheme.neutral,
                scheme.neutral_variant,
            ),
            dark: Theme::dark_from_sources(
                scheme.primary,
                scheme.secondary,
                scheme.tertiary,
                scheme.error,
                scheme.neutral,
                scheme.neutral_variant,
            ),
            primary_fixed: FixedTheme::from_source(scheme.primary),
            secondary_fixed: FixedTheme::from_source(scheme.secondary),
            tertiary_fixed: FixedTheme::from_source(scheme.tertiary),
            scrim: scheme.neutral.color(1),
            shadow: scheme.neutral.color(1),
        }
    }
}

impl From<ColorScheme> for ThemePair {
    fn from(scheme: ColorScheme) -> Self {
        Self::from_scheme(&scheme)
    }
}

impl Default for ThemePair {
    fn default() -> Self {
        Self::from(ColorScheme::default())
    }
}

/// A Cushy Color theme.
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
    /// The primary color, brightened for highlighting content.
    pub color_bright: Color,
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
            color_dim: source.color(20),
            color_bright: source.color(45),
            on_color: source.color(100),
            container: source.color(90),
            on_container: source.color(10),
        }
    }

    /// Returns a new dark color theme for `source`.
    #[must_use]
    pub fn dark_from_source(source: ColorSource) -> Self {
        Self {
            color: source.color(80),
            color_dim: source.color(60),
            color_bright: source.color(85),
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

impl PartialEq for ColorSource {
    fn eq(&self, other: &Self) -> bool {
        (self.hue.into_degrees() - other.hue.into_degrees()).abs() < f32::EPSILON
            && self.saturation == other.saturation
    }
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
        let average_saturation = ZeroToOne::new((*self.saturation + *other.saturation) / 2.);

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

        ZeroToOne::new((*saturation_delta + *hue_delta * *average_saturation) / 2.)
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
    /// Converts this color into its hue, saturation, and lightness components.
    fn into_hsla(self) -> Hsla;

    /// Returns the hue and saturation of this color.
    fn source(self) -> ColorSource {
        self.into_hsla().hsl.source
    }

    /// Returns the perceived lightness of this color.
    #[must_use]
    fn lightness(self) -> ZeroToOne {
        self.into_hsla().hsl.lightness
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
    fn into_hsla(self) -> Hsla {
        let mut hsl: palette::Okhsl =
            Srgb::new(self.red_f32(), self.green_f32(), self.blue_f32()).into_color();

        if hsl.saturation.is_nan() && self.red() == 255 && self.green() == 255 && self.blue() == 255
        {
            // This works around a calculation causing NaN in the saturation
            // field when pure white is converted:
            // <https://github.com/Ogeon/palette/issues/368>
            hsl.saturation = 0.0;
        }

        Hsla {
            hsl: Hsl {
                source: ColorSource {
                    hue: hsl.hue,
                    saturation: ZeroToOne::new(hsl.saturation),
                },
                lightness: ZeroToOne::new(hsl.lightness),
            },
            alpha: ZeroToOne::new(self.alpha_f32()),
        }
    }

    fn contrast_between(
        self,
        check_source: ColorSource,
        check_lightness: ZeroToOne,
        check_alpha: ZeroToOne,
    ) -> ZeroToOne {
        let other = self.into_hsla();
        let lightness_delta = other.hsl.lightness.difference_between(check_lightness);

        let source_change = check_source.contrast_between(other.hsl.source);

        let alpha_delta = check_alpha.difference_between(other.alpha);

        // The lightness amount is the most important factor in contrast
        // calculations. Thus, we give a higher weight to it in our average
        // calculation.
        ZeroToOne::new((*lightness_delta * 3. + *source_change + *alpha_delta) / 5.)
    }

    fn most_contrasting(self, others: &[Self]) -> Self
    where
        Self: Copy,
    {
        let check = self.into_hsla();

        let mut others = others.iter().copied();
        let mut most_contrasting = others.next().expect("at least one comparison");
        let mut most_contrast_amount =
            most_contrasting.contrast_between(check.hsl.source, check.hsl.lightness, check.alpha);
        for other in others {
            let contrast_amount =
                other.contrast_between(check.hsl.source, check.hsl.lightness, check.alpha);
            if contrast_amount > most_contrast_amount {
                most_contrasting = other;
                most_contrast_amount = contrast_amount;
            }
        }

        most_contrasting
    }
}

/// A color composed of hue, saturation, and lightness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsla {
    /// The hue, saturation, and lightness of this color.
    pub hsl: Hsl,
    /// The alpha of this color.
    pub alpha: ZeroToOne,
}

impl From<Color> for Hsla {
    fn from(value: Color) -> Self {
        value.into_hsla()
    }
}

impl From<Hsla> for Color {
    fn from(value: Hsla) -> Self {
        Color::from(value.hsl).with_alpha_f32(*value.alpha)
    }
}

/// A color composed of hue, saturation, and lightness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsl {
    /// The hue and saturation of this color.
    pub source: ColorSource,
    /// The lightness of this color.
    pub lightness: ZeroToOne,
}

impl From<Color> for Hsl {
    fn from(value: Color) -> Self {
        value.into_hsla().hsl
    }
}

impl From<Hsl> for Color {
    fn from(value: Hsl) -> Self {
        value.source.color(value.lightness)
    }
}

/// A 2d ordering configuration.
#[derive(Copy, Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VisualOrder {
    /// The ordering to apply horizontally.
    pub horizontal: HorizontalOrder,
    /// The ordering to apply vertically.
    pub vertical: VerticalOrder,
}

impl VisualOrder {
    /// Returns a right-to-left ordering.
    #[must_use]
    pub const fn right_to_left() -> Self {
        Self {
            horizontal: HorizontalOrder::RightToLeft,
            vertical: VerticalOrder::TopToBottom,
        }
    }

    /// Returns a left-to-right ordering.
    #[must_use]
    pub const fn left_to_right() -> Self {
        Self {
            horizontal: HorizontalOrder::LeftToRight,
            vertical: VerticalOrder::TopToBottom,
        }
    }

    /// Returns the reverse ordering of `self`.
    #[must_use]
    pub fn rev(self) -> Self {
        Self {
            horizontal: self.horizontal.rev(),
            vertical: self.vertical.rev(),
        }
    }
}

impl From<VisualOrder> for Component {
    fn from(value: VisualOrder) -> Self {
        Self::VisualOrder(value)
    }
}

impl TryFrom<Component> for VisualOrder {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::VisualOrder(order) => Ok(order),
            other => Err(other),
        }
    }
}

impl RequireInvalidation for VisualOrder {
    fn requires_invalidation(&self) -> bool {
        true
    }
}

/// A horizontal direction.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HorizontalOrder {
    /// Describes an order starting at the left and proceeding to the right.
    LeftToRight,
    /// Describes an order starting at the right and proceeding to the left.
    RightToLeft,
}

impl HorizontalOrder {
    /// Returns the reverse order of `self`.
    #[must_use]
    pub fn rev(self) -> Self {
        match self {
            Self::LeftToRight => Self::RightToLeft,
            Self::RightToLeft => Self::LeftToRight,
        }
    }

    pub(crate) fn sort_key(self, rect: &Rect<Px>) -> Px {
        match self {
            HorizontalOrder::LeftToRight => rect.origin.x,
            HorizontalOrder::RightToLeft => -(rect.origin.x + rect.size.width),
        }
    }
}

/// A vertical direction.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VerticalOrder {
    /// Describes an order starting at the top and proceeding to the bottom.
    TopToBottom,
    /// Describes an order starting at the bottom and proceeding to the top.
    BottomToTop,
}

impl VerticalOrder {
    /// Returns the reverse order of `self`.
    #[must_use]
    pub fn rev(self) -> Self {
        match self {
            Self::TopToBottom => VerticalOrder::BottomToTop,
            Self::BottomToTop => VerticalOrder::TopToBottom,
        }
    }

    pub(crate) fn max_px(self) -> Px {
        match self {
            VerticalOrder::TopToBottom => Px::MAX,
            VerticalOrder::BottomToTop => Px::MIN,
        }
    }

    pub(crate) fn smallest_px(self, a: Px, b: Px) -> Px {
        match self {
            VerticalOrder::TopToBottom => a.min(b),
            VerticalOrder::BottomToTop => b.max(a),
        }
    }
}

/// A configuration option to control which controls should be able to receive
/// focus through keyboard focus handling or initial focus handling.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FocusableWidgets {
    /// Allow all widgets that can respond to keyboard input to accept focus.
    #[default]
    All,
    /// Only allow widgets that expect textual input to accept focus.
    OnlyTextual,
}

impl FocusableWidgets {
    /// Returns true if all controls should be focusable.
    #[must_use]
    pub const fn is_all(self) -> bool {
        matches!(self, Self::All)
    }

    /// Returns true if only textual should be focusable.
    #[must_use]
    pub const fn is_only_textual(self) -> bool {
        matches!(self, Self::OnlyTextual)
    }
}

impl From<FocusableWidgets> for Component {
    fn from(value: FocusableWidgets) -> Self {
        Self::FocusableWidgets(value)
    }
}

impl TryFrom<Component> for FocusableWidgets {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::FocusableWidgets(focus) => Ok(focus),
            other => Err(other),
        }
    }
}

impl RequireInvalidation for FocusableWidgets {
    fn requires_invalidation(&self) -> bool {
        false
    }
}

/// A description of the level of depth a
/// [`Container`](crate::widgets::Container) is nested at.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ContainerLevel {
    /// The lowest container level.
    #[default]
    Lowest,
    /// The second lowest container level.
    Low,
    /// The mid-level container level.
    Mid,
    /// The second-highest container level.
    High,
    /// The highest container level.
    Highest,
}

impl ContainerLevel {
    /// Returns the next container level, or None if already at the highet
    /// level.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::Lowest => Some(Self::Low),
            Self::Low => Some(Self::Mid),
            Self::Mid => Some(Self::High),
            Self::High => Some(Self::Highest),
            Self::Highest => None,
        }
    }
}

impl From<ContainerLevel> for Component {
    fn from(value: ContainerLevel) -> Self {
        Self::ContainerLevel(value)
    }
}

impl TryFrom<Component> for ContainerLevel {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::ContainerLevel(level) => Ok(level),
            other => Err(other),
        }
    }
}

impl RequireInvalidation for ContainerLevel {
    fn requires_invalidation(&self) -> bool {
        true
    }
}

/// A builder of [`ColorScheme`]s.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorSchemeBuilder {
    /// The primary color of the scheme.
    pub primary: ColorSource,
    /// The secondary color of the scheme. If not provided, a complimentary
    /// color will be chosen.
    pub secondary: Option<ColorSource>,
    /// The tertiary color of the scheme. If not provided, a complimentary
    /// color will be chosen.
    pub tertiary: Option<ColorSource>,
    /// The error color of the scheme. If not provided, red will be used unless
    /// it contrasts poorly with any of the other colors.
    pub error: Option<ColorSource>,
    /// The neutral color of the scheme. If not provided, a nearly fully
    /// desaturated variation of the primary color will be used.
    pub neutral: Option<ColorSource>,
    /// The neutral variant color of the scheme. If not provided, a mostly
    /// desaturated variation of the primary color will be used.
    pub neutral_variant: Option<ColorSource>,
    hue_shift: OklabHue,
}

impl ColorSchemeBuilder {
    /// Returns a builder for the provided primary color.
    #[must_use]
    pub fn new(primary: impl ProtoColor) -> Self {
        Self {
            primary: primary.into_source(ZeroToOne::new(0.8)),
            secondary: None,
            tertiary: None,
            error: None,
            neutral: None,
            neutral_variant: None,
            hue_shift: OklabHue::new(30.),
        }
    }

    fn generate_secondary(&self) -> ColorSource {
        ColorSource {
            hue: self.primary.hue + self.hue_shift,
            saturation: self.primary.saturation / 2.,
        }
    }

    fn generate_tertiary(&self, secondary: ColorSource) -> ColorSource {
        let hue_shift = (secondary.hue - self.primary.hue).into_degrees().signum()
            * self.hue_shift.into_degrees();
        ColorSource {
            hue: self.primary.hue - hue_shift,
            saturation: self.primary.saturation / 3.,
        }
    }

    fn generate_error(&self, secondary: ColorSource, tertiary: ColorSource) -> ColorSource {
        let mut error = ColorSource::new(30., self.primary.saturation);
        let shift_degrees = self.hue_shift.into_positive_degrees().ceil().cast::<u32>();
        let mut iters_left = (360 - (shift_degrees - 1)) / shift_degrees;
        while iters_left > 0
            && [self.primary, secondary, tertiary]
                .iter()
                .any(|c| c.contrast_between(error) < 0.20)
        {
            error.hue -= self.hue_shift;
            iters_left -= 1;
        }

        error
    }

    fn generate_neutral(&self) -> ColorSource {
        ColorSource {
            hue: self.primary.hue,
            saturation: ZeroToOne::new(0.01),
        }
    }

    fn generate_neutral_variant(&self) -> ColorSource {
        ColorSource {
            hue: self.primary.hue,
            saturation: self.primary.saturation / 10.,
        }
    }

    /// Sets the secondary color and returns self.
    ///
    /// If `secondary` doesn't specify a saturation, a saturation value that is
    /// 50% of the primary saturation will be picked.
    #[must_use]
    pub fn secondary(mut self, secondary: impl ProtoColor) -> Self {
        self.secondary = Some(secondary.into_source(self.primary.saturation / 2.));
        self
    }

    /// Sets the tertiary color and returns self.
    ///
    /// If `tertiary` doesn't specify a saturation, a saturation value that is
    /// 33% of the primary saturation will be picked.
    #[must_use]
    pub fn tertiary(mut self, tertiary: impl ProtoColor) -> Self {
        self.tertiary = Some(tertiary.into_source(self.primary.saturation / 3.));
        self
    }

    /// Sets the error color and returns self.
    ///
    /// If `error` doesn't specify a saturation, the primary color's saturation
    /// will be used.
    #[must_use]
    pub fn error(mut self, error: impl ProtoColor) -> Self {
        self.error = Some(error.into_source(self.primary.saturation));
        self
    }

    /// Sets the neutral color and returns self.
    ///
    /// If `neutral` doesn't specify a saturation, a saturation of 1%.
    #[must_use]
    pub fn neutral(mut self, neutral: impl ProtoColor) -> Self {
        self.neutral = Some(neutral.into_source(0.01));
        self
    }

    /// Sets the neutral color and returns self.
    ///
    /// If `neutral_variant` doesn't specify a saturation, a saturation value
    /// that is 10% of the primary saturation will be picked.
    #[must_use]
    pub fn neutral_variant(mut self, neutral_variant: impl ProtoColor) -> Self {
        self.neutral_variant = Some(neutral_variant.into_source(self.primary.saturation / 10.));
        self
    }

    /// Sets the amount the hue component is shifted when auto-generating colors
    /// to fill in the palette.
    ///
    /// The default hue shift is 30 degrees.
    #[must_use]
    pub fn hue_shift(mut self, hue_shift: impl Into<OklabHue>) -> Self {
        self.hue_shift = hue_shift.into();
        self
    }

    /// Builds a color scheme from the provided colors, generating any
    /// unspecified colors.
    #[must_use]
    pub fn build(self) -> ColorScheme {
        let secondary = self.secondary.unwrap_or_else(|| self.generate_secondary());
        let tertiary = self
            .tertiary
            .unwrap_or_else(|| self.generate_tertiary(secondary));
        ColorScheme {
            primary: self.primary,
            secondary,
            tertiary,
            error: self
                .error
                .unwrap_or_else(|| self.generate_error(secondary, tertiary)),
            neutral: self.neutral.unwrap_or_else(|| self.generate_neutral()),
            neutral_variant: self
                .neutral_variant
                .unwrap_or_else(|| self.generate_neutral_variant()),
        }
    }
}

/// A type that can be interpretted as a hue or hue and saturation.
pub trait ProtoColor: Sized {
    /// Returns the hue of this prototype color.
    #[must_use]
    fn hue(&self) -> OklabHue;
    /// Returns the saturation of this prototype color, if available.
    #[must_use]
    fn saturation(&self) -> Option<ZeroToOne>;

    /// Returns a color source built from this prototype color
    #[must_use]
    fn into_source(self, saturation_if_not_provided: impl Into<ZeroToOne>) -> ColorSource {
        let saturation = self
            .saturation()
            .unwrap_or_else(|| saturation_if_not_provided.into());
        ColorSource::new(self.hue(), saturation)
    }
}

impl<'a> ProtoColor for &'a ColorSource {
    fn hue(&self) -> OklabHue {
        self.hue
    }

    fn saturation(&self) -> Option<ZeroToOne> {
        Some(self.saturation)
    }
}

impl ProtoColor for f32 {
    fn hue(&self) -> OklabHue {
        (*self).into()
    }

    fn saturation(&self) -> Option<ZeroToOne> {
        None
    }
}

impl ProtoColor for OklabHue {
    fn hue(&self) -> OklabHue {
        *self
    }

    fn saturation(&self) -> Option<ZeroToOne> {
        None
    }
}

impl ProtoColor for ColorSource {
    fn hue(&self) -> OklabHue {
        self.hue
    }

    fn saturation(&self) -> Option<ZeroToOne> {
        Some(self.saturation)
    }
}

impl<Hue, Saturation> ProtoColor for (Hue, Saturation)
where
    Hue: Into<OklabHue> + Copy,
    Saturation: Into<ZeroToOne> + Copy,
{
    fn hue(&self) -> OklabHue {
        self.0.into()
    }

    fn saturation(&self) -> Option<ZeroToOne> {
        Some(self.1.into())
    }
}

/// A color scheme for a Cushy application.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorScheme {
    /// The primary accent color.
    pub primary: ColorSource,
    /// A secondary accent color.
    pub secondary: ColorSource,
    /// A tertiary accent color.
    pub tertiary: ColorSource,
    /// A color used to denote errors.
    pub error: ColorSource,
    /// A neutral color.
    pub neutral: ColorSource,
    /// A neutral color with a different tone than `neutral`.
    pub neutral_variant: ColorSource,
}

impl ColorScheme {
    /// Returns a generated color scheme based on a `primary` color.
    #[must_use]
    pub fn from_primary(primary: impl ProtoColor) -> Self {
        ColorSchemeBuilder::new(primary).build()
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::from_primary(138.5)
    }
}

impl From<ColorSource> for ColorScheme {
    fn from(primary: ColorSource) -> Self {
        ColorScheme::from_primary(primary)
    }
}

/// A list of font families.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontFamilyList(Arc<Vec<FamilyOwned>>);

impl Default for FontFamilyList {
    fn default() -> Self {
        static DEFAULT: Lazy<FontFamilyList> = Lazy::new(|| FontFamilyList::from(vec![]));
        DEFAULT.clone()
    }
}

impl FontFamilyList {
    /// Pushes `family` on the end of this list.
    pub fn push(&mut self, family: FamilyOwned) {
        Arc::make_mut(&mut self.0).push(family);
    }
}

impl Deref for FontFamilyList {
    type Target = [FamilyOwned];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromIterator<FamilyOwned> for FontFamilyList {
    fn from_iter<T: IntoIterator<Item = FamilyOwned>>(iter: T) -> Self {
        Self(Arc::new(iter.into_iter().collect()))
    }
}

impl From<FamilyOwned> for FontFamilyList {
    fn from(value: FamilyOwned) -> Self {
        Self::from(vec![value])
    }
}

impl From<Vec<FamilyOwned>> for FontFamilyList {
    fn from(value: Vec<FamilyOwned>) -> Self {
        Self(Arc::new(value))
    }
}

impl IntoValue<FontFamilyList> for FamilyOwned {
    fn into_value(self) -> Value<FontFamilyList> {
        FontFamilyList::from(self).into_value()
    }
}

impl IntoValue<FontFamilyList> for Vec<FamilyOwned> {
    fn into_value(self) -> Value<FontFamilyList> {
        FontFamilyList::from(self).into_value()
    }
}

impl From<FontFamilyList> for Component {
    fn from(list: FontFamilyList) -> Self {
        Component::custom(list)
    }
}

impl RequireInvalidation for FontFamilyList {
    fn requires_invalidation(&self) -> bool {
        true
    }
}

impl TryFrom<Component> for FontFamilyList {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::Custom(custom) => custom
                .downcast()
                .cloned()
                .ok_or_else(|| Component::Custom(custom)),
            other => Err(other),
        }
    }
}

/// A [`Component`] that resolves its value at runtime.
#[derive(Clone)]
pub struct DynamicComponent(Arc<dyn DynamicComponentResolver>);

impl Debug for DynamicComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DynamicComponent").finish()
    }
}

impl PartialEq for DynamicComponent {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// A type that resolves to a [`Component`] at runtime.
pub trait DynamicComponentResolver: Send + Sync + 'static {
    /// Returns the effective component, if one should be applied.
    fn resolve_component(&self, context: &WidgetContext<'_>) -> Option<Component>;
}

struct DynamicFunctionWrapper<F>(F);

impl<T> DynamicComponentResolver for DynamicFunctionWrapper<T>
where
    T: for<'a, 'context> Fn(&'a WidgetContext<'context>) -> Option<Component>
        + Send
        + Sync
        + 'static,
{
    fn resolve_component(&self, context: &WidgetContext<'_>) -> Option<Component> {
        self.0(context)
    }
}

impl<T> DynamicComponentResolver for T
where
    T: ComponentDefinition + Clone + Send + Sync + 'static,
{
    fn resolve_component(&self, context: &WidgetContext<'_>) -> Option<Component> {
        Some(context.get(self).into_component())
    }
}

impl<T> From<T> for DynamicComponent
where
    T: DynamicComponentResolver,
{
    fn from(resolve: T) -> Self {
        Self(Arc::new(resolve))
    }
}

impl DynamicComponent {
    /// Returns a new dynamic component that invokes `resolve` each time it is
    /// used by widgets.
    #[must_use]
    pub fn new<Func>(resolve: Func) -> Self
    where
        Func: for<'a, 'context> Fn(&'a WidgetContext<'context>) -> Option<Component>
            + Send
            + Sync
            + 'static,
    {
        Self::from(DynamicFunctionWrapper(resolve))
    }

    /// Invokes the resolver function, optionally returning a resolved
    /// component.
    #[must_use]
    pub fn resolve(&self, context: &WidgetContext<'_>) -> Option<Component> {
        self.0.resolve_component(context)
    }
}
