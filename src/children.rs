use std::ops::{Index, IndexMut};

use alot::OrderedLots;

use crate::widget::{BoxedWidget, MakeWidget};

#[derive(Debug, Default)]
#[must_use]
pub struct Children {
    ordered: OrderedLots<BoxedWidget>,
}

impl Children {
    pub const fn new() -> Self {
        Self {
            ordered: OrderedLots::new(),
        }
    }

    pub fn with_widget<W>(mut self, widget: W) -> Self
    where
        W: MakeWidget,
    {
        self.ordered.push(widget.make_widget());
        self
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.ordered.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&BoxedWidget> {
        self.ordered.get_by_index(index)
    }

    #[must_use]
    pub fn iter(&self) -> alot::ordered::Iter<'_, BoxedWidget> {
        self.into_iter()
    }
}

impl Index<usize> for Children {
    type Output = BoxedWidget;

    fn index(&self, index: usize) -> &Self::Output {
        &self.ordered[index]
    }
}
impl IndexMut<usize> for Children {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.ordered[index]
    }
}

impl<'a> IntoIterator for &'a Children {
    type IntoIter = alot::ordered::Iter<'a, BoxedWidget>;
    type Item = &'a BoxedWidget;

    fn into_iter(self) -> Self::IntoIter {
        self.ordered.iter()
    }
}
