use std::fmt::Debug;
use std::mem;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use alot::{LotId, Lots};
use kludgine::figures::units::Px;
use kludgine::figures::{Point, Rect};

use crate::widget::{BoxedWidget, Widget};

#[derive(Clone, Default)]
pub struct Tree {
    data: Arc<Mutex<TreeData>>,
}

impl Tree {
    pub fn push<W>(&self, widget: W, parent: Option<&ManagedWidget>) -> ManagedWidget
    where
        W: Widget,
    {
        self.push_boxed(BoxedWidget::new(widget), parent)
    }

    pub fn push_boxed(&self, widget: BoxedWidget, parent: Option<&ManagedWidget>) -> ManagedWidget {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        let id = WidgetId(data.nodes.push(Node {
            widget: widget.clone(),
            children: Vec::new(),
            parent: parent.map(|parent| parent.id),
            last_rendered_location: None,
        }));
        if let Some(parent) = parent {
            let parent = &mut data.nodes[parent.id.0];
            parent.children.push(id);
        }
        ManagedWidget {
            id,
            widget,
            tree: self.clone(),
        }
    }

    #[allow(clippy::needless_pass_by_value)] // This is sort of a destructor type call
    pub fn remove_child(&self, child: ManagedWidget, parent: &ManagedWidget) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.remove_child(child.id, parent.id);
    }

    fn note_rendered_rect(&self, widget: WidgetId, rect: Rect<Px>) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.nodes[widget.0].last_rendered_location = Some(rect);
        data.render_order.push(widget);
    }

    fn last_rendered_at(&self, widget: WidgetId) -> Option<Rect<Px>> {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.nodes[widget.0].last_rendered_location
    }

    pub(crate) fn reset_render_order(&self) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.render_order.clear();
    }

    pub fn hover(&self, new_hover: Option<&ManagedWidget>) -> Result<Option<ManagedWidget>, ()> {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.update_tracked_widget(new_hover, self, |data| &mut data.hover)
    }

    pub fn focus(&self, new_focus: Option<&ManagedWidget>) -> Result<Option<ManagedWidget>, ()> {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.update_tracked_widget(new_focus, self, |data| &mut data.focus)
    }

    pub fn activate(
        &self,
        new_active: Option<&ManagedWidget>,
    ) -> Result<Option<ManagedWidget>, ()> {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.update_tracked_widget(new_active, self, |data| &mut data.active)
    }

    pub fn widget(&self, id: WidgetId) -> ManagedWidget {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        ManagedWidget {
            id,
            widget: data.nodes[id.0].widget.clone(),
            tree: self.clone(),
        }
    }

    pub fn active_widget(&self) -> Option<WidgetId> {
        self.data
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g)
            .active
    }

    pub fn hovered_widget(&self) -> Option<WidgetId> {
        self.data
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g)
            .hover
    }

    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.data
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g)
            .focus
    }

    pub(crate) fn widgets_at_point(&self, point: Point<Px>) -> Vec<ManagedWidget> {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        let mut hits = Vec::new();
        for id in data.render_order.iter().rev() {
            if let Some(last_rendered) = data.nodes[id.0].last_rendered_location {
                if last_rendered.contains(point) {
                    hits.push(ManagedWidget {
                        id: *id,
                        widget: data.nodes[id.0].widget.clone(),
                        tree: self.clone(),
                    });
                }
            }
        }
        hits
    }

    pub(crate) fn parent(&self, id: WidgetId) -> Option<WidgetId> {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.nodes[id.0].parent
    }
}

#[derive(Default)]
struct TreeData {
    nodes: Lots<Node>,
    active: Option<WidgetId>,
    focus: Option<WidgetId>,
    hover: Option<WidgetId>,
    render_order: Vec<WidgetId>,
}

impl TreeData {
    fn remove_child(&mut self, child: WidgetId, parent: WidgetId) {
        let removed_node = self.nodes.remove(child.0).expect("widget already removed");
        let parent = &mut self.nodes[parent.0];
        let index = parent
            .children
            .iter()
            .enumerate()
            .find_map(|(index, c)| (*c == child).then_some(index))
            .expect("child not found in parent");
        parent.children.remove(index);
        let mut detached_nodes = removed_node.children;

        while let Some(node) = detached_nodes.pop() {
            let mut node = self.nodes.remove(node.0).expect("detached node missing");
            detached_nodes.append(&mut node.children);
        }
    }

    fn update_tracked_widget(
        &mut self,
        new_widget: Option<&ManagedWidget>,
        tree: &Tree,
        property: impl FnOnce(&mut Self) -> &mut Option<WidgetId>,
    ) -> Result<Option<ManagedWidget>, ()> {
        match (
            mem::replace(property(self), new_widget.map(|w| w.id)),
            new_widget,
        ) {
            (Some(old_widget), Some(new_widget)) if old_widget == new_widget.id => Err(()),
            (Some(old_widget), _) => Ok(Some(ManagedWidget {
                id: old_widget,
                widget: self.nodes[old_widget.0].widget.clone(),
                tree: tree.clone(),
            })),
            (None, _) => Ok(None),
        }
    }
}

#[derive(Clone)]
pub struct ManagedWidget {
    pub(crate) id: WidgetId,
    pub(crate) widget: BoxedWidget,
    pub(crate) tree: Tree,
}

impl Debug for ManagedWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedWidget")
            .field("id", &self.id)
            .field("widget", &self.widget)
            .finish_non_exhaustive()
    }
}

impl ManagedWidget {
    pub(crate) fn lock(&self) -> MutexGuard<'_, dyn Widget> {
        self.widget.lock()
    }

    pub(crate) fn note_rendered_rect(&self, rect: Rect<Px>) {
        self.tree.note_rendered_rect(self.id, rect);
    }

    pub fn last_rendered_at(&self) -> Option<Rect<Px>> {
        self.tree.last_rendered_at(self.id)
    }

    pub fn active(&self) -> bool {
        self.tree.active_widget() == Some(self.id)
    }

    pub fn hovered(&self) -> bool {
        self.tree.hovered_widget() == Some(self.id)
    }

    pub fn focused(&self) -> bool {
        self.tree.focused_widget() == Some(self.id)
    }

    pub fn parent(&self) -> Option<ManagedWidget> {
        self.tree.parent(self.id).map(|id| self.tree.widget(id))
    }
}

impl PartialEq for ManagedWidget {
    fn eq(&self, other: &Self) -> bool {
        self.widget == other.widget
    }
}

impl PartialEq<BoxedWidget> for ManagedWidget {
    fn eq(&self, other: &BoxedWidget) -> bool {
        &self.widget == other
    }
}

pub struct Node {
    pub widget: BoxedWidget,
    pub children: Vec<WidgetId>,
    pub parent: Option<WidgetId>,
    pub last_rendered_location: Option<Rect<Px>>,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct WidgetId(LotId);
