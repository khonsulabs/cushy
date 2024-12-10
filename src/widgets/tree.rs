//! This module provides structures and functionality for managing a tree of widgets.
//!
//! The tree uses `Dynamic` and `WidgetList` to manage dynamic changes in widget hierarchies,
//! such as the expansion and collapsing of nodes.
//!
//! The main components are:
//! - `TreeNodeKey`: A unique identifier for each tree node.
//! - `Tree`: A structure that holds the tree structure, mapping node keys to `TreeNode` instances,
//!   and provides methods for manipulating the tree structure like inserting children or siblings.
//! - `TreeNode`: Represents a node in the tree with information about its parent, depth,
//!   widget instances, and children.
//! - `TreeWidget`: Represents the root widget for displaying tree nodes.
//! - `TreeNodeWidget`: Manages the visual representation of a tree node, including expand/collapse
//!   functionality and the display of child widgets.
//!
//! # Example Usage
//!
//! The `Tree` and `TreeNodeWidget` structures provide methods to construct and manage the tree, 
//! allowing for dynamic UI components that react to user interactions.
//!
//! ```rust
//! use cushy::widget::MakeWidget;
//! let mut tree = Tree::default();
//! let root_widget = "root".to_label().make_widget();
//! let root_key = tree.insert_child(root_widget, None);
//! if let Some(root_key) = root_key {
//!     let child_widget = "child".to_label().make_widget();
//!     tree.insert_child(child_widget, Some(&root_key));
//! }
//! ```
use std::fmt::{Debug, Formatter};
use cushy::figures::units::Px;
use cushy::widget::{MakeWidget, WidgetRef, WrapperWidget};
use cushy::widgets::Space;
use indexmap::IndexMap;
use crate::reactive::value::{Destination, Dynamic, Source, Switchable};
use crate::widget::{WidgetInstance, WidgetList};
use crate::widgets::label::Displayable;


/// A tree node key.
///
/// These are created when adding nodes to a tree.  Keys are never re-used.
#[derive(Default, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TreeNodeKey(usize);

/// Represents a node within a tree structure.
pub struct TreeNode {
    parent: Option<TreeNodeKey>,
    depth: usize,
    child_widget: WidgetInstance,
    children: Dynamic<WidgetList>,
    is_expanded: Dynamic<bool>,
}

/// A widget for a tree node, including the button to collapse/expand any children.
pub struct TreeNodeWidget {
    is_expanded: Dynamic<bool>,
    child: WidgetRef,
    child_height: Option<Px>,
}

impl TreeNodeWidget {
    fn new(child: WidgetInstance, children: Dynamic<WidgetList>, is_expanded: Dynamic<bool>) -> Self {
        let indicator = is_expanded.clone().map_each(|value|{
            match value {
                true => "v",
                false => ">"
            }
        }).into_label();

        let expand_button = indicator.into_button()
            .on_click({
                let is_expanded = is_expanded.clone();
                move |_event| {
                    is_expanded.toggle();
                }
            })
            .make_widget();

        let children_switcher = is_expanded.clone().switcher(move |value, _active| {
            match value {
                false => Space::default().make_widget(),
                true => children.clone().into_rows().make_widget()
            }
        }).make_widget();

        let child = expand_button
            .and(child)
            .into_columns()
            .and(children_switcher)
            .into_rows()
            .into_ref();

        Self {
            is_expanded,
            child,
            child_height: None,
        }
    }
}

impl Debug for TreeNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeNode")
            .field("parent", &self.parent)
            .field("depth", &self.depth)
            .finish()
    }
}

/// A tree of widgets, with expandable/collapsible nodes.
///
/// See module documentation.
#[derive(Debug)]
pub struct Tree {
    nodes: Dynamic<IndexMap<TreeNodeKey, TreeNode>>,
    next_key: TreeNodeKey,
}

/// The widget for a tree.
pub struct TreeWidget {
    root: WidgetRef,
}

impl Default for Tree {
    fn default() -> Self {
        let nodes = Dynamic::new(IndexMap::<TreeNodeKey, TreeNode>::new());

        Self {
            nodes,
            next_key: TreeNodeKey::default(),
        }
    }
}
impl Tree {
    /// Make the widget for the tree.
    ///
    /// The tree is NOT consumed and can be used to manipulate the tree, e.g. add/remove/collapse/expand nodes.
    pub fn make_widget(&self) -> WidgetInstance {
        let root = self.nodes.clone().switcher(|nodes, _active| {
            if nodes.is_empty()  {
                Space::default().make_widget()
            } else {
                let (_root_key, root_node) = nodes.first().unwrap();

                root_node.child_widget.clone()
            }
        }).into_ref();

        TreeWidget {
            root
        }.make_widget()
    }

