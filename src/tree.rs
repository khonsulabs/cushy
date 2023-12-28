use std::mem;
use std::sync::{Arc, Mutex, Weak};

use ahash::AHashMap;
use alot::{LotId, Lots};
use figures::units::{Px, UPx};
use figures::{Point, Rect, Size};

use crate::styles::{Styles, ThemePair, VisualOrder};
use crate::utils::IgnorePoison;
use crate::value::Value;
use crate::widget::{MountedWidget, WidgetId, WidgetInstance};
use crate::window::{ThemeMode, WindowHandle};
use crate::ConstraintLimit;

#[derive(Clone, Default)]
pub struct Tree {
    data: Arc<Mutex<TreeData>>,
}

impl Tree {
    pub fn push_boxed(
        &self,
        widget: WidgetInstance,
        parent: Option<&MountedWidget>,
    ) -> MountedWidget {
        let mut data = self.data.lock().ignore_poison();
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
            last_layout_query: None,
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
        if let Some(next_focus) = widget.next_focus() {
            data.previous_focuses.insert(next_focus, id);
        }
        MountedWidget {
            node_id,
            widget,
            tree: WeakTree(Arc::downgrade(&self.data)),
        }
    }

    pub fn remove_child(&self, child: &MountedWidget, parent: &MountedWidget) {
        let mut data = self.data.lock().ignore_poison();
        data.remove_child(child.node_id, parent.node_id);

        if child.widget.is_default() {
            data.defaults.retain(|id| *id != child.node_id);
        }
        if child.widget.is_escape() {
            data.escapes.retain(|id| *id != child.node_id);
        }
    }

    pub(crate) fn set_layout(&self, widget: LotId, rect: Rect<Px>) {
        let mut data = self.data.lock().ignore_poison();

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
        let data = self.data.lock().ignore_poison();
        data.nodes.get(widget).and_then(|widget| widget.layout)
    }

    pub(crate) fn new_frame(&self, invalidations: impl IntoIterator<Item = WidgetId>) {
        let mut data = self.data.lock().ignore_poison();
        data.render_info.clear();

        for id in invalidations {
            let Some(id) = data.nodes_by_id.get(&id).copied() else {
                continue;
            };

            data.invalidate(id, true);
        }
    }

    pub(crate) fn note_widget_rendered(&self, widget: LotId) {
        let mut data = self.data.lock().ignore_poison();
        let Some(layout) = data.nodes.get(widget).and_then(|node| node.layout) else {
            return;
        };
        data.render_info.push(widget, layout);
    }

    pub(crate) fn begin_layout(
        &self,
        parent: LotId,
        constraints: Size<ConstraintLimit>,
    ) -> Option<Size<UPx>> {
        let mut data = self.data.lock().ignore_poison();

        let node = &mut data.nodes[parent];
        if let Some(cached_layout) = &node.last_layout_query {
            if constraints.width.max() <= cached_layout.constraints.width.max()
                && constraints.height.max() <= cached_layout.constraints.height.max()
            {
                return Some(cached_layout.size);
            }

            node.last_layout_query = None;
        }

        let children = node.children.clone();
        for child in children {
            data.invalidate(child, false);
        }

        None
    }

    pub(crate) fn persist_layout(
        &self,
        id: LotId,
        constraints: Size<ConstraintLimit>,
        size: Size<UPx>,
    ) {
        let mut data = self.data.lock().ignore_poison();
        data.nodes[id].last_layout_query = Some(CachedLayoutQuery { constraints, size });
    }

    pub(crate) fn visually_ordered_children(
        &self,
        parent: LotId,
        order: VisualOrder,
    ) -> Vec<MountedWidget> {
        let data = self.data.lock().ignore_poison();
        let node = &data.nodes[parent];
        let mut unordered = node.children.clone();
        let mut ordered = Vec::<MountedWidget>::with_capacity(unordered.len());
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
        let data = self.data.lock().ignore_poison();
        data.nodes[id].effective_styles.clone()
    }

