use std::{any::TypeId, borrow::Cow, collections::HashMap, ops::Deref};

use stylecs::{Style, StyleComponent};

use crate::Widget;

/// A set of style [`Rule`]s to apply to a program.
#[derive(Default, Debug)]
pub struct StyleSheet {
    /// The rules in the style sheet.
    pub rules: Vec<Rule>,

    rules_by_widget: HashMap<TypeId, Vec<usize>>,
}

impl StyleSheet {
    /// Uses `W::CLASS` and any [`Classes`] components present in `style` to
    /// apply style rules. The result will prefer the components specified
    /// in `style`, but any components not specified will be provided by
    /// rules that match the id or classes provided.
    #[must_use]
    pub fn effective_style_for<W: Widget>(&self, mut style: Style, state: &State) -> Style {
        if let Some(rules) = self.rules_by_widget.get(&TypeId::of::<W>()) {
            for &rule in rules.iter().rev() {
                let rule = &self.rules[rule];
                // TODO check classes
                if rule.applies(state) {
                    style = style.merge_with(&rule.style, false);
                }
            }
        }

        style
    }

    /// Pushes `rule` and returns self. Builder-style implementation of
    /// [`Self::push()`].
    #[must_use]
    pub fn with(mut self, rule: Rule) -> Self {
        self.push(rule);
        self
    }

    /// Pushes `rule` into the collection. Rules pushed later will have
    /// higher priority than rules that are pushed later.
    pub fn push(&mut self, rule: Rule) {
        let index = self.rules.len();

        let rules = self.rules_by_widget.entry(rule.widget_type_id).or_default();
        rules.push(index);

        self.rules.push(rule);
    }

    /// Merges `self` with `other`, such that the rules in `self` are preferred
    /// to the ones in `other`.
    #[must_use]
    pub fn merge_with(&self, other: &Self) -> Self {
        let mut combined = Self {
            rules: Vec::with_capacity(self.rules.len() + other.rules.len()),
            rules_by_widget: other.rules_by_widget.clone(),
        };
        combined.rules.extend(other.rules.iter().cloned());
        combined.rules.extend(self.rules.iter().cloned());
        let rule_offset = other.rules.len();
        for (&key, index) in &self.rules_by_widget {
            let class_rules = combined.rules_by_widget.entry(key).or_default();
            class_rules.extend(index.iter().map(|&i| i + rule_offset));
        }

        combined
    }
}

/// A style rule.
#[derive(Debug, Clone)]
pub struct Rule {
    /// The [`TypeId`] of the widget this rule is associated with.
    pub widget_type_id: TypeId,
    /// The classes this rule should optionally filter by.
    pub classes: Option<Classes>,
    /// If specified, only applies `style` if `hovered` matches
    /// [`State::hovered`].
    pub hovered: Option<bool>,
    /// If specified, only applies `style` if `focused` matches
    /// [`State::focused`].
    pub focused: Option<bool>,
    /// If specified, only applies `style` if `active` matches
    /// [`State::active`].
    pub active: Option<bool>,
    /// The style to apply if the criteria are met.
    pub style: Style,
}

impl Rule {
    /// Returns a default `Rule` with `selector` of [`Classes`] `classes`.
    #[must_use]
    pub fn for_widget<W: Widget>() -> Self {
        Self {
            widget_type_id: TypeId::of::<W>(),
            classes: None,
            hovered: None,
            focused: None,
            active: None,
            style: Style::default(),
        }
    }

    /// Returns a default `Rule` with `selector` of [`Classes`] `classes`.
    #[must_use]
    pub fn with_classes<C: Into<Classes>>(mut self, classes: C) -> Self {
        self.classes = Some(classes.into());
        self
    }

    /// Builder-style function that sets [`Self::hovered`] to `Some(true)`.
    #[must_use]
    pub const fn when_hovered(mut self) -> Self {
        self.hovered = Some(true);
        self
    }

    /// Builder-style function that sets [`Self::hovered`] to `Some(false)`.
    #[must_use]
    pub const fn when_not_hovered(mut self) -> Self {
        self.hovered = Some(false);
        self
    }

    /// Builder-style function that sets [`Self::focused`] to `Some(true)`.
    #[must_use]
    pub const fn when_focused(mut self) -> Self {
        self.focused = Some(true);
        self
    }

    /// Builder-style function that sets [`Self::focused`] to `Some(false)`.
    #[must_use]
    pub const fn when_not_focused(mut self) -> Self {
        self.focused = Some(false);
        self
    }

    /// Builder-style function that sets [`Self::active`] to `Some(true)`.
    #[must_use]
    pub const fn when_active(mut self) -> Self {
        self.active = Some(true);
        self
    }

    /// Builder-style function that sets [`Self::active`] to `Some(false)`.
    #[must_use]
    pub const fn when_not_active(mut self) -> Self {
        self.active = Some(false);
        self
    }

    /// Builder-style function that passes the current value of [`Self::style`]
    /// into `initializer` and stores the result back into [`Self::style`].
    #[must_use]
    pub fn with_styles<F: FnOnce(Style) -> Style>(mut self, initializer: F) -> Self {
        self.style = initializer(self.style);
        self
    }

