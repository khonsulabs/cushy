use std::borrow::Cow;
use std::ops::Deref;

use interner::global::{GlobalString, StringPool};

static NAMES: StringPool = StringPool::new();

/// A smart-string type that is used as a "name" in Gooey.
///
/// This type ensures that globably only one instance of any unique wrapped
/// string exists. By ensuring all instances of each unique string are the same
/// exact underlying instance, optimizations can be made that avoid string
/// comparisons.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Name(GlobalString);

impl Name {
    /// Returns a name for the given string.
    pub fn new<'a>(name: impl Into<Cow<'a, str>>) -> Self {
        Self(NAMES.get(name))
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&'a str> for Name {
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
