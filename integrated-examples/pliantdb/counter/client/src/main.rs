use gooey::{
    core::{Context, StyledWidget, WidgetId},
    widgets::{
        button::Button,
        component::{Behavior, Component, ComponentBuilder, ComponentTransmogrifier},
        container::Container,
    },
    App,
};
use pliantdb::{
    client::Client,
    core::pubsub::{PubSub, Subscriber},
};
use pliantdb_counter_shared::{
    ExampleApi, Request, Response, COUNTER_CHANGED_TOPIC, DATABASE_NAME,
};

/// The example's main entrypoint.
fn main() {
    // The user interface and database will be run separately, and flume
    // channels will send `DatabaseCommand`s to do operations on the database
    // server.
    let (command_sender, command_receiver) = flume::unbounded();

    // Spawn an async task that processes commands sent by `command_sender`.
    App::spawn(process_database_commands(command_receiver));

    App::default()
        // Register our custom component's transmogrifier.
        .with(ComponentTransmogrifier::<Counter>::default())
        // Run the app using the widget returned by the initializer.
        .run(|storage|
            // The root widget is a `Component` with our component behavior
            // `Counter`.
            Component::new(Counter::new(command_sender), storage))
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
        Container::from_registration(builder.register_widget(
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
    // Connect to the locally running server. `cargo run --package server`
    // launches the server.
    let client = Client::new("ws://127.0.0.1:8081".parse().unwrap())
        .await
        .unwrap();
    // Will store the `DatabaseContext` once we receive it from the user interface.
    let mut context = None;
    // For each `DatabaseCommand`. The only error possible from recv_async() is
    // a disconnected error, which should only happen when the app is shutting
    // down.
    while let Ok(command) = receiver.recv_async().await {
        match command {
            DatabaseCommand::Initialize(new_context) => {
                // Launch a task that listens for events when other clients click their buttons.
                App::spawn(watch_for_changes(client.clone(), new_context.clone()));
                // Store the context for use in future commands.
                context = Some(new_context);
            }
            DatabaseCommand::Increment => {
                increment_counter(&client, context.as_ref().expect("never initialized")).await;
            }
        }
    }
}

/// Listens for `PubSub` events that come in from other clients pressing the
/// button.
async fn watch_for_changes(client: Client<ExampleApi>, context: DatabaseContext) {
    // Connect to a database, so that we can use `PubSub`. Usually a database
    // will have a Schema that allows storing collections of data. This example
    // only needs a simple counter, so we don't provide a schema.
    let database = client.database::<()>(DATABASE_NAME).await.unwrap();
    // Create a `PubSub` subscriber.
    let subscriber = database.create_subscriber().await.unwrap();
    // Subscribe to the counter changed topic. This topic is the one that the
    // server will publish messages to when the counter is incremented.
    subscriber
        .subscribe_to(COUNTER_CHANGED_TOPIC)
        .await
        .unwrap();

    while let Ok(message) = subscriber.receiver().recv_async().await {
        // We only need to worry about a single topic, but if you subscribed to
        // multiple topics, `message` contains a `topic` field that you can use
        // to determine what type the payload contains. For this example, the server
        // sends the current value as a `u64` for our topic.
        let new_count = message.payload::<u64>().unwrap();
        context
            .context
            .with_widget_mut(&context.button_id, |button: &mut Button, context| {
                button.set_label(new_count.to_string(), context);
            });
    }
}

async fn increment_counter(client: &Client<ExampleApi>, context: &DatabaseContext) {
    // While we could use the key value store directly, this example is showing
    // another powerful feature of PliantDb: the ablity to easily add a custom
    // api using your own enums.
    match client.send_api_request(Request::IncrementCounter).await {
        Ok(response) => {
            // Our API can only respond with one value, so let's destructure it and get the
            // response out.
            let Response::CounterIncremented(count) = response;
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
