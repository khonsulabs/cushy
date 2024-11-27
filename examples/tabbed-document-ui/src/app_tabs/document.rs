use slotmap::SlotMap;
use cushy::value::Dynamic;
use cushy::widget::WidgetInstance;
use crate::action::Action;
use crate::context::Context;
use crate::documents::{DocumentKey, DocumentKind};
use crate::documents::image::{ImageDocument, ImageDocumentAction, ImageDocumentMessage};
use crate::documents::text::{TextDocument, TextDocumentAction, TextDocumentMessage};
use crate::task::Task;
use crate::widgets::tab_bar::{Tab, TabKey};

#[derive(Clone, Debug)]
pub enum DocumentTabMessage {
    None,
    ImageDocumentMessage(ImageDocumentMessage),
    TextDocumentMessage(TextDocumentMessage),
}

impl Default for DocumentTabMessage {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug)]
pub enum DocumentTabAction {
    None,
    ImageDocumentTask(Task<ImageDocumentMessage>),
    TextDocumentTask(Task<TextDocumentMessage>),
}

#[derive(Clone)]
pub struct DocumentTab {
    pub document_key: DocumentKey,
    message: Dynamic<DocumentTabMessage>,
}

impl DocumentTab {
    pub fn new(document_key: DocumentKey, message: Dynamic<DocumentTabMessage>) -> Self {
        Self {
            document_key,
            message,
        }
    }
}

impl Tab<DocumentTabMessage, DocumentTabAction> for DocumentTab {

    fn label(&self, context: &Dynamic<Context>) -> String {
        context.lock().with_context::<Dynamic<SlotMap<DocumentKey, DocumentKind>>, _, _>(|documents| {
            let documents_guard = documents.lock();
            let document = documents_guard.get(self.document_key).unwrap();

            let path = match document {
                DocumentKind::TextDocument(document) => &document.path,
                DocumentKind::ImageDocument(document) => &document.path,
            };

            path.file_name().unwrap().to_str().unwrap().to_string()

        }).unwrap()
    }

    fn make_content(&self, context: &Dynamic<Context>, _tab_key: TabKey) -> WidgetInstance {

        context.lock().with_context::<Dynamic<SlotMap<DocumentKey, DocumentKind>>, _, _>(|documents| {
            let documents_guard = documents.lock();
            let document = documents_guard.get(self.document_key).unwrap();

            match document {
                DocumentKind::TextDocument(text_document) => text_document.create_content(),
                DocumentKind::ImageDocument(image_document) => image_document.create_content()
            }
        }).unwrap()
    }

    fn update(&mut self, context: &Dynamic<Context>, _tab_key: TabKey, message: DocumentTabMessage) -> Action<DocumentTabAction> {

        let action = context.lock().with_context::<Dynamic<SlotMap<DocumentKey, DocumentKind>>, _, _>(|documents| {
            let mut documents_guard = documents.lock();
            let document = documents_guard.get_mut(self.document_key).unwrap();

            match (document, message) {
                (DocumentKind::ImageDocument(document), DocumentTabMessage::ImageDocumentMessage(message)) => {
                    let action = document.update(message);

                    match action.into_inner() {
                        ImageDocumentAction::None => DocumentTabAction::None,
                        ImageDocumentAction::Create => {
                            let task = Task::perform(ImageDocument::create(document.path.clone()),
                                // TODO handle errors
                                move |_result|ImageDocumentMessage::Load
                            );
                            DocumentTabAction::ImageDocumentTask(task)
                        }
                        ImageDocumentAction::Load => {
                            let task = Task::perform(ImageDocument::load(document.path.clone()),
                                // TODO handle errors
                                move |result|ImageDocumentMessage::Loaded(result.unwrap())
                            );
                            DocumentTabAction::ImageDocumentTask(task)
                        }
                    }
                }
                (DocumentKind::TextDocument(document), DocumentTabMessage::TextDocumentMessage(message)) => {
                    let action = document.update(message);

                    match action.into_inner() {
                        TextDocumentAction::None => DocumentTabAction::None,
                        TextDocumentAction::Create => {
                            let task = Task::perform(TextDocument::create(document.path.clone()), {
                                // TODO handle errors
                                move |_result| TextDocumentMessage::Load
                            });
                            DocumentTabAction::TextDocumentTask(task)
                        }
                        TextDocumentAction::Load => {
                            let task = Task::perform(TextDocument::load(document.path.clone()), {
                                // TODO handle errors
                                move |result| TextDocumentMessage::Loaded(result.unwrap())
                            });
                            DocumentTabAction::TextDocumentTask(task)
                        }
                    }
                }
                _ => unreachable!(),
            }
        }).unwrap();

        Action::new(action)
    }
}