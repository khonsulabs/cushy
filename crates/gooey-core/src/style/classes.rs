use std::ops::Index;
use std::sync::Arc;

use alot::{LotId, OrderedLots};
use kempt::Map;
use stylecs::{Name, NameKey};

#[derive(Default, Clone, Eq, PartialEq, Debug)]
pub struct Classes {
    data: Arc<ClassesData>,
}

#[derive(Default, Clone, Eq, Debug)]
struct ClassesData {
    ordered: OrderedLots<Name>,
    by_name: Map<NameKey<'static>, LotId>,
}

impl Classes {
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: Arc::new(ClassesData {
                ordered: OrderedLots::new(),
                by_name: Map::new(),
            }),
        }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Arc::new(ClassesData {
                ordered: OrderedLots::with_capacity(capacity),
                by_name: Map::with_capacity(capacity),
            }),
        }
    }

    pub fn push(&mut self, class: Name) -> bool {
        let this = Arc::make_mut(&mut self.data);
        match this.by_name.entry(NameKey::Owned(class)) {
            kempt::map::Entry::Vacant(vacant) => {
                let id = this.ordered.push((**vacant.key()).clone());
                vacant.insert(id);
                true
            }
            kempt::map::Entry::Occupied(_) => false,
        }
    }

    /// Inserts a class at `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is greater than this collection's length.
    pub fn insert(&mut self, index: usize, class: Name) -> bool {
        let this = Arc::make_mut(&mut self.data);
        assert!(index <= this.ordered.len());
        match this.by_name.entry(NameKey::Owned(class)) {
            kempt::map::Entry::Vacant(vacant) => {
                let id = this.ordered.insert(index, (**vacant.key()).clone());
                vacant.insert(id);
                true
            }
            kempt::map::Entry::Occupied(_) => false,
        }
    }

    pub fn remove(&mut self, class: &Name) -> Option<Name> {
        let this = Arc::make_mut(&mut self.data);
        this.by_name
            .remove(&NameKey::Borrowed(class))
            .map(|removed| {
                this.ordered.remove(removed.value);
                removed.into_key().clone().into()
            })
    }

    #[must_use]
    pub fn contains(&self, class: &Name) -> bool {
        self.data.by_name.contains(&NameKey::Borrowed(class))
    }

    #[must_use]
    pub fn contains_all(&self, classes: &Classes) -> bool {
        self.data
            .by_name
            .intersection(&classes.data.by_name)
            .count()
            == classes.len()
    }

    #[must_use]
    pub fn iter(&self) -> alot::ordered::Iter<'_, Name> {
        self.into_iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.data.by_name.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.by_name.is_empty()
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Name> {
        self.data.ordered.get_by_index(index)
    }
}

impl PartialEq for ClassesData {
    fn eq(&self, other: &Self) -> bool {
        self.ordered == other.ordered
    }
}

impl<'a> IntoIterator for &'a Classes {
    type IntoIter = alot::ordered::Iter<'a, Name>;
    type Item = &'a Name;

    fn into_iter(self) -> Self::IntoIter {
        self.data.ordered.iter()
    }
}

impl IntoIterator for Classes {
    type IntoIter = IntoIter;
    type Item = Name;

    fn into_iter(self) -> Self::IntoIter {
        match Arc::try_unwrap(self.data) {
            Ok(data) => IntoIter(IntoIterData::Unwrapped(data.ordered.into_iter())),
            Err(data) => IntoIter(IntoIterData::Wrapped {
                length: data.ordered.len(),
                data,
                index: 0,
            }),
        }
    }
}

impl FromIterator<Name> for Classes {
    fn from_iter<T: IntoIterator<Item = Name>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut classes = Self::with_capacity(iter.size_hint().0);
        for name in iter {
            classes.push(name);
        }
        classes
    }
}

impl Index<usize> for Classes {
    type Output = Name;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data.ordered[index]
    }
}

pub struct IntoIter(IntoIterData);

enum IntoIterData {
    Unwrapped(alot::ordered::IntoIter<Name>),
    Wrapped {
        data: Arc<ClassesData>,
        index: usize,
        length: usize,
    },
}

impl Iterator for IntoIter {
    type Item = Name;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            IntoIterData::Unwrapped(iter) => iter.next(),
            IntoIterData::Wrapped {
                data,
                index,
                length,
            } => {
                let next = index.checked_add(1)?;
                if next < *length {
                    let name = data.ordered[*index].clone();
                    *index = next;
                    Some(name)
                } else {
                    None
                }
            }
        }
    }
}

#[test]
fn classes() {
    let mut classes = Classes::new();
    let a = Name::private("a").unwrap();
    let b = Name::private("b").unwrap();
    let c = Name::private("c").unwrap();
    assert!(classes.push(a.clone()));
    assert!(!classes.push(a.clone()));
    assert!(classes.push(b.clone()));
    assert_eq!(&classes[0], &a);
    assert_eq!(&classes[1], &b);
    assert!(classes.insert(0, c.clone()));
    assert_eq!(&classes[0], &c);
    assert_eq!(&classes[1], &a);
    assert_eq!(&classes[2], &b);
    assert!(classes.remove(&a).is_some());
    assert_eq!(&classes[0], &c);
    assert_eq!(&classes[1], &b);
}
