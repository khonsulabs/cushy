//! Types for styling Gooey applications.

use core::slice;
use std::borrow::Cow;
use std::ops::Deref;

use alot::{LotId, Lots};
use figures::units::{Lp, Px};
use figures::Angle;
use gooey_reactor::Dynamic;
use kempt::{Map, Set};
use palette::FromColor;
use stylecs::NameKey;
pub use stylecs::{static_name, Identifier, Name, StaticName, Style, StyleComponent};
pub mod classes;

pub use classes::Classes;

use crate::{Context, Value};

#[derive(Debug, Clone, Copy)]
pub enum Dimension {
    Zero,
    Length(Length),
    Percent(f32),
}

#[derive(Debug, Clone, Copy)]
pub enum Length {
    Pixels(Px),
    LogicalPixels(Lp),
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Percent(pub f32);

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Zero;

impl From<Px> for Dimension {
    fn from(value: Px) -> Self {
        Self::Length(Length::Pixels(value))
    }
}

impl From<Lp> for Dimension {
    fn from(value: Lp) -> Self {
        Self::Length(Length::LogicalPixels(value))
    }
}

impl From<Percent> for Dimension {
    fn from(value: Percent) -> Self {
        Self::Percent(value.0)
    }
}

impl From<Zero> for Dimension {
    fn from(_: Zero) -> Self {
        Self::Zero
    }
}

#[derive(Debug, StyleComponent, Clone, Copy)]
#[style(authority = gooey, inherited = true)]
pub enum Color {
    Rgba {
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    Hsl {
        hue: Angle,
        saturation: f32,
        value: f32,
        alpha: f32,
    },
}

impl Color {
    #[must_use]
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::Rgba { r, g, b, a }
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn into_rgba(self) -> (u8, u8, u8, u8) {
        match self {
            Color::Rgba { r, g, b, a } => (r, g, b, a),
            Color::Hsl {
                hue,
                saturation,
                value,
                alpha,
            } => {
                let hsl = palette::Hsl::new(hue.into_raidans_f(), saturation, value);
                let rgb = palette::Srgb::from_color(hsl);
                (
                    (rgb.red * 255.).round() as u8,
                    (rgb.green * 255.).round() as u8,
                    (rgb.blue * 255.).round() as u8,
                    (alpha * 255.).round() as u8,
                )
            }
        }
    }
}

#[derive(Debug, StyleComponent, Clone, Copy)]
#[style(authority = gooey, inherited = true)]
pub struct FontSize(pub Dimension);

impl<T> From<T> for FontSize
where
    T: Into<Dimension>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, StyleComponent, Clone, Copy)]
#[style(authority = gooey)]
pub struct BackgroundColor(pub Color);

pub enum Constant {
    Primitive(Primitive),
    Style(Style),
}

pub enum Primitive {
    Dimension(Dimension),
    Angle(Angle),
    Color(Color),
}

pub struct Pattern {
    pub selector: Selector,
    pub apply: WidgetStyle,
}

impl Pattern {
    #[must_use]
    pub fn new(selector: Selector) -> Self {
        Self {
            selector,
            apply: WidgetStyle::default(),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Selector {
    pub id: Option<Name>,
    pub widget: Option<Name>,
    pub classes: Classes,
}

impl Default for Selector {
    fn default() -> Self {
        Self::new()
    }
}

impl Selector {
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: None,
            widget: None,
            classes: Classes::new(),
        }
    }

    #[must_use]
    pub fn id(mut self, id: Name) -> Self {
        self.id = Some(id);
        self
    }

    #[must_use]
    pub fn classes(mut self, classes: Classes) -> Self {
        self.classes = classes;
        self
    }

    #[must_use]
    pub fn widget<StaticWidget>(self) -> Self
    where
        StaticWidget: crate::StaticWidget,
    {
        self.widget_name(StaticWidget::static_name())
    }

    #[must_use]
    pub fn widget_name(mut self, widget: Name) -> Self {
        self.widget = Some(widget);
        self
    }
}

#[derive(Default)]
pub struct WidgetStyle {
    pub style: Style,
    pub nested: Map<NestedSelector, Pattern>,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Clone)]
pub enum NestedSelector {
    Name(Name),
    Widget(Name),
}

pub struct Library {
    _constants: Map<NameKey<'static>, Constant>,
    patterns_by_selector: Map<SelectorKey<'static>, Vec<PatternId>>,
    patterns: Lots<Pattern>,
}

