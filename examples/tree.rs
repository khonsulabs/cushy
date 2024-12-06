use cushy::{App, Open, Run};
use cushy::widget::MakeWidget;
use cushy::widgets::tree::Tree;
use cushy::window::PendingWindow;

#[cushy::main]
fn main(app: &mut App) -> cushy::Result {

    let pending = PendingWindow::default();
  
    let mut tree: Tree = Tree::default();
    let root_key = tree.insert_child("root".to_string(), None).unwrap();
    let child_key_1 = tree.insert_child("child 1".to_string(), Some(&root_key)).unwrap();
    let nested_child_key_1 = tree.insert_child("nested 1".to_string(), Some(&child_key_1)).unwrap();
    let _nested_child_key_2 = tree.insert_after("nested 2".to_string(), &nested_child_key_1).unwrap();
    let child_key_2 = tree.insert_child("child 2".to_string(), Some(&root_key)).unwrap();
    let nested_child_key_3 = tree.insert_child("nested 3".to_string(), Some(&child_key_2)).unwrap();
    let _nested_child_key_4 = tree.insert_after("nested 4".to_string(), &nested_child_key_3);

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