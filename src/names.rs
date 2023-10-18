use std::borrow::Cow;
use std::ops::Deref;

use interner::global::{GlobalString, StringPool};

static NAMES: StringPool = StringPool::new();

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Name(GlobalString);

impl Name {
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