    pub(crate) fn hover(&self, new_hover: Option<&MountedWidget>) -> HoverResults {
        let mut data = self.data.lock().ignore_poison();
        let hovered = new_hover
            .map(|new_hover| data.widget_hierarchy(new_hover.node_id, self))
            .unwrap_or_default();
        let unhovered =
            match data.update_tracked_widget(new_hover.map(MountedWidget::id), self, |data| {
                &mut data.hover
            }) {
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

    pub fn focus(&self, new_focus: Option<WidgetId>) -> Result<Option<MountedWidget>, ()> {
        let mut data = self.data.lock().ignore_poison();
        data.update_tracked_widget(new_focus, self, |data| &mut data.focus)
    }

    pub fn previous_focus(&self, focus: WidgetId) -> Option<MountedWidget> {
        let data = self.data.lock().ignore_poison();
        let previous = *data.previous_focuses.get(&focus)?;
        data.widget_from_id(previous, self)
    }

    pub fn activate(
        &self,
        new_active: Option<&MountedWidget>,
    ) -> Result<Option<MountedWidget>, ()> {
        let mut data = self.data.lock().ignore_poison();
        data.update_tracked_widget(new_active.map(MountedWidget::id), self, |data| {
            &mut data.active
        })
    }

    pub fn widget(&self, id: WidgetId) -> Option<MountedWidget> {
        let data = self.data.lock().ignore_poison();
        data.widget_from_id(id, self)
    }

    pub(crate) fn widget_from_node(&self, id: LotId) -> Option<MountedWidget> {
        let data = self.data.lock().ignore_poison();
        data.widget_from_node(id, self)
    }

    pub(crate) fn is_enabled(&self, mut id: LotId, context: &WindowHandle) -> bool {
        let data = self.data.lock().ignore_poison();
        loop {
            let Some(node) = data.nodes.get(id) else {
                return false;
            };

            if !node.widget.enabled(context) {
                return false;
            }

            let Some(parent) = node.parent else { break };

            id = parent;
        }

        true
    }

    pub(crate) fn active_widget(&self) -> Option<LotId> {
        self.data.lock().ignore_poison().active
    }

    pub(crate) fn hovered_widget(&self) -> Option<LotId> {
        self.data.lock().ignore_poison().hover
    }

    pub(crate) fn default_widget(&self) -> Option<LotId> {
        self.data.lock().ignore_poison().defaults.last().copied()
    }

    pub(crate) fn escape_widget(&self) -> Option<LotId> {
        self.data.lock().ignore_poison().escapes.last().copied()
    }

    pub(crate) fn is_hovered(&self, id: LotId) -> bool {
        let data = self.data.lock().ignore_poison();
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
        self.data.lock().ignore_poison().focus
    }

    pub(crate) fn widgets_under_point(&self, point: Point<Px>) -> Vec<MountedWidget> {
        let data = self.data.lock().ignore_poison();
        data.render_info.widgets_under_point(point, &data, self)
    }

    pub(crate) fn parent(&self, id: LotId) -> Option<LotId> {
        let data = self.data.lock().ignore_poison();
        data.nodes.get(id).expect("missing widget").parent
    }

    pub(crate) fn is_child(&self, mut id: LotId, possible_parent: &WidgetInstance) -> bool {
        let data = self.data.lock().ignore_poison();
        while let Some(node) = data.nodes.get(id) {
            if &node.widget == possible_parent {
                return true;
            }

            match node.parent {
                Some(parent) => {
                    id = parent;
                }
                None => break,
            }
        }

        false
    }

    pub(crate) fn attach_styles(&self, id: LotId, styles: Value<Styles>) {
        let mut data = self.data.lock().ignore_poison();
        data.attach_styles(id, styles);
    }

    pub(crate) fn attach_theme(&self, id: LotId, theme: Value<ThemePair>) {
        let mut data = self.data.lock().ignore_poison();
        data.nodes.get_mut(id).expect("missing widget").theme = Some(theme);
    }

    pub(crate) fn attach_theme_mode(&self, id: LotId, theme: Value<ThemeMode>) {
        let mut data = self.data.lock().ignore_poison();
        data.nodes.get_mut(id).expect("missing widget").theme_mode = Some(theme);
    }

    pub(crate) fn overriden_theme(
        &self,
        id: LotId,
    ) -> (Styles, Option<Value<ThemePair>>, Option<Value<ThemeMode>>) {
        let data = self.data.lock().ignore_poison();
        let node = data.nodes.get(id).expect("missing widget");
        (
            node.effective_styles.clone(),
            node.theme.clone(),
            node.theme_mode.clone(),
        )
    }

    pub fn invalidate(&self, id: LotId, include_hierarchy: bool) {
        self.data
            .lock()
            .ignore_poison()
            .invalidate(id, include_hierarchy);
    }
}

pub(crate) struct HoverResults {
    pub unhovered: Vec<MountedWidget>,
    pub hovered: Vec<MountedWidget>,
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
    render_info: RenderInfo,
    previous_focuses: AHashMap<WidgetId, WidgetId>,
}

impl TreeData {
    fn widget_from_id(&self, id: WidgetId, tree: &Tree) -> Option<MountedWidget> {
        let node_id = *self.nodes_by_id.get(&id)?;
        Some(MountedWidget {
            node_id,
            widget: self.nodes[node_id].widget.clone(),
            tree: WeakTree(Arc::downgrade(&tree.data)),
        })
    }

    fn widget_from_node(&self, node_id: LotId, tree: &Tree) -> Option<MountedWidget> {
        Some(MountedWidget {
            node_id,
            widget: self.nodes.get(node_id)?.widget.clone(),
            tree: WeakTree(Arc::downgrade(&tree.data)),
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

        if let Some(next_focus) = removed_node.widget.next_focus() {
            self.previous_focuses.remove(&next_focus);
        }

        while let Some(node) = detached_nodes.pop() {
            let mut node = self.nodes.remove(node).expect("detached node missing");
            self.nodes_by_id.remove(&node.widget.id());
            if let Some(next_focus) = node.widget.next_focus() {
                self.previous_focuses.remove(&next_focus);
            }
            detached_nodes.append(&mut node.children);
        }
    }

    pub(crate) fn widget_hierarchy(&self, mut widget: LotId, tree: &Tree) -> Vec<MountedWidget> {
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
        new_widget: Option<WidgetId>,
        tree: &Tree,
        property: impl FnOnce(&mut Self) -> &mut Option<LotId>,
    ) -> Result<Option<MountedWidget>, ()> {
        let new_widget = new_widget.and_then(|w| self.widget_from_id(w, tree));
        match (
            mem::replace(property(self), new_widget.as_ref().map(|w| w.node_id)),
            new_widget,
        ) {
            (Some(old_widget), Some(new_widget)) if old_widget == new_widget.node_id => Err(()),
            (Some(old_widget), _) => Ok(self.widget_from_node(old_widget, tree)),
            (None, _) => Ok(None),
        }
    }

    fn invalidate(&mut self, id: LotId, include_hierarchy: bool) {
        let mut node = &mut self.nodes[id];
        loop {
            node.layout = None;
            node.last_layout_query = None;

            let (true, Some(parent)) = (include_hierarchy, node.parent) else {
                break;
            };
            node = &mut self.nodes[parent];
        }
    }
}

#[derive(Default)]
struct RenderInfo {
    order: Vec<RenderArea>,
}

impl RenderInfo {
    pub fn push(&mut self, node: LotId, region: Rect<Px>) {
        let area = RenderArea::new(node, region);
        self.order.push(area);
    }

    pub fn clear(&mut self) {
        self.order.clear();
    }

    fn widgets_under_point(
        &self,
        point: Point<Px>,
        tree_data: &TreeData,
        tree: &Tree,
    ) -> Vec<MountedWidget> {
        // We pessimistically allocate a vector as if all widgets match, up to a
        // reasonable limit. This should ensure minimal allocations in all but
        // extreme circumstances where widgets are nested with a significant
        // amount of depth.
        let mut hits = Vec::with_capacity(self.order.len().min(256));
        for area in self.order.iter().rev() {
            if area.min.x <= point.x
                && area.min.y <= point.y
                && area.max.x >= point.x
                && area.max.y >= point.y
            {
                let Some(widget) = tree_data.widget_from_node(area.node, tree) else {
                    continue;
                };
                hits.push(widget);
            }
        }
        hits
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
struct RenderArea {
    node: LotId,
    min: Point<Px>,
    max: Point<Px>,
}

impl RenderArea {
    fn new(node: LotId, area: Rect<Px>) -> Self {
        let (min, max) = area.extents();
        Self { node, min, max }
    }
}

struct Node {
    widget: WidgetInstance,
    children: Vec<LotId>,
    parent: Option<LotId>,
    layout: Option<Rect<Px>>,
    last_layout_query: Option<CachedLayoutQuery>,
    associated_styles: Option<Value<Styles>>,
    effective_styles: Styles,
    theme: Option<Value<ThemePair>>,
    theme_mode: Option<Value<ThemeMode>>,
}

impl Node {
    fn child_styles(&self) -> Styles {
        let mut effective_styles = self.effective_styles.clone();
        if let Some(associated) = &self.associated_styles {
            let mut merged = associated.get();
            merged.inherit_from(effective_styles);
            effective_styles = merged;
        } else {
            effective_styles = effective_styles.into_inherited();
        }
        effective_styles
    }
}

struct CachedLayoutQuery {
    constraints: Size<ConstraintLimit>,
    size: Size<UPx>,
}

#[derive(Clone, Debug)]
pub struct WeakTree(Weak<Mutex<TreeData>>);

impl WeakTree {
    pub fn upgrade(&self) -> Option<Tree> {
        self.0.upgrade().map(|data| Tree { data })
    }
}
