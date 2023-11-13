use std::mem;
use std::sync::{Arc, Mutex, PoisonError};

use ahash::AHashMap;
use alot::{LotId, Lots};
use kludgine::figures::units::Px;
use kludgine::figures::{Point, Rect};

use crate::styles::{Styles, ThemePair, VisualOrder};
use crate::value::Value;
use crate::widget::{ManagedWidget, WidgetId, WidgetInstance};
use crate::window::ThemeMode;

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
        let (effective_styles, parent_id) = if let Some(parent) = parent {
            (
                data.nodes[parent.node_id].child_styles(),
                Some(parent.node_id),
            )
        } else {
            (Styles::default(), None)
        };
        let node_id = data.nodes.push(Node {
            widget: widget.clone(),
            children: Vec::new(),
            parent: parent_id,
            layout: None,
            associated_styles: None,
            effective_styles,
            theme: None,
            theme_mode: None,
        });
        data.nodes_by_id.insert(id, node_id);
        if widget.is_default() {
            data.defaults.push(node_id);
        }
        if widget.is_escape() {
            data.escapes.push(node_id);
        }
        if let Some(parent) = parent_id {
            let parent = &mut data.nodes[parent];
            parent.children.push(node_id);
        }
        if let Some(next_focus) = widget
            .next_focus()
            .and_then(|id| data.nodes_by_id.get(&id))
            .copied()
        {
            data.previous_focuses.insert(next_focus, node_id);
        }
        ManagedWidget {
            node_id,
            widget,
            tree: self.clone(),
        }
    }

    pub fn remove_child(&self, child: &ManagedWidget, parent: &ManagedWidget) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.remove_child(child.node_id, parent.node_id);

        if child.widget.is_default() {
            data.defaults.retain(|id| *id != child.node_id);
        }
        if child.widget.is_escape() {
            data.escapes.retain(|id| *id != child.node_id);
        }
    }

    pub(crate) fn set_layout(&self, widget: LotId, rect: Rect<Px>) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);

        let node = &mut data.nodes[widget];
        node.layout = Some(rect);
        let mut children_to_offset = node.children.clone();
        while let Some(child) = children_to_offset.pop() {
            if let Some(layout) = data
                .nodes
                .get_mut(child)
                .and_then(|child| child.layout.as_mut())
            {
                layout.origin += rect.origin;
                children_to_offset.extend(data.nodes[child].children.iter().copied());
            }
        }
    }

    pub(crate) fn layout(&self, widget: LotId) -> Option<Rect<Px>> {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.nodes.get(widget).and_then(|widget| widget.layout)
    }

    pub(crate) fn reset_render_order(&self) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.render_order.clear();
    }

    pub(crate) fn note_widget_rendered(&self, widget: LotId) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.render_order.push(widget);
    }

    pub(crate) fn reset_child_layouts(&self, parent: LotId) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        let children = data.nodes[parent].children.clone();
        for child in children {
            data.nodes.get_mut(child).expect("missing widget").layout = None;
        }
    }

    pub(crate) fn visually_ordered_children(
        &self,
        parent: LotId,
        order: VisualOrder,
    ) -> Vec<ManagedWidget> {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        let node = &data.nodes[parent];
        let mut unordered = node.children.clone();
        let mut ordered = Vec::<ManagedWidget>::with_capacity(unordered.len());
        loop {
            // Identify the next "row" of widgets by finding the top of a widget that is the closest to the origin of
            let mut min_vertical = order.vertical.max_px();
            let mut max_vertical = order.vertical.max_px();

            let mut index = 0;
            while index < unordered.len() {
                let Some(layout) = &data.nodes[unordered[index]].layout else {
                    unordered.remove(index);
                    continue;
                };
                let top = layout.origin.y;
                let bottom = top + layout.size.height;
                min_vertical = order.vertical.smallest_px(min_vertical, top);
                max_vertical = order.vertical.smallest_px(min_vertical, bottom);

                index += 1;
            }

            if unordered.is_empty() {
                break;
            }

            // Find all widgets whose top is within the range found.
            index = 0;
            let row_base = ordered.len();
            while index < unordered.len() {
                let top_left = data.nodes[unordered[index]]
                    .layout
                    .expect("all have layouts")
                    .origin;
                if min_vertical <= top_left.y && top_left.y <= max_vertical {
                    ordered.push(
                        data.widget_from_node(unordered.remove(index), self)
                            .expect("widget is owned"),
                    );
                } else {
                    index += 1;
                }
            }

            ordered[row_base..].sort_unstable_by_key(|managed| {
                order.horizontal.sort_key(
                    &data.nodes[managed.node_id]
                        .layout
                        .expect("all have layouts"),
                )
            });
        }
        ordered
    }

    pub(crate) fn effective_styles(&self, id: LotId) -> Styles {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.nodes[id].effective_styles.clone()
    }

    pub(crate) fn hover(&self, new_hover: Option<&ManagedWidget>) -> HoverResults {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        let hovered = new_hover
            .map(|new_hover| data.widget_hierarchy(new_hover.node_id, self))
            .unwrap_or_default();
        let unhovered = match data.update_tracked_widget(new_hover, self, |data| &mut data.hover) {
            Ok(Some(old_hover)) => {
                let mut old_hovered = data.widget_hierarchy(old_hover.node_id, self);
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
        data.widget_from_id(id, self)
    }

    pub(crate) fn widget_from_node(&self, id: LotId) -> Option<ManagedWidget> {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.widget_from_node(id, self)
    }

    pub(crate) fn active_widget(&self) -> Option<LotId> {
        self.data
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g)
            .active
    }

    pub(crate) fn hovered_widget(&self) -> Option<LotId> {
        self.data
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g)
            .hover
    }

    pub(crate) fn default_widget(&self) -> Option<LotId> {
        self.data
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g)
            .defaults
            .last()
            .copied()
    }

    pub(crate) fn escape_widget(&self) -> Option<LotId> {
        self.data
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g)
            .escapes
            .last()
            .copied()
    }

    pub(crate) fn is_hovered(&self, id: LotId) -> bool {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        let mut search = data.hover;
        while let Some(hovered) = search {
            if hovered == id {
                return true;
            }
            search = data.nodes.get(hovered).and_then(|node| node.parent);
        }

        false
    }

    pub(crate) fn focused_widget(&self) -> Option<LotId> {
        self.data
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g)
            .focus
    }

    pub(crate) fn widgets_at_point(&self, point: Point<Px>) -> Vec<ManagedWidget> {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        let mut hits = Vec::new();
        for id in data.render_order.iter().rev() {
            if let Some(last_rendered) = data.nodes.get(*id).and_then(|widget| widget.layout) {
                if last_rendered.contains(point) {
                    hits.push(data.widget_from_node(*id, self).expect("just accessed"));
                }
            }
        }
        hits
    }

    pub(crate) fn parent(&self, id: LotId) -> Option<LotId> {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.nodes.get(id).expect("missing widget").parent
    }

    pub(crate) fn attach_styles(&self, id: LotId, styles: Value<Styles>) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.attach_styles(id, styles);
    }

    pub(crate) fn attach_theme(&self, id: LotId, theme: Value<ThemePair>) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.nodes.get_mut(id).expect("missing widget").theme = Some(theme);
    }

    pub(crate) fn attach_theme_mode(&self, id: LotId, theme: Value<ThemeMode>) {
        let mut data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        data.nodes.get_mut(id).expect("missing widget").theme_mode = Some(theme);
    }

    pub(crate) fn overriden_theme(
        &self,
        id: LotId,
    ) -> (Styles, Option<Value<ThemePair>>, Option<Value<ThemeMode>>) {
        let data = self.data.lock().map_or_else(PoisonError::into_inner, |g| g);
        let node = data.nodes.get(id).expect("missing widget");
        (
            node.effective_styles.clone(),
            node.theme.clone(),
            node.theme_mode.clone(),
        )
    }
}

