use std::time::Duration;

use bonsaidb::client::Client;
use bonsaidb_counter_shared::{ExampleApi, Request, Response};
use gooey::{
    core::{Context, StyledWidget, WidgetId},
    widgets::{
        button::Button,
        component::{Behavior, Component, ComponentBuilder, ComponentTransmogrifier},
        container::Container,
    },
    App,
};

/// The example's main entrypoint.
fn main() {
    // The user interface and database will be run separately, and flume
    // channels will send `DatabaseCommand`s to do operations on the database
    // server.
    let (command_sender, command_receiver) = flume::unbounded();

    // Spawn an async task that processes commands sent by `command_sender`.
    App::spawn(process_database_commands(command_receiver));

    App::from_root(|storage|
        // The root widget is a `Component` with our component behavior
        // `Counter`.
        Component::new(Counter::new(command_sender), storage))
    // Register our custom component's transmogrifier.
    .with(ComponentTransmogrifier::<Counter>::default())
    // Run the app using the widget returned by the initializer.
    .run()
}

/// The state of the `Counter` component.
#[derive(Debug)]
struct Counter {
    command_sender: flume::Sender<DatabaseCommand>,
    count: Option<u32>,
}

impl Counter {
    /// Returns a new instance that sends database commands to `command_sender`.
    pub const fn new(command_sender: flume::Sender<DatabaseCommand>) -> Self {
        Self {
            command_sender,
            count: None,
        }
    }
}

/// Component defines a trait `Behavior` that allows you to write cross-platform
/// code that interacts with one or more other widgets.
impl Behavior for Counter {
    /// The root widget of the `Component` will be a `Container`.
    type Content = Container;
    /// The event enum that child widget events will send.
    type Event = CounterEvent;
    /// An enum of child widgets.
    type Widgets = CounterWidgets;

    fn create_content(&mut self, builder: &mut ComponentBuilder<Self>) -> StyledWidget<Container> {
        StyledWidget::from(builder.register(
            CounterWidgets::Button,
            Button::new(
                "Click Me!",
                builder.map_event(|_| CounterEvent::ButtonClicked),
            ),
        ))
    }

    fn initialize(component: &mut Component<Self>, context: &Context<Component<Self>>) {
        let _ = component
            .behavior
            .command_sender
            .send(DatabaseCommand::Initialize(DatabaseContext {
                context: context.clone(),
                button_id: component
                    .registered_widget(&CounterWidgets::Button)
                    .unwrap()
                    .id()
                    .clone(),
            }));
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        _context: &Context<Component<Self>>,
    ) {
        let CounterEvent::ButtonClicked = event;

        let _ = component
            .behavior
            .command_sender
            .send(DatabaseCommand::Increment);
    }
}

/// This enum identifies widgets that you want to send commands to. If a widget
/// doesn't need to receive commands, it doesn't need an entry in this enum.
#[derive(Debug, Hash, Eq, PartialEq)]
enum CounterWidgets {
    /// The button that users click.
    Button,
}

/// All events that the `Counter` behavior will receive from child widgets.
#[derive(Debug)]
enum CounterEvent {
    /// The button was clicked.
    ButtonClicked,
}

/// Commands that the user interface will send to the database task.
enum DatabaseCommand {
    /// Initializes the worker with a context, which
    Initialize(DatabaseContext),
    /// Increment the counter.
    Increment,
}

/// A context provides the information necessary to communicate with the user
/// inteface.
#[derive(Clone)]
struct DatabaseContext {
    /// The button's id.
    button_id: WidgetId,
    /// The context of the component.
    context: Context<Component<Counter>>,
}

/// Processes each command from `receiver` as it becomes available.
async fn process_database_commands(receiver: flume::Receiver<DatabaseCommand>) {
    let context = match receiver.recv_async().await.unwrap() {
        DatabaseCommand::Initialize(context) => context,
        _ => unreachable!(),
    };

    // Connect to the locally running server. `cargo run --package server`
    // launches the server.
    let loop_context = context.clone();
    let client = loop {
        let client_context = loop_context.clone();
        match Client::build("ws://127.0.0.1:8081".parse().unwrap())
            .with_custom_api_callback::<ExampleApi, _>(move |response| {
                let Response::CounterValue(count) = response;
                client_context.context.with_widget_mut(
                    &client_context.button_id,
                    |button: &mut Button, context| {
                        button.set_label(count.to_string(), context);
                    },
                );
            })
            .finish()
            .await
        {
            Ok(client) => break client,
            Err(err) => {
                log::error!("Error connecting: {:?}", err);
                App::sleep_for(Duration::from_secs(1)).await;
            }
        }
    };

    match client.send_api_request(Request::GetCounter).await {
        Ok(Response::CounterValue(count)) => {
            context
                .context
                .with_widget_mut(&context.button_id, |button: &mut Button, context| {
                    button.set_label(count.to_string(), context);
                });
        }
        Err(err) => {
            log::error!("Error retrieving current counter value: {:?}", err);
        }
    }
    // For each `DatabaseCommand`. The only error possible from recv_async() is
    // a disconnected error, which should only happen when the app is shutting
    // down.
    while let Ok(command) = receiver.recv_async().await {
        match command {
            DatabaseCommand::Increment => {
                increment_counter(&client, &context).await;
            }
            DatabaseCommand::Initialize(_) => unreachable!(),
        }
    }
}

async fn increment_counter(client: &Client<ExampleApi>, context: &DatabaseContext) {
    // While we could use the key value store directly, this example is showing
    // another powerful feature of BonsaiDb: the ablity to easily add a custom
    // api using your own enums.
    match client.send_api_request(Request::IncrementCounter).await {
        Ok(Response::CounterValue(count)) => {
            context
                .context
                .with_widget_mut(&context.button_id, |button: &mut Button, context| {
                    button.set_label(count.to_string(), context);
                });
        }
        Err(err) => {
            log::error!("Error sending request: {:?}", err);
            eprintln!("Error sending request: {:?}", err);
        }
    }
}
