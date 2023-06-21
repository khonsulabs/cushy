use std::sync::{Arc, Mutex};

use alot::{LotId, Lots};
use gooey_reactor::Value;
use stylecs::{Name, Style};

use crate::ActiveContext;

#[derive(Clone)]
pub struct WidgetTree {
    data: Arc<Mutex<Data>>,
}

impl WidgetTree {
    pub fn new(widget: Name, context: &ActiveContext) -> TreeScope {
        let mut nodes = Lots::new();
        let root = WidgetId(nodes.push(WidgetNode::new(widget, None, context)));
        TreeScope {
            tree: Self {
                data: Arc::new(Mutex::new(Data { nodes, root })),
            },
            id: root,
        }
    }
}

pub struct TreeScope {
    tree: WidgetTree,
    id: WidgetId,
}

impl TreeScope {
    pub fn new_child(
        &self,
        widget: Name,
        name: Option<Name>,
        id: Option<Name>,
        style: Value<Style>,
    ) -> Self {
        let mut data = self.tree.data.lock().expect("lock poisoned");
        let id = WidgetId(data.nodes.push(WidgetNode {
            widget,
            name,
            id,
            style,
            parent_id: Some(self.id),
            children: Vec::new(),
        }));
        data.nodes[self.id.0].children.push(id);

        Self {
            id,
            tree: self.tree.clone(),
        }
    }
}

impl Drop for TreeScope {
    fn drop(&mut self) {
        let mut data = self.tree.data.lock().expect("lock poisoned");
        let removed = data.nodes.remove(self.id.0).expect("node not found");
        if let Some(parent_id) = removed.parent_id {
            data.nodes[parent_id.0]
                .children
                .retain(|child| self.id != *child);
        }
        assert!(
            removed.children.is_empty(),
            "children nodes not dropped before the parent"
        );
    }
}

struct Data {
    nodes: Lots<WidgetNode>,
    root: WidgetId,
}

struct WidgetNode {
    widget: Name,
    name: Option<Name>,
    id: Option<Name>,
    style: Value<Style>,
    parent_id: Option<WidgetId>,
    children: Vec<WidgetId>,
}

impl WidgetNode {
    pub fn new(widget: Name, parent_id: Option<WidgetId>, context: &ActiveContext) -> Self {
        Self {
            widget,
            style: context.new_value(Style::new()),
            parent_id,
            children: Vec::new(),
            name: None,
            id: None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct WidgetId(LotId);
