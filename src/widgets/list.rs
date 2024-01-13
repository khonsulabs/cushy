//! A list of elements with optional item indicators.

use std::fmt::Debug;
use std::sync::Arc;

use super::grid::GridWidgets;
use super::input::CowString;
use super::Grid;
use crate::value::{IntoValue, MapEach, Source, Value};
use crate::widget::{MakeWidget, WidgetInstance, WidgetList};

/// A list of items displayed with an optional item indicator.
pub struct List {
    style: Value<ListStyle>,
    children: Value<WidgetList>,
}

impl List {
    /// Returns a new list with the default [`ListStyle`]/
    pub fn new(children: impl IntoValue<WidgetList>) -> Self {
        Self {
            children: children.into_value(),
            style: Value::Constant(ListStyle::default()),
        }
    }
}

/// The style of a [`List`] widget's item indicators.
#[derive(Default, Debug, Clone)]
pub enum ListStyle {
    #[default]
    Disc,
    Custom(Arc<dyn ListIndicator>),
}

impl PartialEq for ListStyle {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Custom(l0), Self::Custom(r0)) => Arc::ptr_eq(l0, r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

/// A [`ListStyle`] implementation that provides an optional indicator for a
/// given list index.
pub trait ListIndicator: Debug + Sync + Send + 'static {
    /// Returns the indicator to use at `index`.
    fn list_indicator(&self, index: usize) -> Option<CowString>;
}

impl ListIndicator for ListStyle {
    fn list_indicator(&self, index: usize) -> Option<CowString> {
        match self {
            ListStyle::Disc => Some(CowString::new("\u{2022}")),
            ListStyle::Custom(style) => style.list_indicator(index),
        }
    }
}

impl MakeWidget for List {
    fn make_widget(self) -> WidgetInstance {
        let rows = match (self.children, self.style) {
            (children, Value::Constant(style)) => {
                children.map_each(move |children| build_grid_widgets(&style, children))
            }
            (Value::Dynamic(children), Value::Dynamic(style)) => Value::Dynamic(
                (&style, &children)
                    .map_each(|(style, children)| build_grid_widgets(style, children)),
            ),
            (Value::Constant(children), Value::Dynamic(style)) => {
                Value::Dynamic(style.map_each(move |style| build_grid_widgets(style, &children)))
            }
        };
        Grid::from_rows(rows).make_widget()
    }
}

fn build_grid_widgets(style: &ListStyle, children: &WidgetList) -> GridWidgets<2> {
    // This is horrible. We should be be using synchronize_with to avoid
    // recreating the gridwidgets every time.
    children
        .iter()
        .enumerate()
        .map(|(index, child)| {
            (
                style.list_indicator(index).unwrap_or_default(),
                child.clone().align_left().make_widget(),
            )
        })
        .collect()
}
