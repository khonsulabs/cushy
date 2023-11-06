use std::collections::HashMap;
use std::fmt::Debug;
use std::mem;
use std::sync::atomic::{self, AtomicU64};
use std::sync::{Arc, Mutex, PoisonError};

use kludgine::figures::units::Px;
use kludgine::figures::{Point, Rect};

use crate::styles::{ComponentDefaultvalue, ComponentDefinition, ComponentType, Styles};
use crate::widget::{ManagedWidget, WidgetInstance};

#[derive(Clone, Default)]
pub struct Tree {
    data: Arc<Mutex<TreeData>>,
}

impl Tree {
    pub fn push_boxed(
        &self,
        widget: WidgetInstance,
        parent: Option<&ManagedWidget>,
    ) -> ManagedWidget {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        let id = widget.id();
        data.nodes.insert(
            id,
            Node {
                widget: widget.clone(),
                children: Vec::new(),
                parent: parent.map(ManagedWidget::id),
                layout: None,
                styles: None,
            },
        );
        if let Some(parent) = parent {
            let parent = data.nodes.get_mut(&parent.id()).expect("missing parent");
            parent.children.push(id);
        }
        ManagedWidget {
            widget,
            tree: self.clone(),
        }
    }

    pub fn remove_child(&self, child: &ManagedWidget, parent: &ManagedWidget) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.remove_child(child.id(), parent.id());
    }

    pub(crate) fn set_layout(&self, widget: WidgetId, rect: Rect<Px>) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);

        data.render_order.push(widget);
        let node = data.nodes.get_mut(&widget).expect("missing widget");
        node.layout = Some(rect);
        let mut children_to_offset = node.children.clone();
        while let Some(child) = children_to_offset.pop() {
            if let Some(layout) = data
                .nodes
                .get_mut(&child)
                .and_then(|child| child.layout.as_mut())
            {
                layout.origin += rect.origin;
                children_to_offset.extend(data.nodes[&child].children.iter().copied());
            }
        }
    }

    pub(crate) fn layout(&self, widget: WidgetId) -> Option<Rect<Px>> {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.nodes[&widget].layout
    }

    pub(crate) fn reset_render_order(&self) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.render_order.clear();
    }

    pub(crate) fn reset_child_layouts(&self, parent: WidgetId) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        let children = data.nodes[&parent].children.clone();
        for child in children {
            data.nodes.get_mut(&child).expect("missing widget").layout = None;
        }
    }

    pub(crate) fn hover(&self, new_hover: Option<&ManagedWidget>) -> HoverResults {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        let hovered = new_hover
            .map(|new_hover| data.widget_hierarchy(new_hover.id(), self))
            .unwrap_or_default();
        let unhovered = match data.update_tracked_widget(new_hover, self, |data| &mut data.hover) {
            Ok(Some(old_hover)) => {
                let mut old_hovered = data.widget_hierarchy(old_hover.id(), self);
                // For any widgets that were shared, remove them, as they don't
                // need to have their events fired again.
                let mut new_index = 0;
                while !old_hovered.is_empty() && old_hovered.get(0) == hovered.get(new_index) {
                    old_hovered.remove(0);
                    new_index += 1;
                }
                old_hovered
            }
            _ => Vec::new(),
        };
        HoverResults { unhovered, hovered }
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

    pub fn widget(&self, id: WidgetId) -> Option<ManagedWidget> {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.widget(id, self)
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

    pub fn is_hovered(&self, id: WidgetId) -> bool {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        let mut search = data.hover;
        while let Some(hovered) = search {
            if hovered == id {
                return true;
            }
            search = data.nodes[&hovered].parent;
        }

        false
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
            if let Some(last_rendered) = data.nodes[id].layout {
                if last_rendered.contains(point) {
                    hits.push(ManagedWidget {
                        widget: data.nodes[id].widget.clone(),
                        tree: self.clone(),
                    });
                }
            }
        }
        hits
    }

    pub(crate) fn parent(&self, id: WidgetId) -> Option<WidgetId> {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.nodes.get(&id).expect("missing widget").parent
    }

    pub(crate) fn attach_styles(&self, id: WidgetId, styles: Styles) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.nodes.get_mut(&id).expect("missing widget").styles = Some(styles);
    }

    pub fn query_styles(
        &self,
        perspective: &ManagedWidget,
        query: &[&dyn ComponentDefaultvalue],
    ) -> Styles {
        self.data
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g)
            .query_styles(perspective.id(), query)
    }

    pub fn query_style<Component: ComponentDefinition>(
        &self,
        perspective: &ManagedWidget,
        component: &Component,
    ) -> Component::ComponentType {
        self.data
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g)
            .query_style(perspective.id(), component)
    }
}

