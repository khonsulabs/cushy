use std::fs;
use std::path::PathBuf;
use cushy::value::Dynamic;
use cushy::widget::{MakeWidget, WidgetInstance};
use cushy::widgets::input::InputValue;

pub struct TextDocument {
    pub path: PathBuf,

    content: Dynamic<String>
}

impl TextDocument {
    pub fn from_path(path: PathBuf) -> TextDocument {

        let text = fs::read_to_string(&path).unwrap();

        let content = Dynamic::new(text);

        Self {
            path,
            content,
        }
    }

    pub fn create_content(&self) -> WidgetInstance {
        println!("TextDocument::create_content. path: {:?}", self.path);

        self.content.clone().into_input()
            .make_widget()
    }
}