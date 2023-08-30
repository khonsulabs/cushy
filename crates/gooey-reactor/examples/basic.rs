use std::thread;
use std::time::Duration;

use gooey_reactor::Reactor;

fn main() {
    let runtime = Reactor::default();
    let shutdown = runtime.new_dynamic(false);

    thread::spawn(move || {
        println!("Sleeping");
        thread::sleep(Duration::from_secs(1));
        println!("Shutting down.");
        shutdown.set(true);
    });

    let mut shutdown = shutdown.into_iter();
    while shutdown.next() == Some(false) {
        unreachable!("shutdown is never set to false");
    }

    println!("Shut down");
}
