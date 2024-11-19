use slotmap::SlotMap;
use cushy::value::Dynamic;
use cushy::widget::WidgetInstance;
use crate::context::Context;
use crate::documents::{DocumentKey, DocumentKind};
use crate::widgets::tab_bar::{Tab, TabKey};

#[derive(Clone, PartialEq)]
pub enum DocumentTabMessage {
    None,
}

impl Default for DocumentTabMessage {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone)]
pub struct DocumentTab {
    document_key: DocumentKey,
}

impl DocumentTab {
    pub fn new(document_key: DocumentKey) -> Self {
        Self {
            document_key,
        }
    }
}

impl Tab<DocumentTabMessage> for DocumentTab {

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

    fn update(&mut self, context: &Dynamic<Context>, tab_key: TabKey, message: DocumentTabMessage) -> () {
        todo!()
    }
}