
use cushy::{App, Open, Run};
use cushy::reactive::value::Dynamic;
use cushy::widget::{MakeWidget, WidgetInstance};
use cushy::widgets::label::Displayable;
use cushy::widgets::tree::{Tree, TreeNodeKey};
use cushy::window::PendingWindow;


fn make_node_with_label_and_buttons(tree: Dynamic<Tree>, key: TreeNodeKey, name: &'static str) -> WidgetInstance {
    name.into_label()
        .and("Delete"
            .into_button()
            .on_click({
                let tree = tree.clone();
                let key = key.clone();
                move |event|{
                    tree.lock().remove_node(&key);
                }
            })
            .make_widget()
        )
        .and("Add child"
            .into_button()
            .on_click({
                let tree = tree.clone();
                let key = key.clone();
                move |event|{
                    tree.lock().insert_child_with_key(|child_key|{
                        make_node_with_label_and_buttons(tree.clone(), child_key.clone(), "generated child").make_widget()
                    }, Some(&key));
                }
            })
            .make_widget()
        )
        .and("Add sibling"
            .into_button()
            .on_click({
                let tree = tree.clone();
                let key = key.clone();
                move |event|{
                    tree.lock().insert_after_with_key(|sibling_key|{
                        make_node_with_label_and_buttons(tree.clone(), sibling_key.clone(), "generated sibling").make_widget()
                    }, &key);
                }
            })
            .make_widget()
        )
        .into_columns()
        .make_widget()
}


#[cushy::main]
fn main(app: &mut App) -> cushy::Result {
    let pending = PendingWindow::default();

    let mut dyn_tree: Dynamic<Tree> = Dynamic::new(Tree::default());
    let root_key = {
        let mut tree = dyn_tree.lock();

        let root_key = tree.insert_child_with_key(|key| {
            make_node_with_label_and_buttons(dyn_tree.clone(), key, "root").make_widget()
        }, None).unwrap();

        let child_key_1 = tree.insert_child_with_key(|key| {
            make_node_with_label_and_buttons(dyn_tree.clone(), key, "child 1").make_widget()
        }, Some(&root_key)).unwrap();

        let nested_child_key_1 = tree.insert_child_with_key(|key| {
            make_node_with_label_and_buttons(dyn_tree.clone(), key, "nested 1").make_widget()
        }, Some(&child_key_1)).unwrap();

        let _nested_child_key_2 = tree.insert_after_with_key(|key| {
            make_node_with_label_and_buttons(dyn_tree.clone(), key, "nested 2")
        }, &nested_child_key_1).unwrap();

        let child_key_2 = tree.insert_child_with_key(|key| {
            make_node_with_label_and_buttons(dyn_tree.clone(), key, "child 2").make_widget()
        }, Some(&root_key)).unwrap();

        let nested_child_key_3 = tree.insert_child_with_key(|key| {
            make_node_with_label_and_buttons(dyn_tree.clone(), key, "nested 3").make_widget()
        }, Some(&child_key_2)).unwrap();

        let _nested_child_key_4 = tree.insert_after_with_key(|key|{
            make_node_with_label_and_buttons(dyn_tree.clone(), key, "nested 4").make_widget()
        }, &nested_child_key_3);

        root_key
    };

    let tree_widget = dyn_tree.lock().make_widget();

    // the tree can still be accessed after making a widget
    let _keys = dyn_tree.lock().children_keys(root_key);

    let elements = "content above".contain()
        .and(tree_widget.contain())
        .and("content below".contain())
        .into_rows()
        .contain()
        .vertical_scroll()
        .centered()
        .make_widget();

    let ui = pending.with_root(elements)
        .titled("tree");
    
    ui.open(app)?;
    
    Ok(())
}