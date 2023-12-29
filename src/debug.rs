//! Utililies to help debug Cushy apps.

use std::fmt::Debug;

use alot::OrderedLots;

use crate::value::{Dynamic, DynamicReader, ForEach, WeakDynamic};
use crate::widget::{Children, MakeWidget, WidgetInstance};
use crate::widgets::grid::{Grid, GridWidgets};
use crate::window::Window;
use crate::Open;

/// A widget that can provide extra information when debugging.
#[derive(Clone, Default)]
pub struct DebugContext {
    section: Dynamic<DebugSection>,
}

impl DebugContext {
    /// Observes `value` using `label` in this debug context.
    ///
    /// When the final reference to `value` is dropped, this observation will
    /// automatically be removed.
    pub fn observe<T>(&self, label: impl Into<String>, value: &Dynamic<T>)
    where
        T: PartialEq + Clone + Debug + Send + Sync + 'static,
    {
        let reader = value.create_reader();
        let id = self.section.map_ref(|section| {
            section.values.lock().push(Box::new(RegisteredValue {
                label: label.into(),
                value: reader.clone(),
                widget: value
                    .weak_clone()
                    .map_each(|value| format!("{value:?}"))
                    .make_widget(),
            }))
        });
        let this = self.clone();
        reader.on_disconnect(move || {
            this.section
                .map_ref(|section| section.values.lock().remove(id));
        });
    }

    /// Returns a new child context with the given `label`.
    ///
    /// This creates a nested hierarchy of debug contexts. If a section with the
    /// name already exists, a context for the existing section will be
    /// returned.
    #[must_use]
    pub fn section(&self, label: impl Into<String>) -> Self {
        let label = label.into();
        let this = self.section.lock();
        let mut children = this.children.lock();
        let section = if let Some(existing) = children.iter().find_map(|child| {
            child
                .map_ref(|child| child.label == label)
                .then(|| child.clone())
        }) {
            existing
        } else {
            let new_section = Dynamic::new(DebugSection::new(Some(&self.section), label.clone()));
            let mut insert_at = children.len();
            for index in 0..children.len() {
                if children[index].map_ref(|child| label < child.label) {
                    insert_at = index;
                    break;
                }
            }

            children.insert(insert_at, new_section.clone());
            new_section
        };

        Self { section }
    }

    fn into_window(self) -> Window {
        self.section
            .map_ref(|section| section.widget.clone())
            .vertical_scroll()
            .into_window()
            .titled("Cushy Debugger")
    }

    /// Returns true if this debug context has no child sections or observed
    /// values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.section.map_ref(|section| {
            section.children.map_ref(OrderedLots::len) + section.values.map_ref(OrderedLots::len)
        }) == 0
    }
}

impl Open for DebugContext {
    fn open<App>(self, app: &App) -> crate::Result<Option<crate::window::WindowHandle>>
    where
        App: crate::Application + ?Sized,
    {
        self.into_window().open(app)
    }

    fn run_in(self, app: crate::PendingApp) -> crate::Result {
        self.into_window().run_in(app)
    }
}

impl Drop for DebugContext {
    fn drop(&mut self) {
        // If the only two references are this context and the parent owning the
        // child section, then we want to remove the section if nothing was
        // added to it.
        if self.section.instances() == 2 {
            let section = self.section.lock();
            if let Some(parent) = section.parent.clone() {
                let label = section.label.clone();
                drop(section);
                DebugSection::remove_child_section(&parent, &label);
            }
        }
    }
}

trait Observable: Send {
    fn label(&self) -> &str;
    fn alive(&self) -> bool;
    fn widget(&self) -> &WidgetInstance;
}

struct RegisteredValue<T> {
    label: String,
    value: DynamicReader<T>,
    widget: WidgetInstance,
}

impl<T> Observable for RegisteredValue<T>
where
    T: Send,
{
    fn label(&self) -> &str {
        &self.label
    }

    fn alive(&self) -> bool {
        self.value.connected()
    }

    fn widget(&self) -> &WidgetInstance {
        &self.widget
    }
}

struct DebugSection {
    label: String,
    children: Dynamic<OrderedLots<Dynamic<DebugSection>>>,
    values: Dynamic<OrderedLots<Box<dyn Observable>>>,
    widget: WidgetInstance,
    parent: Option<WeakDynamic<DebugSection>>,
}

impl Default for DebugSection {
    fn default() -> Self {
        Self::new(None, String::default())
    }
}

impl DebugSection {
    fn new(parent: Option<&Dynamic<Self>>, label: String) -> Self {
        // Create the grid of observed values
        let values = Dynamic::<OrderedLots<Box<dyn Observable>>>::default();
        let value_grid = Grid::from_rows(values.map_each(|values| {
            values
                .iter()
                .map(|o| [o.label().make_widget(), o.widget().clone()])
                .collect::<GridWidgets<2>>()
        }));

        // Create the list of collapsable sub contexts
        let children = Dynamic::<OrderedLots<Dynamic<DebugSection>>>::default();
        let child_widgets = children.map_each(|children| {
            children
                .iter()
                .map(|section| section.map_ref(|section| section.widget.clone()))
                .collect::<Children>()
        });

        let parent = parent.map(Dynamic::downgrade);
        // Create a cleanup task to remove this section once it becomes empty.
        if let Some(parent) = parent.clone() {
            let label = label.clone();
            (&children, &values)
                .for_each({
                    move |(children, values)| {
                        if children.is_empty() && values.is_empty() {
                            Self::remove_child_section(&parent, &label);
                        }
                    }
                })
                .persist();
        }

        let contents = value_grid
            .and(child_widgets.into_rows())
            .into_rows()
            .make_widget();
        let widget = if label.is_empty() {
            contents
        } else {
            contents
                .disclose()
                .labelled_by(label.as_str())
                .collapsed(false)
                .make_widget()
        };

        Self {
            label,
            children,
            values,
            widget,
            parent,
        }
    }

    fn remove_child_section(parent: &WeakDynamic<DebugSection>, label: &str) {
        if let Some(parent) = parent.upgrade() {
            let parent = parent.lock();
            let mut children = parent.children.lock();
            if let Some(index) = children.iter().enumerate().find_map(|(index, child)| {
                child.map_ref(|child| child.label == label).then_some(index)
            }) {
                children.remove_by_index(index);
            }
        }
    }
}

#[test]
fn empty_child_clears_on_drop() {
    let root = DebugContext::default();
    drop(root.section("child"));
    assert!(root.is_empty());
}
