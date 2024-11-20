use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use cushy::value::{Destination, Dynamic};
use cushy::widget::{MakeWidget, WidgetInstance};
use cushy::widgets::input::InputValue;
use crate::action::Action;

#[derive(Clone, Debug)]
pub enum TextDocumentMessage {
    Create,
    Load,
    Loaded(String)
}

#[derive(Debug)]
pub enum TextDocumentAction {
    None,
    Load,
    Create
}

#[derive(Debug)]
pub enum TextDocumentError {
    ErrorCreatingFile(PathBuf),
    ErrorLoadingFile(PathBuf),
}

pub struct TextDocument {
    pub path: PathBuf,

    content: Dynamic<String>
}

impl TextDocument {
    pub fn from_path(path: PathBuf) -> (Self, TextDocumentMessage) {
        (
            Self {
                path,
                content: Dynamic::default(),
            },
            TextDocumentMessage::Load
        )
    }

    pub fn new(path: PathBuf) -> (Self, TextDocumentMessage) {
        (
            Self {
                path,
                content: Dynamic::default(),
            },
            TextDocumentMessage::Create
        )
    }

    pub async fn create(path: PathBuf) -> Result<(), TextDocumentError> {
        println!("creating text document. path: {:?}", path);
        // TODO improve error handling by using '_error'
        fs::write(&path, "New text file")
            .map_err(|_error|TextDocumentError::ErrorCreatingFile(path))
    }

    pub async fn load(path: PathBuf) -> Result<String, TextDocumentError> {
        println!("loading text document. path: {:?}", path);

        // Simulate slow loading
        async_std::task::sleep(Duration::from_millis(500)).await;

        // TODO improve error handling by using '_error'
        fs::read_to_string(&path)
            .map_err(|_error|TextDocumentError::ErrorLoadingFile(path))
    }

    pub fn create_content(&self) -> WidgetInstance {
        println!("TextDocument::create_content. path: {:?}", self.path);

        self.content.clone().into_input()
            .make_widget()
    }

    pub fn update(&mut self, message: TextDocumentMessage) -> Action<TextDocumentAction> {
        let action = match message {
            TextDocumentMessage::Create => TextDocumentAction::Create,
            TextDocumentMessage::Load => TextDocumentAction::Load,
            TextDocumentMessage::Loaded(content) => {
                self.content.replace(content);
                TextDocumentAction::None
            }
        };

        Action::new(action)
    }

}