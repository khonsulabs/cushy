use std::path;
use std::path::PathBuf;
use slotmap::SlotMap;
use thiserror::Error;
use cushy::figures::units::{Lp, Px};
use cushy::App;
use cushy::dialog::{FilePicker, FileType};
use cushy::reactive::value::{Destination, Dynamic, Source};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance};
use cushy::widgets::{Expand, Stack};
use cushy::window::{PendingWindow, WindowHandle};
use cushy::Open;
use cushy::styles::components::IntrinsicPadding;
use cushy::styles::Dimension;
use crate::app_tabs::document::DocumentTab;
use crate::app_tabs::home::HomeTab;
use crate::app_tabs::new::NewTab;
use crate::app_tabs::TabKind;
use crate::config::Config;
use crate::context::Context;
use crate::documents::{DocumentKey, DocumentKind};
use crate::documents::image::ImageDocument;
use crate::documents::text::TextDocument;
use crate::widgets::tab_bar::{TabBar, TabKey, TabMessage};

mod config;
mod widgets;
mod global_context;
mod context;
mod app_tabs;
mod documents;

#[derive(Clone, PartialEq)]
enum Message {
    None,
    TabMessage(TabMessage)
}

impl Default for Message {
    fn default() -> Self {
        Message::None
    }
}

struct AppState {
    tab_bar: Dynamic<TabBar<TabKind>>,
    config: Dynamic<Config>,
    context: Dynamic<Context>,

    documents: Dynamic<SlotMap<DocumentKey, DocumentKind>>,
}

#[cushy::main]
fn main(app: &mut App) -> cushy::Result {

    let pending = PendingWindow::default();
    let window = pending.handle();

    let message: Dynamic<Message> = Dynamic::default();

    let config = Dynamic::new(config::load());
    let documents = Dynamic::new(SlotMap::default());

    let tab_message = Dynamic::default();
    tab_message.for_each_cloned({
        let message = message.clone();
        move |tab_message|{
            message.set(Message::TabMessage(tab_message));
        }
    })
        .persist();

    let tab_bar = Dynamic::new(TabBar::new(&tab_message));

    let mut context = Context::default();
    context.provide(config.clone());
    context.provide(documents.clone());
    context.provide(tab_bar.clone());
    context.provide(window);

    let context = Dynamic::new(context);

    let mut app_state = AppState {
        tab_bar: tab_bar.clone(),
        context: context.clone(),
        config,
        documents,
    };


    let toolbar = make_toolbar(&mut app_state);

    let ui_elements = [
        toolbar.make_widget(),
        app_state.tab_bar.lock().make_widget(),
    ];

    let dyn_app_state = Dynamic::new(app_state);

    message
        .for_each_cloned({
            let dyn_app_state = dyn_app_state.clone();
            move |message|{
                dyn_app_state.lock().update(message);
            }
        })
        .persist();


    let ui = pending.with_root(
        ui_elements
            .into_rows()
            .width(Px::new(800)..)
            .height(Px::new(600)..)
            .fit_vertically()
            .fit_horizontally()
            .make_widget()
    )
        .on_close({
            let dyn_app_state = dyn_app_state.clone();
            let config = dyn_app_state.lock().config.clone();
            let documents = dyn_app_state.lock().documents.clone();
            move ||{
                // TODO update the list of open documents

                update_open_documents(&config, &documents);

                let config = config.lock();
                println!("Saving config");
                config::save(&*config);
            }
        })
        .titled("Tabbed document UI");


    {
        let app_state_guard = dyn_app_state.lock();
        let app_state = &*app_state_guard;


        if app_state.config.lock().show_home_on_startup
        {
            add_home_tab(&context, &app_state.tab_bar);
        }

        for path in app_state.config.lock().open_document_paths.clone() {
            open_document(&context, &app_state.documents, &app_state.tab_bar, path).ok();
        }
    }

    ui.open(app)?;

    // FIXME control never returns here (at least on windows)

    Ok(())
}

impl AppState {
    fn update(&mut self, message: Message) {
        match message {
            Message::None => {}
            Message::TabMessage(message) => {
                self.tab_bar.lock().update(message);
            }
        }
    }
}

fn update_open_documents(config: &Dynamic<Config>, documents: &Dynamic<SlotMap<DocumentKey, DocumentKind>>) {
    let open_documents: Vec<PathBuf> = documents.lock().iter()
        .map(|(_key, document)| {
            match document {
                DocumentKind::TextDocument(document) => document.path.clone(),
                DocumentKind::ImageDocument(document) => document.path.clone(),
            }
        })
        .collect();

    println!("open_documents: {:?}", open_documents);
    config.lock().open_document_paths = open_documents;
}

#[derive(Error, Debug)]
enum OpenDocumentError {
    #[error("Unsupported file type. extension: {extension}")]
    UnsupportedFileExtension{extension: String},
    #[error("IO error, cause: {cause}")]
    IoError{cause: std::io::Error},
}

