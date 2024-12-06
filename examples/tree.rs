use cushy::{App, Open, Run};
use cushy::widget::MakeWidget;
use cushy::widgets::label::Displayable;
use cushy::widgets::tree::Tree;
use cushy::window::PendingWindow;

fn make_node_with_label_and_buttons(name: &'static str) -> impl MakeWidget {
    name.into_label()
        .and("Delete"
            .into_button()
            .on_click(|event|{
                // FIXME here we want to do this, but we have neither the tree nor the key
                //       tree.remove_node(key);
            })
            .make_widget()
        )
        .and("Add child"
            .into_button()
            .on_click(|event|{
                // FIXME here we want to do this, but we have neither the tree nor the key
                //       tree.insert_child(make_node_with_label_and_buttons("generated child"), Some(key));
            })
            .make_widget()
        )
        .and("Add sibling"
            .into_button()
            .on_click(|event|{
                // FIXME here we want to do this, but we have neither the tree nor the key
                //       tree.insert_after(make_node_with_label_and_buttons("generated child"), key);
            })
            .make_widget()
        )
        .into_columns()
        .make_widget()
}

#[cushy::main]
fn main(app: &mut App) -> cushy::Result {

    let pending = PendingWindow::default();
  
    let mut tree: Tree = Tree::default();
    let root_key = tree.insert_child(make_node_with_label_and_buttons("root"), None).unwrap();
    let child_key_1 = tree.insert_child(make_node_with_label_and_buttons("child 1"), Some(&root_key)).unwrap();
    let nested_child_key_1 = tree.insert_child(make_node_with_label_and_buttons("nested 1"), Some(&child_key_1)).unwrap();
    let _nested_child_key_2 = tree.insert_after(make_node_with_label_and_buttons("nested 2"), &nested_child_key_1).unwrap();
    let child_key_2 = tree.insert_child(make_node_with_label_and_buttons("child 2"), Some(&root_key)).unwrap();
    let nested_child_key_3 = tree.insert_child(make_node_with_label_and_buttons("nested 3"), Some(&child_key_2)).unwrap();
    let _nested_child_key_4 = tree.insert_after(make_node_with_label_and_buttons("nested 4"), &nested_child_key_3);

    let elements = "content above".contain()
        .and(tree.contain())
        .and("content below".contain())
        .into_rows()
        .contain()
        .make_widget();

    let ui = pending.with_root(elements)
        .titled("tree");
    
    ui.open(app)?;
    
    Ok(())
}