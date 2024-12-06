use std::fmt::{Debug, Formatter};
use cushy::ConstraintLimit;
use cushy::context::LayoutContext;
use cushy::figures::Size;
use cushy::figures::units::Px;
use cushy::widget::{MakeWidget, WidgetRef, WrappedLayout, WrapperWidget};
use cushy::widgets::Space;
use indexmap::IndexMap;

#[derive(Default,Clone, Debug, Hash, PartialEq, Eq)]
pub struct TreeNodeKey(usize);

pub struct TreeNode {
    is_expanded: bool,
    parent: Option<TreeNodeKey>,
    depth: usize,

    child: WidgetRef,
    child_height: Option<Px>,
}

impl Debug for TreeNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeNode")
            .field("is_expanded", &self.is_expanded)
            .field("value", &"<...>")
            .field("parent", &self.parent)
            .field("depth", &self.depth)
            .finish()
    }
}


#[derive(Debug)]
pub struct Tree {
    nodes: IndexMap<TreeNodeKey, TreeNode>,
    next_key: TreeNodeKey,
    root: WidgetRef,
}

impl Default for Tree {
    fn default() -> Self {
        let root = Space::default().into_ref();

        Self {
            nodes: IndexMap::new(),
            next_key: TreeNodeKey::default(),
            root
        }
    }
}
impl Tree {
    fn generate_next_key(&mut self) -> TreeNodeKey {
        let key = self.next_key.clone();
        self.next_key.0 += 1;
        key
    }

    /// Inserts a child after the given parent
    pub fn insert_child(&mut self, value: impl MakeWidget, parent: Option<&TreeNodeKey>) -> Option<TreeNodeKey> {

        let child = value.into_ref();

        if let Some(parent) = parent {
            let depth = if let Some(parent_node) = self.nodes.get(parent) {
                parent_node.depth + 1
            } else {
                return None;
            };

            let key = self.generate_next_key();
            let child_node = TreeNode {
                is_expanded: false,
                parent: Some(parent.clone()),
                depth,
                child,
                child_height: None,
            };
            self.nodes.insert(key.clone(), child_node);
            Some(key)
        } else {
            let key = self.generate_next_key();
            let root_node = TreeNode {
                is_expanded: false,
                parent: None,
                depth: 0,
                child,
                child_height: None,
            };
            self.nodes.insert(key.clone(), root_node);
            Some(key)
        }
    }

    /// Inserts a sibling after the given node.
    ///
    /// Returns `None` if the given node doesn't exist.
    pub fn insert_after(&mut self, value: impl MakeWidget, node: &TreeNodeKey) -> Option<TreeNodeKey> {
        if let Some(existing_node) = self.nodes.get(node) {
            let child = value.into_ref();

            let sibling_node = TreeNode {
                is_expanded: false,
                parent: existing_node.parent.clone(),
                depth: existing_node.depth,
                child,
                child_height: None,
            };
            let sibling_key = self.generate_next_key();
            
            self.nodes.insert(sibling_key.clone(), sibling_node);
            
            Some(sibling_key)
        } else {
            None
        }
    }

    /// Clears the tree, removing all nodes and resetting the key.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.next_key = TreeNodeKey::default();
    }

    /// Removes the node and all descendants.
    pub fn remove_node(&mut self, node_key: &TreeNodeKey) {
        // First, check if the node exists
        if !self.nodes.contains_key(node_key) {
            return;
        }

        // Create a stack to hold nodes to be removed
        let mut to_remove = vec![node_key.clone()];

        // We perform a DFS traversal to collect all descendant keys
        while let Some(current_key) = to_remove.pop() {
            if let Some(_node) = self.nodes.shift_remove(&current_key) {
                // Add children of the current node to the stack
                self.nodes
                    .keys()
                    .filter(|&key| self.nodes[key].parent.as_ref() == Some(&current_key))
                    .for_each(|key| to_remove.push(key.clone()));
            }
        }
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
        // when
        
        assert_eq!(key.0, 0);
        assert_eq!(tree.nodes.len(), 1);
        // and
        let root = tree.nodes.get(&key).unwrap();
        
        assert_eq!(root.parent, None);
        assert_eq!(root.depth, 0);
    }
    
    #[test]
    pub fn add_child_to_root() {
        // given
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".to_string(), None).unwrap();

        // when
        let child_key = tree.insert_child("child".to_string(), Some(&root_key)).unwrap();

        // then
        assert_eq!(child_key.0, 1);
        assert_eq!(tree.nodes.len(), 2);

        // and
        let child = tree.nodes.get(&child_key).unwrap();
        assert_eq!(child.parent, Some(root_key.clone()));
        assert_eq!(child.depth, 1);
    }


    #[test]
    pub fn add_sibling_to_child() {
        // given
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".to_string(), None).unwrap();
        let first_child_key = tree.insert_child("first_child".to_string(), Some(&root_key)).unwrap();

        // when
        let sibling_key = tree.insert_after("sibling".to_string(), &first_child_key).unwrap();

        // then
        assert_eq!(tree.nodes.len(), 3);

        // and verify the sibling properties
        let sibling = tree.nodes.get(&sibling_key).unwrap();
        assert_eq!(sibling.parent, Some(root_key.clone()));
        assert_eq!(sibling.depth, 1); // Assuming sibling has the same depth as the first child
    }


    #[test]
    pub fn remove_node() {
        // given
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".to_string(), None).unwrap();
        let child_key = tree.insert_child("child".to_string(), Some(&root_key)).unwrap();
        let _descendant_key = tree.insert_child("descendant".to_string(), Some(&child_key)).unwrap();

        // node to be removed
        let node_to_remove = root_key.clone();

        // assume we have a remove_node method
        tree.remove_node(&node_to_remove);

        // then
        tree.nodes.iter().for_each(|(key, node)| {
            println!("key: {:?}: node: {:?}", key, node);
        });
        // and root, child and descendant nodes should be removed
        assert_eq!(tree.nodes.len(), 0);
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
        let root_key = tree.insert_child("root".to_string(), None).unwrap();
        // direct children
        let key_1 = tree.insert_child("1".to_string(), Some(&root_key)).unwrap();
        let key_2 = tree.insert_child("2".to_string(), Some(&root_key)).unwrap();
        // descendants
        let key_3 = tree.insert_child("3".to_string(), Some(&key_1)).unwrap();
        let _key_4 = tree.insert_child("3".to_string(), Some(&key_2)).unwrap();

        // ensure they exist before removal
        assert_eq!(tree.nodes.len(), 5);
        
        // node to be removed
        let node_to_remove = key_1.clone();

        // when
        tree.remove_node(&node_to_remove);

        // then the root node should remain
        assert_eq!(tree.nodes.len(), 3);
        assert!(tree.nodes.get(&root_key).is_some());

        // and child and childred should be removed
        assert!(tree.nodes.get(&key_1).is_none());
        assert!(tree.nodes.get(&key_3).is_none());
    }
}

impl WrapperWidget for TreeNode {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn position_child(&mut self, size: Size<Px>, _available_space: Size<ConstraintLimit>, _context: &mut LayoutContext<'_, '_, '_, '_>) -> WrappedLayout {
        if self.child_height.is_none() {
            self.child_height.replace(size.height);
        }

        let size = match self.is_expanded {
            true => Size::new(size.width, self.child_height.unwrap()),
            false => Size::new(size.width, Px::new(0)),
        };

        size.into()
    }
}

impl WrapperWidget for Tree {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.root
    }
}