pub(crate) struct HoverResults {
    pub unhovered: Vec<ManagedWidget>,
    pub hovered: Vec<ManagedWidget>,
}

#[derive(Default)]
struct TreeData {
    nodes: HashMap<WidgetId, Node>,
    active: Option<WidgetId>,
    focus: Option<WidgetId>,
    hover: Option<WidgetId>,
    render_order: Vec<WidgetId>,
}

impl TreeData {
    fn widget(&self, id: WidgetId, tree: &Tree) -> Option<ManagedWidget> {
        Some(ManagedWidget {
            widget: self.nodes.get(&id)?.widget.clone(),
            tree: tree.clone(),
        })
    }

    fn remove_child(&mut self, child: WidgetId, parent: WidgetId) {
        let removed_node = self.nodes.remove(&child).expect("widget already removed");
        let parent = self.nodes.get_mut(&parent).expect("missing widget");
        let index = parent
            .children
            .iter()
            .enumerate()
            .find_map(|(index, c)| (*c == child).then_some(index))
            .expect("child not found in parent");
        parent.children.remove(index);
        let mut detached_nodes = removed_node.children;

        while let Some(node) = detached_nodes.pop() {
            let mut node = self.nodes.remove(&node).expect("detached node missing");
            detached_nodes.append(&mut node.children);
        }
    }

    pub(crate) fn widget_hierarchy(&self, mut widget: WidgetId, tree: &Tree) -> Vec<ManagedWidget> {
        let mut hierarchy = Vec::new();
        while let Some(managed) = self.widget(widget, tree) {
            hierarchy.push(managed);
            let Some(parent) = self.nodes[&widget].parent else {
                break;
            };
            widget = parent;
        }

        hierarchy.reverse();

        hierarchy
    }

    fn update_tracked_widget(
        &mut self,
        new_widget: Option<&ManagedWidget>,
        tree: &Tree,
        property: impl FnOnce(&mut Self) -> &mut Option<WidgetId>,
    ) -> Result<Option<ManagedWidget>, ()> {
        match (
            mem::replace(property(self), new_widget.map(ManagedWidget::id)),
            new_widget,
        ) {
            (Some(old_widget), Some(new_widget)) if old_widget == new_widget.id() => Err(()),
            (Some(old_widget), _) => Ok(Some(ManagedWidget {
                widget: self.nodes[&old_widget].widget.clone(),
                tree: tree.clone(),
            })),
            (None, _) => Ok(None),
        }
    }

    fn query_styles(
        &self,
        mut perspective: WidgetId,
        query: &[&dyn ComponentDefaultvalue],
    ) -> Styles {
        let mut query = query.iter().map(|n| n.name()).collect::<Vec<_>>();
        let mut resolved = Styles::new();
        while !query.is_empty() {
            let node = &self.nodes[&perspective];
            if let Some(styles) = &node.styles {
                query.retain(|name| {
                    if let Some(component) = styles.get(name) {
                        resolved.insert(name, component.clone());
                        false
                    } else {
                        true
                    }
                });
            }
            let Some(parent) = node.parent else { break };
            perspective = parent;
        }
        resolved
    }

    fn query_style<Component: ComponentDefinition>(
        &self,
        mut perspective: WidgetId,
        query: &Component,
    ) -> Component::ComponentType {
        let name = query.name();
        loop {
            let node = &self.nodes[&perspective];
            if let Some(styles) = &node.styles {
                if let Some(component) = styles.get(&name) {
                    let Ok(value) =
                        <Component::ComponentType>::try_from_component(component.clone())
                    else {
                        break;
                    };
                    return value;
                }
            }
            let Some(parent) = node.parent else { break };
            perspective = parent;
        }
        query.default_value()
    }
}

pub struct Node {
    pub widget: WidgetInstance,
    pub children: Vec<WidgetId>,
    pub parent: Option<WidgetId>,
    pub layout: Option<Rect<Px>>,
    pub styles: Option<Styles>,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub struct WidgetId(u64);

impl WidgetId {
    pub fn unique() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, atomic::Ordering::Acquire))
    }
}