const SUPPORTED_IMAGE_EXTENSIONS: [&'static str; 5] = ["bmp", "png", "jpg", "jpeg", "svg"];
const SUPPORTED_TEXT_EXTENSIONS: [&'static str; 1] = ["txt"];

fn open_document(
    context: &Dynamic<Context>,
    documents: &Dynamic<SlotMap<DocumentKey, DocumentKind>>,
    tab_bar: &Dynamic<TabBar<TabKind>>,
    path: PathBuf
) -> Result<(), OpenDocumentError> {
    println!("open_document. path: {:?}", path);

    let path = path::absolute(path)
        .or_else(|cause| Err(OpenDocumentError::IoError { cause }))?;

    let extension = path.extension().unwrap().to_str().unwrap();

    let tab_key = if SUPPORTED_TEXT_EXTENSIONS.contains(&extension) {
        let document = DocumentKind::TextDocument(TextDocument::from_path(path));

        let tab_key = make_document_tab(context, documents, tab_bar, document);

        tab_key
    } else if SUPPORTED_IMAGE_EXTENSIONS.contains(&extension) {
        let document = DocumentKind::ImageDocument(ImageDocument::from_path(path));

        let tab_key = make_document_tab(context, documents, tab_bar, document);

        tab_key
    } else {
        return Err(OpenDocumentError::UnsupportedFileExtension { extension: extension.to_string() });
    };

    println!("added document tab with key. key: {:?}", tab_key);

    Ok(())
}

fn make_document_tab(context: &Dynamic<Context>, documents: &Dynamic<SlotMap<DocumentKey, DocumentKind>>, tab_bar: &Dynamic<TabBar<TabKind>>, document: DocumentKind) -> TabKey {
    let document_key = documents.lock().insert(document);

    let document_tab = DocumentTab::new(document_key);

    let mut tab_bar_guard = tab_bar.lock();
    let tab_key = tab_bar_guard.add_tab(context, TabKind::Document(document_tab), {
        let documents = documents.clone();
        let document_key = document_key.clone();
        move || {
            documents.lock().remove(document_key);
        }
    });
    tab_key
}

fn make_toolbar(app_state: &mut AppState) -> Stack {
    let button_padding = Dimension::Lp(Lp::points(4));

    let window = app_state.context.lock().with_context::<WindowHandle, _, _>(|window_handle| {
        window_handle.clone()
    }).unwrap();

    let home_button = "Home"
        .into_button()
        .on_click({
            let tab_bar = app_state.tab_bar.clone();
            let context = app_state.context.clone();
            move |_|{
                println!("home clicked");

                add_home_tab(&context, &tab_bar);
            }
        })
        .with(&IntrinsicPadding, button_padding);

    let new_button = "New"
        .into_button()
        .on_click({
            let tab_bar = app_state.tab_bar.clone();
            let context = app_state.context.clone();
            move |_|{
                println!("New clicked");

                add_new_tab(&context, &tab_bar)
            }
        })
        .with(&IntrinsicPadding, button_padding);

    let open_button = "Open"
        .into_button()
        .on_click({
            let tab_bar = app_state.tab_bar.clone();
            let documents = app_state.documents.clone();
            let context = app_state.context.clone();
            move |_|{
                println!("open clicked");

                let all_extensions: Vec<_> = SUPPORTED_TEXT_EXTENSIONS.iter().cloned().chain(SUPPORTED_IMAGE_EXTENSIONS.iter().cloned()).collect();

                FilePicker::new()
                    .with_title("Open file")
                    .with_types([
                        FileType::from(("All supported files", into_array::<_, 6>(all_extensions))),
                        FileType::from(("Text files", SUPPORTED_TEXT_EXTENSIONS)),
                        FileType::from(("Image files", SUPPORTED_IMAGE_EXTENSIONS)),
                    ])
                    .pick_file(&window,{

                        // NOTE: Nested callbacks require a second clone
                        let tab_bar = tab_bar.clone();
                        let documents = documents.clone();
                        let context = context.clone();

                        move |path|{
                            if let Some(path) = path {
                                println!("path: {:?}", path);

                                open_document(&context, &documents, &tab_bar, path).ok();
                            }
                        }
                    });
            }
        })
        .with(&IntrinsicPadding, button_padding);


    let close_all_button = "Close all"
        .into_button()
        .on_click({
            let tab_bar = app_state.tab_bar.clone();
            move |_| {
                println!("close all clicked");

                tab_bar.lock().close_all();
            }
        })
        .with(&IntrinsicPadding, button_padding);


    let toolbar_widgets: [WidgetInstance; 5] = [
        home_button.make_widget(),
        new_button.make_widget(),
        open_button.make_widget(),
        close_all_button.make_widget(),
        Expand::empty().make_widget(),
    ];

    let toolbar = toolbar_widgets.into_columns();
    toolbar
}

fn add_new_tab(context: &Dynamic<Context>, tab_bar: &Dynamic<TabBar<TabKind>>) {
    let mut tab_bar_guard = tab_bar
        .lock();

    tab_bar_guard
        .add_tab(context, TabKind::New(NewTab::default()), ||{});
}

fn add_home_tab(context: &Dynamic<Context>, tab_bar: &Dynamic<TabBar<TabKind>>) {
    let mut tab_bar_guard = tab_bar
        .lock();

    let home_tab_result = tab_bar_guard.with_tabs(|mut iter|{
        iter.find_map(move |(_key, tab)|
            match tab {
                TabKind::Home(tab) => Some((_key, tab.clone())),
                _ => None
            }
        )
    });

    if let Some((key, _tab)) = home_tab_result {
        tab_bar_guard.activate(key);
    } else {
        tab_bar_guard
            .add_tab(context, TabKind::Home(HomeTab::default()), ||{});
    }
}

fn into_array<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Incorrect element count. required: {}, actual: {}", N, v.len()))
}
