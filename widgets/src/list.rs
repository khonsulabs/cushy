use std::{borrow::Cow, fmt::Debug, marker::PhantomData};

use gooey_core::{
    figures::Figure,
    styles::{Padding, StyleComponent},
    Context, Key, KeyedStorage, RelatedStorage, Scaled, StyledWidget, Widget, WidgetId,
    WidgetRegistration, WidgetStorage,
};
use septem::Roman;

use crate::component::{Behavior, ComponentBuilder, Content, ContentBuilder};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug)]
pub enum Kind {
    Unordered {
        kind: UnorderedListKind,
    },
    Ordered {
        start: Option<i32>,
        kind: OrderedListKind,
        reversed: bool,
    },
}

impl Kind {
    #[must_use]
    pub const fn is_unordered(&self) -> bool {
        matches!(self, Kind::Unordered { .. })
    }

    #[must_use]
    pub const fn is_ordered(&self) -> bool {
        !self.is_unordered()
    }

    #[must_use]
    pub const fn is_unadorned(&self) -> bool {
        matches!(
            self,
            Self::Unordered {
                kind: UnorderedListKind::None
            }
        )
    }
}

impl Default for Kind {
    fn default() -> Self {
        Self::Unordered {
            kind: UnorderedListKind::Bullet,
        }
    }
}

#[derive(Debug)]
pub enum OrderedListKind {
    Decimal,
    AlphaLower,
    AlphaUpper,
    RomanLower,
    RomanUpper,
}

impl Default for OrderedListKind {
    fn default() -> Self {
        Self::Decimal
    }
}

#[derive(Debug)]
pub enum UnorderedListKind {
    None,
    Bullet,
    Circle,
    Square,
}

#[derive(Debug)]
pub struct List {
    children: Vec<WidgetRegistration>,
    kind: Kind,
}

impl List {
    #[must_use]
    pub fn build(storage: &WidgetStorage) -> Builder<(), WidgetStorage> {
        Builder::new(storage.clone())
    }

    #[must_use]
    pub fn bulleted(storage: &WidgetStorage) -> Builder<(), WidgetStorage> {
        Builder::new(storage.clone()).bulleted()
    }

    #[must_use]
    pub fn unadorned(storage: &WidgetStorage) -> Builder<(), WidgetStorage> {
        Builder::new(storage.clone()).unadorned()
    }

    pub fn remove(&mut self, index: usize, context: &Context<Self>) -> WidgetRegistration {
        let removed_child = self.children.remove(index);
        context.send_command(ListCommand::ChildRemoved(removed_child.id().clone()));
        removed_child
    }

    pub fn push<W: Widget>(&mut self, widget: StyledWidget<W>, context: &Context<Self>) {
        let registration = context.register(widget);
        self.push_registration(registration, context);
    }

    pub fn push_registration(&mut self, registration: WidgetRegistration, context: &Context<Self>) {
        self.children.push(registration.clone());

        context.send_command(ListCommand::ChildAdded(registration));
    }
}

#[derive(Debug)]
pub enum ListCommand {
    ChildRemoved(WidgetId),
    ChildAdded(WidgetRegistration),
}

impl Widget for List {
    type Command = ListCommand;
    type Event = ();

    const CLASS: &'static str = "gooey-list";
    const FOCUSABLE: bool = false;
}

#[derive(Debug)]
pub struct Builder<K: Key, S: KeyedStorage<K>> {
    storage: S,
    kind: Kind,
    children: Vec<WidgetRegistration>,
    _phantom: PhantomData<K>,
}

impl<K: Key, S: KeyedStorage<K>> Builder<K, S> {
    pub fn storage(&self) -> &WidgetStorage {
        self.storage.storage()
    }

    pub fn bulleted(mut self) -> Self {
        self.kind = Kind::Unordered {
            kind: UnorderedListKind::Bullet,
        };
        self
    }

    pub fn unadorned(mut self) -> Self {
        self.kind = Kind::Unordered {
            kind: UnorderedListKind::None,
        };
        self
    }

    pub fn ordered(mut self, kind: OrderedListKind) -> Self {
        self.kind = Kind::Ordered {
            start: None,
            kind,
            reversed: false,
        };
        self
    }

    pub fn reversed(mut self) -> Self {
        match &mut self.kind {
            Kind::Ordered { reversed, .. } => *reversed = true,
            Kind::Unordered { .. } => panic!("Call reversed() only after calling ordered()"),
        }
        self
    }

    pub fn start_at(mut self, start_at: i32) -> Self {
        match &mut self.kind {
            Kind::Ordered { start, .. } => *start = Some(start_at),
            Kind::Unordered { .. } => panic!("Call start_at() only after calling ordered()"),
        }
        self
    }

