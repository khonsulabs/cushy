use std::path;
use std::path::PathBuf;
use slotmap::SlotMap;
use thiserror::Error;
use cushy::figures::units::{Lp, Px};
use cushy::App;
use cushy::dialog::{FilePicker, FileType};
use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance};
use cushy::widgets::{Expand, Stack};
use cushy::window::{PendingWindow, WindowHandle};
use cushy::styles::components::IntrinsicPadding;
use cushy::styles::Dimension;
use crate::action::Action;
use crate::app_tabs::document::{DocumentTab, DocumentTabAction, DocumentTabMessage};
use crate::app_tabs::home::{HomeTab, HomeTabAction};
use crate::app_tabs::new::{KindChoice, NewTab, NewTabAction, NewTabMessage};
use crate::app_tabs::{TabKind, TabKindAction, TabKindMessage};
use crate::config::Config;
use crate::context::Context;
use crate::documents::{DocumentKey, DocumentKind};
use crate::documents::image::ImageDocument;
use crate::documents::text::TextDocument;
use crate::runtime::{Executor, MessageDispatcher, RunTime};
use crate::task::{Task};
use crate::widgets::tab_bar::{TabAction, TabBar, TabKey, TabMessage};

mod config;
mod widgets;
mod global_context;
mod context;
mod app_tabs;
mod documents;
mod task;
mod action;
mod runtime;

#[derive(Clone, Debug)]
enum AppMessage {
    None,
    TabMessage(TabMessage<TabKindMessage>),
    ToolBarMessage(ToolbarMessage),
    FileOpened(PathBuf),
}

impl Default for AppMessage {
    fn default() -> Self {
        AppMessage::None
    }
}

struct AppState {
    tab_bar: Dynamic<TabBar<TabKind, TabKindMessage, TabKindAction>>,
    config: Dynamic<Config>,
    context: Dynamic<Context>,

    documents: Dynamic<SlotMap<DocumentKey, DocumentKind>>,
    message: Dynamic<AppMessage>,
}