    fn generate_next_key(&mut self) -> TreeNodeKey {
        let key = self.next_key.clone();
        self.next_key.0 += 1;
        key
    }

    /// Inserts a child after the given parent
    ///
    /// Returns `None` if the given node doesn't exist or is the root node.
    pub fn insert_child(&mut self, value: WidgetInstance, parent: Option<&TreeNodeKey>) -> Option<TreeNodeKey> {
        self.insert_child_with_key(|_key|value, parent)
    }

    /// Inserts a child after the given parent, the key is provided to a callback function.
    ///
    /// This method is useful when creating buttons/widgets that need to manipulate the tree.
    ///
    /// Returns `None` if the given node doesn't exist or is the root node.
    pub fn insert_child_with_key<F>(&mut self, value_f: F, parent: Option<&TreeNodeKey>) -> Option<TreeNodeKey>
    where
        F: FnOnce(TreeNodeKey) -> WidgetInstance
    {
        // Determine whether a new key and node should be created
        let (depth, parent_key) = {
            let nodes = self.nodes.lock();
            if let Some(parent) = parent {
                if let Some(parent_node) = nodes.get(parent) {
                    (Some(parent_node.depth + 1), Some(parent.clone()))
                } else {
                    (None, None) // Parent not found, node won't be inserted
                }
            } else {
                // If no parent is specified, this is a root node
                (Some(0), None)
            }
        };

        // If depth is determined, generate key and create the node
        if let Some(depth) = depth {
            let key = self.generate_next_key(); // Generate key after deciding a node is needed

            let value = value_f(key.clone());

            let children = Dynamic::new(WidgetList::new());
            let is_expanded = Dynamic::new(true);
            let child_widget = TreeNodeWidget::new(value, children.clone(), is_expanded.clone()).make_widget();

            let child_node = TreeNode {
                parent: parent_key.clone(),
                depth,
                is_expanded,
                child_widget,
                children,
            };

            {
                let mut nodes = self.nodes.lock();
                nodes.insert(key.clone(), child_node);
            }

            self.update_children_widgetlist(parent_key);

            Some(key)
        } else {
            None
        }
    }

    fn update_children_widgetlist(&mut self, parent_key: Option<TreeNodeKey>) {
        if let Some(parent_key) = parent_key {
            // regenerate the 'children' widget list for the parent

            let children: WidgetList = self.children_keys(&parent_key)
                .into_iter()
                .map(|key| {
                    let nodes = self.nodes.lock();
                    let node = nodes.get(&key).unwrap();

                    node.child_widget.clone()
                        .make_widget()
                })
                .collect();

            let nodes = self.nodes.lock();
            let parent = nodes.get(&parent_key).unwrap();
            parent.children.set(children);
        }
    }

    /// Inserts a sibling after the given node.
    ///
    /// Returns `None` if the given node doesn't exist or is the root node.
    pub fn insert_after(&mut self, value: WidgetInstance, sibling: &TreeNodeKey) -> Option<TreeNodeKey> {
        self.insert_after_with_key(|_key|value, sibling)
    }