pub(crate) struct HoverResults {
    pub unhovered: Vec<ManagedWidget>,
    pub hovered: Vec<ManagedWidget>,
}

#[derive(Default)]
struct TreeData {
    nodes: Lots<Node>,
    nodes_by_id: AHashMap<WidgetId, LotId>,
    active: Option<LotId>,
    focus: Option<LotId>,
    hover: Option<LotId>,
    defaults: Vec<LotId>,
    escapes: Vec<LotId>,
    render_order: Vec<LotId>,
    previous_focuses: AHashMap<LotId, LotId>,
}

impl TreeData {
    fn widget_from_id(&self, id: WidgetId, tree: &Tree) -> Option<ManagedWidget> {
        let node_id = *self.nodes_by_id.get(&id)?;
        Some(ManagedWidget {
            node_id,
            widget: self.nodes[node_id].widget.clone(),
            tree: tree.clone(),
        })
    }

    fn widget_from_node(&self, node_id: LotId, tree: &Tree) -> Option<ManagedWidget> {
        Some(ManagedWidget {
            node_id,
            widget: self.nodes.get(node_id)?.widget.clone(),
            tree: tree.clone(),
        })
    }

    fn attach_styles(&mut self, id: LotId, styles_value: Value<Styles>) {
        let node = &mut self.nodes[id];
        node.associated_styles = Some(styles_value);
        if !node.children.is_empty() {
            // We had previously associated styles, we need to rebuild all
            // children effective styles
            let child_styles = node.child_styles();
            let children = node.children.clone();
            self.update_effective_styles(&child_styles, children);
        }
    }