    /// Returns true if the rule should apply based on `state`.
    #[must_use]
    pub fn applies(&self, state: &State) -> bool {
        check_one_state(self.hovered, state.hovered)
            .or_else(|| check_one_state(self.focused, state.focused))
            .or_else(|| check_one_state(self.active, state.active))
            .unwrap_or(true)
    }
}

fn check_one_state(condition: Option<bool>, state: bool) -> Option<bool> {
    condition.map(|condition| condition == state)
}

/// A filter for a [`Rule`].
#[derive(Debug, Clone)]
pub enum Selector {
    /// Matches when a [`Style`] has a [`Classes`] component that contains all
    /// of the classes in the contianed value.
    Classes(Classes),
}

/// A list of class names. Not inherited when merging styles.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Classes(Vec<Cow<'static, str>>);

impl Classes {
    /// Returns a new Classes component with the classes passed.
    ///
    /// # Valid Class Names
    ///
    /// Valid characters in class names are:
    ///
    /// * `'a'..='z'`
    /// * `'A'..='Z'`
    /// * `'0'..='9'`
    /// * `'_'`
    /// * `'-'`
    ///
    /// # Panics
    ///
    /// Panics upon an illegal class name.
    // TODO refactor to have an error. Implement TryFrom as well, move the panic into the From
    // implementations.
    #[must_use]
    pub fn new(mut classes: Vec<Cow<'static, str>>) -> Self {
        classes.sort();
        for class in &classes {
            for ch in class.chars() {
                match ch {
                    'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => {}
                    illegal => panic!("invalid character '{}' in class name '{}'", illegal, class),
                }
            }
        }
        Self(classes)
    }
}

impl Deref for Classes {
    type Target = Vec<Cow<'static, str>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for Classes {
    fn from(s: String) -> Self {
        Self::new(vec![Cow::Owned(s)])
    }
}

impl From<&'static str> for Classes {
    fn from(s: &'static str) -> Self {
        Self::new(vec![Cow::Borrowed(s)])
    }
}

impl From<Vec<String>> for Classes {
    fn from(s: Vec<String>) -> Self {
        Self::new(s.into_iter().map(Cow::Owned).collect())
    }
}

impl From<Vec<&'static str>> for Classes {
    fn from(s: Vec<&'static str>) -> Self {
        Self::new(s.into_iter().map(|s| Cow::Borrowed(s)).collect())
    }
}

impl StyleComponent for Classes {
    fn should_be_inherited(&self) -> bool {
        false
    }

    fn merge(&self, other: &Self) -> Self
    where
        Self: Clone,
    {
        Self(
            UniqueOrderedMerge::merge(self.0.iter(), other.0.iter())
                .cloned()
                .collect(),
        )
    }
}

struct UniqueOrderedMerge<T, I>
where
    T: Clone + Ord,
    I: Iterator<Item = T>,
{
    iter1: I,
    iter2: I,
    last_iter1: Option<T>,
    last_iter2: Option<T>,
    last_value: Option<T>,
}

impl<T, I> UniqueOrderedMerge<T, I>
where
    T: Clone + Ord,
    I: Iterator<Item = T>,
{
    pub fn merge(iter1: I, iter2: I) -> Self {
        Self {
            iter1,
            iter2,
            last_iter1: None,
            last_iter2: None,
            last_value: None,
        }
    }

    fn next_item(iter: &mut I, last_value: Option<T>) -> Option<T> {
        if last_value.is_some() {
            last_value
        } else {
            iter.next()
        }
    }
}

impl<T, I> Iterator for UniqueOrderedMerge<T, I>
where
    T: Clone + Ord,
    I: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let item1 = Self::next_item(&mut self.iter1, self.last_iter1.take());
        let item2 = Self::next_item(&mut self.iter2, self.last_iter2.take());

        let resulting_value = match (item1, item2) {
            (Some(item1), Some(item2)) => {
                match item1.cmp(&item2) {
                    std::cmp::Ordering::Less => {
                        self.last_iter2 = Some(item2);
                        Some(item1)
                    }
                    std::cmp::Ordering::Equal => {
                        // When equal, drop one
                        Some(item1)
                    }
                    std::cmp::Ordering::Greater => {
                        self.last_iter1 = Some(item1);
                        Some(item2)
                    }
                }
            }
            (Some(item), None) | (None, Some(item)) => Some(item),
            (None, None) => None,
        };

        if resulting_value.is_some() && self.last_value == resulting_value {
            // When we produce the same value as the last time, automatically get the next
            // value.
            self.next()
        } else {
            self.last_value = resulting_value.clone();
            resulting_value
        }
    }
}

/// An element state.
#[derive(Default, Debug)]
pub struct State {
    /// Whether the element is hovered or not.
    pub hovered: bool,
    /// Whether the element is focused or not.
    pub focused: bool,
    /// Whether the element is active or not. For example, a push button
    /// actively being depressed.
    pub active: bool,
}
