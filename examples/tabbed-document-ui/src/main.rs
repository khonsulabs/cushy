use std::path;
use std::path::PathBuf;
use futures::{select, StreamExt};
use slotmap::SlotMap;
use thiserror::Error;
use cushy::figures::units::{Lp, Px};
use cushy::App;
use cushy::dialog::{FilePicker, FileType};
use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance};
use cushy::widgets::{Expand, Stack};
use cushy::window::{PendingWindow, WindowHandle};
use cushy::Open;
use cushy::styles::components::IntrinsicPadding;
use cushy::styles::Dimension;
use crate::app_tabs::document::DocumentTab;
use crate::app_tabs::home::HomeTab;
use crate::app_tabs::new::{NewTab, NewTabMessage};
use crate::app_tabs::{TabKind, TabKindMessage};
use crate::config::Config;
use crate::context::Context;
use crate::documents::{DocumentKey, DocumentKind};
use crate::documents::image::ImageDocument;
use crate::documents::text::TextDocument;
use crate::runtime::{Executor, RunTime};
use crate::task::{Task};
use crate::widgets::tab_bar::{TabBar, TabKey, TabMessage};

mod config;
mod widgets;
mod global_context;
mod context;
mod app_tabs;
mod documents;
mod task;
mod runtime;

#[derive(Clone, PartialEq)]
enum AppMessage {
    None,
    TabMessage(TabMessage<TabKindMessage>),
    ToolBarMessage(ToolbarMessage),
}

impl Default for AppMessage {
    fn default() -> Self {
        AppMessage::None
    }
}

struct AppState {
    tab_bar: Dynamic<TabBar<TabKind, TabKindMessage>>,
    config: Dynamic<Config>,
    context: Dynamic<Context>,

    documents: Dynamic<SlotMap<DocumentKey, DocumentKind>>,
    message: Dynamic<AppMessage>,
}

#[cushy::main]
fn main(app: &mut App) -> cushy::Result {

    let message: Dynamic<AppMessage> = Dynamic::default();

    let (sender, mut receiver) = futures::channel::mpsc::unbounded();

    let executor = Executor::new().expect("should be able to create an executor");
    executor.spawn({
        let message = message.clone();
        async move {
            loop {
                select! {
                    received_message = receiver.select_next_some() => {
                        message.set(received_message);
                    }
                }
            }
        }
    });
    let mut runtime = RunTime::new(executor, sender);

    let pending = PendingWindow::default();
    let window = pending.handle();

    let config = Dynamic::new(config::load());
    let documents = Dynamic::new(SlotMap::default());

    let tab_message = Dynamic::default();
    tab_message.for_each_cloned({
        let message = message.clone();
        move |tab_message|{
            message.set(AppMessage::TabMessage(tab_message));
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

    let app_state = AppState {
        tab_bar: tab_bar.clone(),
        context: context.clone(),
        config,
        documents,
        message: message.clone(),
    };

    let toolbar_message: Dynamic<ToolbarMessage> = Dynamic::default();
    toolbar_message.for_each_cloned({
        let message = message.clone();
        move |toolbar_message|{
            message.set(AppMessage::ToolBarMessage(toolbar_message));
        }
    })
        .persist();

    let toolbar = make_toolbar(toolbar_message);

    let ui_elements = [
        toolbar.make_widget(),
        app_state.tab_bar.lock().make_widget(),
    ];

    let dyn_app_state = Dynamic::new(app_state);

    message
        .for_each_cloned({
            let dyn_app_state = dyn_app_state.clone();
            move |message|{
                let task = dyn_app_state.lock().update(message);

                if let Some(stream) = task::into_stream(task) {
                    runtime.run(stream);
                }
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
    fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::None => Task::none(),
            AppMessage::TabMessage(message) => {
                self.tab_bar.lock()
                    .update(&self.context, message)
                    .map(|message|AppMessage::TabMessage(message))
            }
            AppMessage::ToolBarMessage(message) => {
                self
                    .on_toolbar_message(message)
                    .map(|message|AppMessage::ToolBarMessage(message))
            }
        }
    }

    fn on_toolbar_message(&self, message: ToolbarMessage) -> Task<ToolbarMessage> {
        match message {
            ToolbarMessage::None => {}
            ToolbarMessage::HomeClicked => {
                println!("home clicked");

                add_home_tab(&self.context, &self.tab_bar);
            }
            ToolbarMessage::NewClicked => {
                println!("New clicked");

                self.add_new_tab();
            }
            ToolbarMessage::OpenClicked => {

                let window = self.context.lock().with_context::<WindowHandle, _, _>(|window_handle| {
                    window_handle.clone()
                }).unwrap();

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
                        let tab_bar = self.tab_bar.clone();
                        let documents = self.documents.clone();
                        let context = self.context.clone();

                        move |path|{
                            if let Some(path) = path {
                                println!("path: {:?}", path);

                                open_document(&context, &documents, &tab_bar, path).ok();
                            }
                        }
                    });

            }
            ToolbarMessage::CloseAllClicked => {
                println!("close all clicked");

                self.tab_bar.lock().close_all();
            }
        }

        Task::none()
    }

    fn add_new_tab(&self) {

        let new_tab_message: Dynamic<NewTabMessage> = Dynamic::default();

        let tab_key = self.tab_bar.lock()
            .add_tab(&self.context, TabKind::New(NewTab::new(new_tab_message.clone())), ||{});

        new_tab_message.for_each_cloned({
            let message = self.message.clone();
            move |new_tab_message|{
                message.set(
                    AppMessage::TabMessage(
                        TabMessage::TabKindMessage(
                            tab_key,
                            TabKindMessage::NewTabMessage(new_tab_message)
                        )
                    )
                );
            }
        })
            .persist();

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
    tab_bar: &Dynamic<TabBar<TabKind, TabKindMessage>>,
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

fn make_document_tab(context: &Dynamic<Context>, documents: &Dynamic<SlotMap<DocumentKey, DocumentKind>>, tab_bar: &Dynamic<TabBar<TabKind, TabKindMessage>>, document: DocumentKind) -> TabKey {
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

#[derive(Clone, PartialEq)]
pub enum ToolbarMessage {
    None,
    OpenClicked,
    HomeClicked,
    NewClicked,
    CloseAllClicked,
}

impl Default for ToolbarMessage {
    fn default() -> Self {
        Self::None
    }
}


fn make_toolbar(toolbar_message: Dynamic<ToolbarMessage>) -> Stack {
    let button_padding = Dimension::Lp(Lp::points(4));

    let home_button = "Home"
        .into_button()
        .on_click({
            let message = toolbar_message.clone();
            move |_event| message.set(ToolbarMessage::HomeClicked)
        })
        .with(&IntrinsicPadding, button_padding);

    let new_button = "New"
        .into_button()
        .on_click({
            let message = toolbar_message.clone();
            move |_event| message.set(ToolbarMessage::NewClicked)
        })
        .with(&IntrinsicPadding, button_padding);

    let open_button = "Open"
        .into_button()
        .on_click({
            let message = toolbar_message.clone();
            move |_event| message.set(ToolbarMessage::OpenClicked)
        })
        .with(&IntrinsicPadding, button_padding);


    let close_all_button = "Close all"
        .into_button()
        .on_click({
            let message = toolbar_message.clone();
            move |_event| message.set(ToolbarMessage::CloseAllClicked)
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

fn add_home_tab(context: &Dynamic<Context>, tab_bar: &Dynamic<TabBar<TabKind, TabKindMessage>>) {
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