    pub fn with<W: Widget>(mut self, widget: StyledWidget<W>) -> Self {
        let widget = self.storage.register(None, widget);
        self.with_registration(widget)
    }

    pub fn with_registration(mut self, registration: WidgetRegistration) -> Self {
        self.children.push(registration);
        self
    }

    pub fn finish(self) -> StyledWidget<List> {
        let is_unadorned = self.kind.is_unadorned();
        let widget = StyledWidget::from(List {
            children: self.children,
            kind: self.kind,
        });
        if is_unadorned {
            widget.with(Padding::default())
        } else {
            widget
        }
    }
}

impl<B: Behavior> Content<B> for List {
    type Builder = Builder<B::Widgets, ComponentBuilder<B>>;

    fn build(storage: ComponentBuilder<B>) -> Self::Builder {
        Builder::new(storage)
    }
}

impl<'a, K: Key, S: KeyedStorage<K>> ContentBuilder<List, K, S> for Builder<K, S> {
    fn new(storage: S) -> Self {
        Self {
            storage,
            kind: Kind::default(),
            children: Vec::default(),
            _phantom: PhantomData,
        }
    }

    fn storage(&self) -> &WidgetStorage {
        self.storage.storage()
    }

    fn related_storage(&self) -> Option<Box<dyn RelatedStorage<K>>> {
        self.storage.related_storage()
    }
}

impl<K: Key, S: KeyedStorage<K>> gooey_core::Builder for Builder<K, S> {
    type Output = StyledWidget<List>;

    fn finish(self) -> Self::Output {
        Builder::finish(self)
    }
}

#[derive(Debug)]
pub struct ListTransmogrifier;

#[derive(Default, Debug, Clone)]
pub struct ListAdornmentSpacing(pub Figure<f32, Scaled>);

impl StyleComponent for ListAdornmentSpacing {}

#[allow(clippy::map_entry)]
#[must_use]
pub fn item_label(value: Option<i32>, kind: &Kind) -> Option<Cow<'static, str>> {
    match kind {
        Kind::Unordered { kind } => match kind {
            UnorderedListKind::None => None,
            UnorderedListKind::Bullet => {
                // bullet point
                Some(Cow::Borrowed("\u{2022}"))
            }
            UnorderedListKind::Circle => Some(Cow::Borrowed("\u{25E6}")),
            UnorderedListKind::Square => Some(Cow::Borrowed("\u{25AA}")),
        },
        Kind::Ordered { kind, .. } => {
            let value = value.expect("value is required for ordered indicators");
            match kind {
                OrderedListKind::Decimal => Some(Cow::Owned(format!("{}.", value))),
                OrderedListKind::AlphaLower => Some(Cow::Owned(alpha(value, 'a'))),
                OrderedListKind::AlphaUpper => Some(Cow::Owned(alpha(value, 'A'))),
                OrderedListKind::RomanLower | OrderedListKind::RomanUpper => {
                    let sign = if value.is_negative() { "-" } else { "" };
                    let as_positive = value.abs() as u32;
                    let roman = Roman::from(as_positive).unwrap();
                    Some(Cow::Owned(if matches!(kind, OrderedListKind::RomanUpper) {
                        format!("{}{}.", sign, roman.to_uppercase())
                    } else {
                        format!("{}{}.", sign, roman.to_lowercase())
                    }))
                }
            }
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn alpha(mut value: i32, a: char) -> String {
    let mut output = String::new();
    while value > 0 {
        let ch = (a as u8 + (value % 26) as u8) as char;
        output.push(ch);
        value /= 26;
    }
    output.push('.');
    output
}

#[must_use]
pub struct ItemLabelIterator<'a> {
    pub kind: &'a Kind,
    pub value: Option<i32>,
    increment: i32,
    remaining: usize,
}

impl<'a> ItemLabelIterator<'a> {
    pub fn new(kind: &'a Kind, count: usize) -> Self {
        let (value, increment) = match kind {
            Kind::Unordered { .. } => (None, 0),
            Kind::Ordered {
                start, reversed, ..
            } => (Some(start.unwrap_or(1)), if *reversed { -1 } else { 1 }),
        };

        Self {
            kind,
            value,
            increment,
            remaining: count,
        }
    }
}

impl<'a> Iterator for ItemLabelIterator<'a> {
    type Item = Option<Cow<'static, str>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            self.remaining -= 1;
            let indicator = item_label(self.value, self.kind);
            self.value = self.value.map(|value| value.wrapping_add(self.increment));
            Some(indicator)
        }
    }
}