impl Library {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _constants: Map::new(),
            patterns_by_selector: Map::new(),
            patterns: Lots::new(),
        }
    }

    pub fn push(&mut self, pattern: Pattern) -> PatternId {
        let mut keys = Vec::with_capacity(
            usize::from(pattern.selector.id.is_some())
                + usize::from(pattern.selector.widget.is_some())
                + pattern.selector.classes.len(),
        );
        if let Some(id) = &pattern.selector.id {
            keys.push(SelectorKey::id(id.clone()));
        }
        if let Some(id) = &pattern.selector.widget {
            keys.push(SelectorKey::widget(id.clone()));
        }
        for class in &pattern.selector.classes {
            keys.push(SelectorKey::class(class.clone()));
        }
        let id = PatternId(self.patterns.push(pattern));
        for key in keys {
            let keys = self
                .patterns_by_selector
                .entry(key)
                .or_insert_with(Vec::new);
            keys.push(id);
        }
        id
    }

    #[must_use]
    pub fn patterns_matching<'a>(
        &'a self,
        id: Option<&'a Name>,
        widget: Option<&'a Name>,
        classes: Option<&'a Classes>,
    ) -> Matches<'a> {
        Matches {
            library: self,
            id,
            widget,
            classes,
            state: MatchState::Start,
            matched: Set::new(),
        }
    }
}

pub struct Matches<'a> {
    library: &'a Library,
    id: Option<&'a Name>,
    widget: Option<&'a Name>,
    classes: Option<&'a Classes>,
    state: MatchState<'a>,
    matched: Set<PatternId>,
}

impl<'a> Iterator for Matches<'a> {
    type Item = &'a Pattern;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.state {
                MatchState::Start => {
                    // The first step is to find a way to start matching. We
                    // prefer most specific to least specific, as the more
                    // unique names are, the less matches we should need to process.
                    if let Some(id) = &self.id {
                        if let Some(matches) =
                            self.library.patterns_by_selector.get(&SelectorKey::id(*id))
                        {
                            self.state = MatchState::Id(matches.iter());
                        } else {
                            self.state = MatchState::NoMatches;
                        }
                    } else if let Some(widget) = &self.widget {
                        if let Some(matches) = self
                            .library
                            .patterns_by_selector
                            .get(&SelectorKey::widget(*widget))
                        {
                            self.state = MatchState::Widget(matches.iter());
                        } else {
                            self.state = MatchState::NoMatches;
                        }
                    } else if let Some(classes) = &self.classes {
                        self.state = MatchState::Classes(classes.iter());
                    } else {
                        self.state = MatchState::NoMatches;
                    }
                }
                MatchState::Id(ids) => {
                    // `ids` contains a list of patterns that match the id
                    // already. Only return matches if the widget and classes match.
                    let id = ids.next()?;
                    let pattern = &self.library.patterns[id.0];
                    if pattern.selector.widget.as_ref() == self.widget
                        && self.classes.as_ref().map_or(true, |classes| {
                            classes.contains_all(&pattern.selector.classes)
                        })
                    {
                        break Some(pattern);
                    }
                }
                MatchState::Widget(ids) => {
                    // `ids` contains a list of patterns that match the widget
                    // already. Only return matches if the pattern has no id and
                    // its classes match.
                    let id = ids.next()?;
                    let pattern = &self.library.patterns[id.0];
                    if pattern.selector.id.is_none()
                        && self.classes.as_ref().map_or(true, |classes| {
                            classes.contains_all(&pattern.selector.classes)
                        })
                    {
                        break Some(pattern);
                    }
                }
                MatchState::Class(ids, classes) => {
                    // `ids` contains a list of patterns that match a class
                    // already. If we run out of patterns to scan for the
                    // current match, we reset the state to iterating over
                    // the classes.
                    let Some(id) = ids.next() else {
                        self.state = MatchState::Classes(classes.clone());
                        continue
                    };
                    // Only return matches if the pattern has no id, no widget,
                    // and its classes fully match.
                    let pattern = &self.library.patterns[id.0];
                    if pattern.selector.id.is_none()
                        && pattern.selector.widget.is_none()
                        && self.classes.as_ref().map_or(true, |classes| {
                            classes.contains_all(&pattern.selector.classes)
                        })
                    {
                        // Unlike the other pattern matching flows, we might
                        // encounter the same pattern multiple times when
                        // searching across all class intersections.
                        if self.matched.insert(*id) {
                            break Some(pattern);
                        }
                    }
                }
                MatchState::Classes(classes) => {
                    let class = classes.next()?;

                    if let Some(matches) = self
                        .library
                        .patterns_by_selector
                        .get(&SelectorKey::class(class))
                    {
                        self.state = MatchState::Class(matches.iter(), classes.clone());
                    }
                }
                MatchState::NoMatches => break None,
            }
        }
    }
}

