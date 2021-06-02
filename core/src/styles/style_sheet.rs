use std::{
    any::TypeId,
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::Deref,
};

use stylecs::{Style, StyleComponent};

use crate::Widget;

/// A set of style [`Rule`]s to apply to a program.
#[derive(Default, Debug)]
pub struct StyleSheet {
    /// The rules in the style sheet.
    pub rules: Vec<Rule>,

    /// The rule indexes, organizd by widget [`TypeId`].
    pub rules_by_widget: HashMap<Option<TypeId>, Vec<usize>>,
}

impl StyleSheet {
    /// Uses `W::CLASS` and any [`Classes`] components present in `style` to
    /// apply style rules. The result will prefer the components specified
    /// in `style`, but any components not specified will be provided by
    /// rules that match the id or classes provided.
    #[must_use]
    pub fn effective_style_for<W: Widget>(&self, mut style: Style, state: &State) -> Style {
        let mut possible_rules = Vec::new();
        if let Some(rules) = self.rules_by_widget.get(&Some(TypeId::of::<W>())) {
            possible_rules.extend(rules.clone());
        }

        if let Some(rules) = self.rules_by_widget.get(&None) {
            possible_rules.extend(rules.clone());
        }

        possible_rules.sort_unstable();
        possible_rules.dedup();

        for rule in possible_rules.into_iter().rev() {
            let rule = &self.rules[rule];

            if rule.applies(state, style.get()) {
                style = style.merge_with(&rule.style, false);
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
    pub widget_type_id: Option<TypeId>,
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
            widget_type_id: Some(TypeId::of::<W>()),
            classes: None,
            hovered: None,
            focused: None,
            active: None,
            style: Style::default(),
        }
    }

    /// Returns a rule targeting any widget with `classes`.
    #[must_use]
    pub fn for_classes<C: Into<Classes>>(classes: C) -> Self {
        Self {
            classes: Some(classes.into()),
            widget_type_id: None,
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
    pub fn applies(&self, state: &State, classes: Option<&Classes>) -> bool {
        self.classes_apply(classes)
            && (check_one_state(self.hovered, state.hovered)
                .or_else(|| check_one_state(self.focused, state.focused))
                .or_else(|| check_one_state(self.active, state.active))
                .unwrap_or(true))
    }

    fn classes_apply(&self, classes: Option<&Classes>) -> bool {
        // If classes aren't defined on this rule, return true.
        // If there are, all classes must be contained in the style's Classes value.

        self.classes.as_ref().map_or(true, |required_classes| {
            classes.map_or(false, |classes| required_classes.is_subset(classes))
        })
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
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Classes(HashSet<Cow<'static, str>>);

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
    pub fn new(classes: HashSet<Cow<'static, str>>) -> Self {
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

    /// Returns a Classes instance with a single name.
    #[must_use]
    pub fn single(class: Cow<'static, str>) -> Self {
        let mut set = HashSet::new();
        set.insert(class);
        Self::new(set)
    }

    /// Converts the classes into a `Vec`.
    #[must_use]
    pub fn to_vec(&self) -> Vec<Cow<'static, str>> {
        self.0.clone().into_iter().collect()
    }
}

impl Deref for Classes {
    type Target = HashSet<Cow<'static, str>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for Classes {
    fn from(s: String) -> Self {
        Self::single(Cow::Owned(s))
    }
}

impl From<&'static str> for Classes {
    fn from(s: &'static str) -> Self {
        Self::single(Cow::Borrowed(s))
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
        Self(self.0.union(&other.0).cloned().collect())
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

#[test]
fn classes_merge_test() {
    assert_eq!(
        Classes::from(vec!["a", "b", "c"]).merge(&Classes::from(vec!["c", "b", "a"])),
        Classes::from(vec!["a", "b", "c"])
    );
    assert_eq!(
        Classes::from(vec!["a", "c", "d", "e"]).merge(&Classes::from(vec!["b", "d", "f"])),
        Classes::from(vec!["a", "b", "c", "d", "e", "f"])
    );
}

// #[test]
// fn rule_applies_tests() {
//     let only_hovered = State {
//         hovered: true,
//         ..State::default()
//     };
//     let only_focused = State {
//         focused: true,
//         ..State::default()
//     };
//     let only_active = State {
//         active: true,
//         ..State::default()
//     };

//     assert!(Rule::for_classes("a").when_hovered().applies(&only_hovered));
//     assert!(!Rule::for_classes("a")
//         .when_hovered()
//         .applies(&State::default()));
//     assert!(Rule::for_classes("a")
//         .when_not_hovered()
//         .applies(&State::default()));

//     assert!(Rule::for_classes("a").when_focused().applies(&only_focused));
//     assert!(!Rule::for_classes("a")
//         .when_focused()
//         .applies(&State::default()));
//     assert!(Rule::for_classes("a")
//         .when_not_focused()
//         .applies(&State::default()));

//     assert!(Rule::for_classes("a").when_active().applies(&only_active));
//     assert!(!Rule::for_classes("a")
//         .when_active()
//         .applies(&State::default()));
//     assert!(Rule::for_classes("a")
//         .when_not_active()
//         .applies(&State::default()));

//     assert!(Rule::for_classes("a").applies(&State::default()));
//     assert!(Rule::for_classes("a")
//         .when_not_active()
//         .applies(&State::default()));
//     assert!(Rule::for_classes("a").applies(&only_hovered));
// }

#[test]
fn style_merge_test() {
    let original = Style::default().with(Classes::from("a"));
    let b_style = Style::default().with(Classes::from("b"));

    let merged = original.merge_with(&b_style, false);
    assert_eq!(
        merged.get::<Classes>().expect("no classes"),
        &Classes::from(vec!["a", "b"])
    );

    let merged = original.merge_with(&b_style, true);
    assert_eq!(
        merged.get::<Classes>().expect("no classes"),
        &Classes::from(vec!["a"])
    );
}