#[cushy::main]
fn main(app: &mut App) -> cushy::Result {

    let message: Dynamic<AppMessage> = Dynamic::default();

    let (mut sender, receiver) = futures::channel::mpsc::unbounded();

    let executor = Executor::new().expect("should be able to create an executor");
    executor.spawn(MessageDispatcher::dispatch(receiver, message.clone()));
    let mut runtime = RunTime::new(executor, sender.clone());

    let pending = PendingWindow::default();
    let window = pending.handle();

    let config = Dynamic::new(config::load());
    let documents = Dynamic::new(SlotMap::default());

    let tab_message = Dynamic::default();
    tab_message.for_each_cloned({
        let message = message.clone();
        move |tab_message|{
            message.force_set(AppMessage::TabMessage(tab_message));
        }
    })
        .persist();

    let tab_bar = Dynamic::new(TabBar::new(&tab_message));

    let mut context = Context::default();
    context.provide(config.clone());
    context.provide(documents.clone());
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
            message.force_set(AppMessage::ToolBarMessage(toolbar_message));
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
        .on_open({
            let message = message.clone();
            let dyn_app_state = dyn_app_state.clone();

            move |_window| {
                message.force_set(AppMessage::ToolBarMessage(ToolbarMessage::NewClicked));

                let tab_key = dyn_app_state.lock().tab_bar.lock().find_tab_by_label("New").unwrap();
                println!("New tab. key: {:?}", tab_key);

                message.force_set(AppMessage::TabMessage(TabMessage::TabKindMessage(tab_key, TabKindMessage::NewTabMessage(NewTabMessage::OkClicked))));
            }
        })
        .on_close({
            let dyn_app_state = dyn_app_state.clone();
            let config = dyn_app_state.lock().config.clone();
            let documents = dyn_app_state.lock().documents.clone();
            move ||{
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
    }

    {
        let paths = dyn_app_state.lock().config.lock().open_document_paths.clone();

        let messages: Vec<_> = paths.iter().cloned().filter_map(|path|{
            match dyn_app_state.lock().open_document(path) {
                Ok(message) => Some(message),
                Err(_error) => {
                    // Silently ignore previously opened documents that cannot be loaded
                    None
                }
            }
        }).collect();

        for message in messages {
            // this causes deadlock
            // dyn_app_state.lock().message.force_set(message);

            // so it's required to use the sender instead
            let _result = sender.start_send(message);
        }
    }

    ui.open_centered(app)?;

    // FIXME control never returns here (at least on windows)

    Ok(())
}

impl AppState {
    fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        //println!("AppState::update, message: {:?}", message);
        match message {
            AppMessage::None => Task::none(),
            AppMessage::TabMessage(message) => {
                let action = self.tab_bar.lock()
                    .update(&self.context, message);

                self.on_tab_action(action)
            }
            AppMessage::ToolBarMessage(message) => {
                self
                    .on_toolbar_message(message)
                    .map(|message|AppMessage::ToolBarMessage(message))
            }
            AppMessage::FileOpened(path) => {
                match self.open_document(path) {
                    Ok(message) => {
                        Task::done(message)
                    }
                    Err(_error) => {
                        // TODO improve error handling by using '_error'
                        Task::none()
                    }
                }
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

                        let message = self.message.clone();

                        move |path|{
                            if let Some(path) = path {
                                println!("path: {:?}", path);
                                message.force_set(AppMessage::FileOpened(path))
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
            .add_tab(&self.context, TabKind::New(NewTab::new(new_tab_message.clone())));

        new_tab_message.for_each_cloned({
            let message = self.message.clone();
            move |new_tab_message|{
                message.force_set(
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

    fn on_tab_action(&mut self, action: Action<TabAction<TabKindAction, TabKind>>) -> Task<AppMessage> {
        let action = action.into_inner();

        match action {
            TabAction::TabSelected(tab_key) => {
                println!("tab selected, key: {:?}", tab_key);
                Task::none()
            },
            TabAction::TabClosed(tab_key, tab) => {
                println!("tab closed, key: {:?}", tab_key);
                match tab {
                    TabKind::Home(_tab) => (),
                    TabKind::Document(tab) => {
                        self.documents.lock().remove(tab.document_key);
                    }
                    TabKind::New(_tab) => ()
                }

                Task::none()
            },
            TabAction::TabAction(tab_key, tab_action) => {
                println!("tab action. key: {:?}, action: {:?}", tab_key, tab_action);
                match tab_action {
                    TabKindAction::HomeTabAction(_tab_key, action) => {
                        match action {
                            HomeTabAction::None => Task::none(),
                        }
                    },
                    TabKindAction::DocumentTabAction(_tab_key, action) => {
                        match action {
                            DocumentTabAction::None => Task::none(),
                            DocumentTabAction::ImageDocumentTask(task) => {
                                task.map(move |message|{
                                    AppMessage::TabMessage(TabMessage::TabKindMessage(tab_key, TabKindMessage::DocumentTabMessage(DocumentTabMessage::ImageDocumentMessage(message))))
                                })
                            }
                            DocumentTabAction::TextDocumentTask(task) => {
                                task.map(move |message|{
                                    AppMessage::TabMessage(TabMessage::TabKindMessage(tab_key, TabKindMessage::DocumentTabMessage(DocumentTabMessage::TextDocumentMessage(message))))
                                })
                            }
                        }
                    },
                    TabKindAction::NewTabAction(tab_key, action) => {
                        match action {
                            NewTabAction::None => Task::none(),
                            NewTabAction::CreateDocument(name, path, kind) => {
                                self.create_document(tab_key, name, path, kind)
                            }
                            NewTabAction::Task(task) => {
                                task.map(move |message|{
                                    AppMessage::TabMessage(TabMessage::TabKindMessage(tab_key, TabKindMessage::NewTabMessage(message)))
                                })
                            }
                        }
                    }
                }
            }
            TabAction::None => Task::none(),
        }
    }

    fn create_document(&self, tab_key: TabKey, mut name: String, mut path: PathBuf, kind: KindChoice) -> Task<AppMessage> {
        println!("kind: {:?}, name: {:?}, path: {:?}", kind, name, path);

        match kind {
            KindChoice::Text => {
                name.push_str(".txt");
                path.push(&name);

                let (text_document, message) = TextDocument::create_new(path.clone());
                let document = DocumentKind::TextDocument(text_document);

                let document_key = self.documents.lock().insert(document);
                let document_tab = DocumentTab::new(document_key);

                self.tab_bar.lock().replace(tab_key, &self.context, TabKind::Document(document_tab));

                let task_message = AppMessage::TabMessage(TabMessage::TabKindMessage(tab_key, TabKindMessage::DocumentTabMessage(DocumentTabMessage::TextDocumentMessage(message))));
                Task::done(task_message)
            }
            KindChoice::Image => {
                name.push_str(".png");
                path.push(&name);

                let (image_document, message) = ImageDocument::create_new(path.clone());
                let document = DocumentKind::ImageDocument(image_document);

                let document_key = self.documents.lock().insert(document);
                let document_tab = DocumentTab::new(document_key);

                self.tab_bar.lock().replace(tab_key, &self.context, TabKind::Document(document_tab));

                let task_message = AppMessage::TabMessage(
                    TabMessage::TabKindMessage(
                        tab_key,
                        TabKindMessage::DocumentTabMessage(
                            DocumentTabMessage::ImageDocumentMessage(message)
                        )
                    )
                );
                Task::done(task_message)
            }
        }
    }

    fn open_document(
        &self,
        path: PathBuf
    ) -> Result<AppMessage, OpenDocumentError> {
        println!("open_document. path: {:?}", path);

        let path = path::absolute(path)
            .or_else(|cause| Err(OpenDocumentError::IoError { cause }))?;

        let extension = path.extension().unwrap().to_str().unwrap();

        let message = if SUPPORTED_TEXT_EXTENSIONS.contains(&extension) {
            let (text_document, message) = TextDocument::from_path(path);
            let document = DocumentKind::TextDocument(text_document);

            let tab_key = self.make_document_tab(document);
            AppMessage::TabMessage(TabMessage::TabKindMessage(tab_key, TabKindMessage::DocumentTabMessage(DocumentTabMessage::TextDocumentMessage(message))))
        } else if SUPPORTED_IMAGE_EXTENSIONS.contains(&extension) {
            let (image_document, message) = ImageDocument::from_path(path);
            let document = DocumentKind::ImageDocument(image_document);

            let tab_key = self.make_document_tab(document);
            AppMessage::TabMessage(TabMessage::TabKindMessage(tab_key, TabKindMessage::DocumentTabMessage(DocumentTabMessage::ImageDocumentMessage(message))))
        } else {
            return Err(OpenDocumentError::UnsupportedFileExtension { extension: extension.to_string() });
        };

        println!("open_document message: {:?}", message);

        Ok(message)
    }

    fn make_document_tab(&self, document: DocumentKind) -> TabKey {
        let document_key = self.documents.lock().insert(document);

        let document_tab = DocumentTab::new(document_key);

        let mut tab_bar_guard = self.tab_bar.lock();
        let tab_key = tab_bar_guard.add_tab(&self.context, TabKind::Document(document_tab));
        tab_key
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

#[derive(Clone, Debug)]
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
            move |_event| message.force_set(ToolbarMessage::HomeClicked)
        })
        .with(&IntrinsicPadding, button_padding);

    let new_button = "New"
        .into_button()
        .on_click({
            let message = toolbar_message.clone();
            move |_event| message.force_set(ToolbarMessage::NewClicked)
        })
        .with(&IntrinsicPadding, button_padding);

    let open_button = "Open"
        .into_button()
        .on_click({
            let message = toolbar_message.clone();
            move |_event| message.force_set(ToolbarMessage::OpenClicked)
        })
        .with(&IntrinsicPadding, button_padding);


    let close_all_button = "Close all"
        .into_button()
        .on_click({
            let message = toolbar_message.clone();
            move |_event| message.force_set(ToolbarMessage::CloseAllClicked)
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

fn add_home_tab(context: &Dynamic<Context>, tab_bar: &Dynamic<TabBar<TabKind, TabKindMessage, TabKindAction>>) {
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
            .add_tab(context, TabKind::Home(HomeTab::default()));
    }
}

fn into_array<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Incorrect element count. required: {}, actual: {}", N, v.len()))
}