    fn update_node_effective_styles(&mut self, id: LotId, effective_styles: &Styles) {
        let node = &mut self.nodes[id];
        node.effective_styles = effective_styles.clone();
        if !node.children.is_empty() {
            let child_styles = node.child_styles();
            let children = node.children.clone();
            self.update_effective_styles(&child_styles, children);
        }
    }

    fn update_effective_styles(&mut self, effective_styles: &Styles, nodes_to_update: Vec<LotId>) {
        for node in nodes_to_update {
            self.update_node_effective_styles(node, effective_styles);
        }
    }

    fn remove_child(&mut self, child: LotId, parent: LotId) {
        let removed_node = self.nodes.remove(child).expect("widget already removed");
        self.nodes_by_id.remove(&removed_node.widget.id());

        let parent = &mut self.nodes[parent];
        let index = parent
            .children
            .iter()
            .enumerate()
            .find_map(|(index, c)| (*c == child).then_some(index))
            .expect("child not found in parent");
        parent.children.remove(index);
        let mut detached_nodes = removed_node.children;

        if let Some(next_focus) = removed_node
            .widget
            .next_focus()
            .and_then(|id| self.nodes_by_id.get(&id))
        {
            self.previous_focuses.remove(next_focus);
        }

        while let Some(node) = detached_nodes.pop() {
            let mut node = self.nodes.remove(node).expect("detached node missing");
            self.nodes_by_id.remove(&node.widget.id());
            detached_nodes.append(&mut node.children);
        }
    }

    pub(crate) fn widget_hierarchy(&self, mut widget: LotId, tree: &Tree) -> Vec<ManagedWidget> {
        let mut hierarchy = Vec::new();
        while let Some(managed) = self.widget_from_node(widget, tree) {
            hierarchy.push(managed);
            let Some(parent) = self.nodes.get(widget).and_then(|widget| widget.parent) else {
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
        property: impl FnOnce(&mut Self) -> &mut Option<LotId>,
    ) -> Result<Option<ManagedWidget>, ()> {
        match (
            mem::replace(property(self), new_widget.map(|w| w.node_id)),
            new_widget,
        ) {
            (Some(old_widget), Some(new_widget)) if old_widget == new_widget.node_id => Err(()),
            (Some(old_widget), _) => Ok(self.widget_from_node(old_widget, tree)),
            (None, _) => Ok(None),
        }
    }
}

pub struct Node {
    pub widget: WidgetInstance,
    pub children: Vec<LotId>,
    pub parent: Option<LotId>,
    pub layout: Option<Rect<Px>>,
    pub associated_styles: Option<Value<Styles>>,
    pub effective_styles: Styles,
    pub theme: Option<Value<ThemePair>>,
    pub theme_mode: Option<Value<ThemeMode>>,
}

impl Node {
    fn child_styles(&self) -> Styles {
        let mut effective_styles = self.effective_styles.clone();
        if let Some(associated) = &self.associated_styles {
            effective_styles.append(associated.get());
        }
        effective_styles
    }
}
