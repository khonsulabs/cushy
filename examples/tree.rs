use cushy::{App, Open, Run};
use cushy::widget::MakeWidget;
use cushy::widgets::tree::Tree;
use cushy::window::PendingWindow;

#[cushy::main]
fn main(app: &mut App) -> cushy::Result {

    let pending = PendingWindow::default();
  
    let mut tree: Tree = Tree::default();
    let root_key = tree.insert_child("root".to_string(), None).unwrap();
    let child_key = tree.insert_child("child".to_string(), Some(&root_key)).unwrap();
    let _nested_child_key = tree.insert_child("nested".to_string(), Some(&child_key));

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