    /// Inserts a sibling after the given node, the key is provided to a callback function.
    ///
    /// This method is useful when creating buttons/widgets that need to manipulate the tree.
    ///
    /// Returns `None` if the given node doesn't exist or is the root node.
    pub fn insert_after_with_key<F>(&mut self, value_f: F, sibling: &TreeNodeKey) -> Option<TreeNodeKey>
    where
        F: FnOnce(TreeNodeKey) -> WidgetInstance
    {
        // FIXME likely the API could be better, so that there is no concept of a 'root' node at all, then this limitation can be removed
        // cannot add siblings to the root, silently ignore.
        if self.nodes.lock().get(sibling).unwrap().parent.is_none() {
            return None
        }

        // Determine whether a new key and node should be created
        let result = {
            let nodes = self.nodes.lock();
            nodes.get_full(sibling).map_or(None, |(sibling_index, _sibling_key, sibling_node)|{
                Some((sibling_index, sibling_node.depth, sibling_node.parent.clone()))
            })
        };

        // If depth is determined, generate key and create the node
        if let Some((sibling_index, depth, parent_key)) = result {
            let key = self.generate_next_key(); // Generate key after deciding a node is needed
            let value = value_f(key.clone());

            let children = Dynamic::new(WidgetList::new());
            let is_expanded = Dynamic::new(true);
            let child_widget = TreeNodeWidget::new(value, children.clone(), is_expanded.clone()).make_widget();

            let child_node = TreeNode {
                parent: parent_key.clone(),
                depth,
                is_expanded,
                child_widget,
                children
            };

            {
                let mut nodes = self.nodes.lock();
                nodes.insert_before(sibling_index + 1, key.clone(), child_node);
            }

            self.update_children_widgetlist(parent_key);

            Some(key)
        } else {
            None
        }
    }

    /// Clears the tree, removing all nodes and resetting the key.
    pub fn clear(&mut self) {
        self.nodes.lock().clear();
        self.next_key = TreeNodeKey::default();
    }

    /// Removes the node and all descendants.
    pub fn remove_node(&mut self, node_key: &TreeNodeKey) {
        let parent_key = {
            let mut nodes = self.nodes.lock();

            // First, check if the node exists
            if !nodes.contains_key(node_key) {
                return;
            }

            let parent_key = nodes[node_key].parent.clone();

            // Create a stack to hold nodes to be removed
            let mut to_remove = vec![node_key.clone()];

            // We perform a DFS traversal to collect all descendant keys
            while let Some(current_key) = to_remove.pop() {
                if let Some(_node) = nodes.shift_remove(&current_key) {
                    // Add children of the current node to the stack
                    nodes
                        .keys()
                        .filter(|&key| nodes[key].parent.as_ref() == Some(&current_key))
                        .for_each(|key| to_remove.push(key.clone()));
                }
            }

            parent_key
        };

        self.update_children_widgetlist(parent_key);
    }