enum MatchState<'a> {
    Start,
    Id(slice::Iter<'a, PatternId>),
    Widget(slice::Iter<'a, PatternId>),
    Classes(alot::ordered::Iter<'a, Name>),
    Class(slice::Iter<'a, PatternId>, alot::ordered::Iter<'a, Name>),
    NoMatches,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Clone)]
enum SelectorKey<'a> {
    Id(Cow<'a, Name>),
    Widget(Cow<'a, Name>),
    Class(Cow<'a, Name>),
}

impl<'a> SelectorKey<'a> {
    fn id(name: impl Into<Cow<'a, Name>>) -> Self {
        Self::Id(name.into())
    }

    fn widget(name: impl Into<Cow<'a, Name>>) -> Self {
        Self::Widget(name.into())
    }

    fn class(name: impl Into<Cow<'a, Name>>) -> Self {
        Self::Class(name.into())
    }
}

#[test]
fn pattern_matching() {
    let mut library = Library::new();
    let my_id = Name::private("my_id").unwrap();
    library.push(Pattern::new(Selector::new().id(my_id.clone())));
    let my_widget = Name::private("my_widget").unwrap();
    library.push(Pattern::new(Selector::new().widget_name(my_widget.clone())));
    let mut classes = Classes::new();
    classes.push(Name::private("a").unwrap());
    classes.push(Name::private("b").unwrap());
    library.push(Pattern::new(Selector::new().classes(classes.clone())));
    library.push(Pattern::new(
        Selector::new()
            .id(my_id.clone())
            .widget_name(my_widget.clone())
            .classes(classes.clone()),
    ));
    let matches = library
        .patterns_matching(Some(&my_id), None, None)
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1);
    assert!(matches[0].selector.id.is_some());

    let matches = library
        .patterns_matching(None, Some(&my_widget), None)
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1);

    let matches = library
        .patterns_matching(None, None, Some(&classes))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1);

    // Match against all criteria
    let matches = library
        .patterns_matching(None, None, Some(&classes))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1);

    // Matching a,b,c against a selector that looks for a,b
    classes.push(Name::private("c").unwrap());
    let matches = library
        .patterns_matching(None, None, Some(&classes))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1);

    // Matching a against a selector for a,b won't match anything
    let a_only = Classes::from_iter([Name::private("a").unwrap()]);
    let matches = library
        .patterns_matching(None, None, Some(&a_only))
        .collect::<Vec<_>>();
    assert!(matches.is_empty());
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub struct PatternId(LotId);

#[derive(Clone, Copy, Debug)]
pub struct DynamicStyle(Dynamic<Style>);

impl DynamicStyle {
    #[must_use]
    pub fn new(context: &Context) -> Self {
        Self(context.new_dynamic(Style::new()))
    }

    #[must_use]
    pub fn with<S>(self, component: impl DynamicOrStaticStyleComponent<S>) -> Self
    where
        S: StyleComponent + Clone,
    {
        self.push(component);
        self
    }

    pub fn push<S>(&self, component: impl DynamicOrStaticStyleComponent<S>)
    where
        S: StyleComponent + Clone,
    {
        match component.into_widget_value() {
            Value::Static(component) => {
                self.0.map_mut(|style| style.push(component));
            }
            Value::Dynamic(value) => value.for_each({
                let style = self.0;
                move |value| {
                    style.map_mut(|style| {
                        style.push(value.clone());
                    });
                }
            }),
        };
    }
}

impl Deref for DynamicStyle {
    type Target = Dynamic<Style>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait DynamicOrStaticStyleComponent<T> {
    fn into_widget_value(self) -> Value<T>;
}

impl<T> DynamicOrStaticStyleComponent<T> for T
where
    T: StyleComponent + Clone,
{
    fn into_widget_value(self) -> Value<T> {
        Value::Static(self)
    }
}

impl<T> DynamicOrStaticStyleComponent<T> for Dynamic<T>
where
    T: StyleComponent + Clone,
{
    fn into_widget_value(self) -> Value<T> {
        Value::Dynamic(self)
    }
}

impl<T> DynamicOrStaticStyleComponent<T> for Value<T>
where
    T: StyleComponent + Clone,
{
    fn into_widget_value(self) -> Value<T> {
        self
    }
}

#[test]
fn dynamic_style_updates() {
    let reactor = gooey_reactor::Reactor::default();
    let scope = reactor.new_scope();
    let font_size = scope.new_dynamic(FontSize::from(Px(13)));
    let style = DynamicStyle(scope.new_dynamic(Style::new()))
        .with(font_size)
        .with(BackgroundColor(Color::Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }));
    assert!(matches!(
        style
            .map_ref(|style| style.get::<FontSize>().copied())
            .flatten(),
        Some(FontSize(Dimension::Length(Length::Pixels(_))))
    ));
    assert!(matches!(
        style
            .map_ref(|style| style.get::<BackgroundColor>().copied())
            .flatten(),
        Some(BackgroundColor(Color::Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }))
    ));
    font_size.set(FontSize::from(Zero));
    assert!(matches!(
        style
            .map_ref(|style| style.get::<FontSize>().copied())
            .flatten(),
        Some(FontSize(Dimension::Zero))
    ));
}
