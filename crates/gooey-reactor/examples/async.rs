use std::time::Duration;

use futures_util::StreamExt;
use gooey_reactor::Reactor;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let runtime = Reactor::default();
    let global = runtime.new_scope();
    let shutdown = global.new_dynamic(false);

    tokio::spawn(async move {
        println!("Sleeping");
        tokio::time::sleep(Duration::from_secs(1)).await;
        println!("Shutting down.");
        shutdown.set(true);
    });

    let mut shutdown = shutdown.into_stream();
    while shutdown.next().await == Some(false) {
        unreachable!("shutdown is never set to false");
    }

    println!("Shut down");
}