    /// Get an ordered list of children keys for a node.
    ///
    /// Returns 'None' if the given does not exist.
    pub fn children_keys(&self, parent_key: &TreeNodeKey) -> Vec<TreeNodeKey> {
        let nodes = self.nodes.lock();
        nodes.iter()
            .filter_map(|(key, node)| {
                if node.parent.as_ref() == Some(parent_key) {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Expand a node.
    ///
    /// Not recursive, the node's children will retain their collapsed/expanded state when they are
    /// expanded.
    pub fn expand(&self, key: &TreeNodeKey) {
        let nodes = self.nodes.lock();
        let node = nodes.get(key).unwrap();
        node.is_expanded.set(true);
    }

    /// Collapse a node.
    ///
    /// Not recursive, the node's children will retain their collapsed/expanded state when they are
    /// re-expanded.
    pub fn collapse(&self, key: &TreeNodeKey) {
        let nodes = self.nodes.lock();
        let node = nodes.get(key).unwrap();
        node.is_expanded.set(false);
    }
}

#[cfg(test)]
mod tests {
    use crate::widget::MakeWidget;
    use crate::widgets::label::Displayable;
    use super::Tree;
    
    #[test]
    pub fn add_root() {
        // given
        
        let mut tree = Tree::default();
        let root_widget = "root".into_label().make_widget();
        // when
        
        let key = tree.insert_child(root_widget, None).unwrap();

        // then
        let nodes = tree.nodes.lock();

        assert_eq!(key.0, 0);
        assert_eq!(nodes.len(), 1);
        // and
        let root = nodes.get(&key).unwrap();
        
        assert_eq!(root.parent, None);
        assert_eq!(root.depth, 0);
    }
    
    #[test]
    pub fn add_child_to_root() {
        // given
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".make_widget(), None).unwrap();

        // when
        let child_key = tree.insert_child("child".make_widget(), Some(&root_key)).unwrap();

        // then
        let nodes = tree.nodes.lock();

        assert_eq!(child_key.0, 1);
        assert_eq!(nodes.len(), 2);

        // and
        let child = nodes.get(&child_key).unwrap();
        assert_eq!(child.parent, Some(root_key.clone()));
        assert_eq!(child.depth, 1);
    }


    #[test]
    pub fn add_sibling_to_child() {
        // given
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".make_widget(), None).unwrap();
        let first_child_key = tree.insert_child("first_child".make_widget(), Some(&root_key)).unwrap();

        // when
        let sibling_key = tree.insert_after("sibling".make_widget(), &first_child_key).unwrap();

        // then
        let nodes = tree.nodes.lock();
        assert_eq!(nodes.len(), 3);

        // and verify the sibling properties
        let sibling = nodes.get(&sibling_key).unwrap();
        assert_eq!(sibling.parent, Some(root_key.clone()));
        assert_eq!(sibling.depth, 1); // Assuming sibling has the same depth as the first child
    }


    #[test]
    pub fn add_sibling_inbetween() {
        // given
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".make_widget(), None).unwrap();
        let first_child_key = tree.insert_child("first_child".make_widget(), Some(&root_key)).unwrap();
        let second_child_key = tree.insert_child("second_child".make_widget(), Some(&root_key)).unwrap();

        // when
        let middle_sibling_key = tree.insert_after("middle_sibling".make_widget(), &first_child_key).unwrap();

        // then ensure the order is correct, i.e., after first_child and before second_child
        let children = tree.children_keys(&root_key);
        let index_of_middle = children.iter().position(|k| k == &middle_sibling_key).unwrap();
        assert_eq!(children[index_of_middle - 1], first_child_key);
        assert_eq!(children[index_of_middle + 1], second_child_key);

        // and
        let nodes = tree.nodes.lock();
        assert_eq!(nodes.len(), 4);
    }


    #[test]
    pub fn remove_node() {
        // given
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".make_widget(), None).unwrap();
        let child_key = tree.insert_child("child".make_widget(), Some(&root_key)).unwrap();
        let _descendant_key = tree.insert_child("descendant".make_widget(), Some(&child_key)).unwrap();

        // node to be removed
        let node_to_remove = root_key.clone();

        // assume we have a remove_node method
        tree.remove_node(&node_to_remove);

        // then
        let nodes = tree.nodes.lock();
        nodes.iter().for_each(|(key, node)| {
            println!("key: {:?}: node: {:?}", key, node);
        });
        // and root, child and descendant nodes should be removed
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    pub fn remove_child_node() {
        // given
        
        // Root
        // +- 1
        // |  +- 3
        // +- 2
        // |  +- 4
        
        
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".make_widget(), None).unwrap();
        // direct children
        let key_1 = tree.insert_child("1".make_widget(), Some(&root_key)).unwrap();
        let key_2 = tree.insert_child("2".make_widget(), Some(&root_key)).unwrap();
        // descendants
        let key_3 = tree.insert_child("3".make_widget(), Some(&key_1)).unwrap();
        let key_4 = tree.insert_child("3".make_widget(), Some(&key_2)).unwrap();

        // ensure they exist before removal
        {
            let nodes = tree.nodes.lock();
            assert_eq!(nodes.len(), 5);
        }
        
        // node to be removed
        let node_to_remove = key_1.clone();

        // when
        tree.remove_node(&node_to_remove);

        // then the root node should remain
        let nodes = tree.nodes.lock();

        assert_eq!(nodes.len(), 3);
        assert!(nodes.get(&root_key).is_some());

        // other elements should remain
        assert!(nodes.get(&key_2).is_some());
        assert!(nodes.get(&key_4).is_some());

        // and child and children should be removed
        assert!(nodes.get(&key_1).is_none());
        assert!(nodes.get(&key_3).is_none());
    }

    #[test]
    pub fn children_keys() {
        // given
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".make_widget(), None).unwrap();
        let child_key_1 = tree.insert_child("child_1".make_widget(), Some(&root_key)).unwrap();
        let child_key_2 = tree.insert_child("child_2".make_widget(), Some(&root_key)).unwrap();

        // when
        let children = tree.children_keys(&root_key);

        // then
        assert_eq!(children.len(), 2);
        assert!(children.contains(&child_key_1));
        assert!(children.contains(&child_key_2));
    }
}

impl Debug for TreeNodeWidget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeNodeWidget")
            .field("is_expanded", &self.is_expanded)
            .field("child", &self.child)
            .field("child_height", &self.child_height)
            .finish()
    }
}

impl WrapperWidget for TreeNodeWidget {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }
}

impl Debug for TreeWidget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeWidget")
            .finish()
    }
}

impl WrapperWidget for TreeWidget {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.root
    }
}