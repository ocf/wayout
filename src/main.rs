use std::sync::mpsc;
use std::thread;

mod counter;
mod watcher;

// The main function of our program
fn main() {
    // Create a channel for the watcher to send the start signal to the counter
    let (start_tx, start_rx) = mpsc::channel();

    // We run the watcher and counter in separate threads, since the counter
    // needs to be able to run its own event loop.
    let watcher_thread: thread::JoinHandle<()> = thread::spawn(|| watcher::run(start_tx));
    let counter_thread = thread::spawn(|| counter::run("edu.berkeley.ocf.wayout", start_rx));

    // Wait for both threads to finish
    watcher_thread.join().unwrap();
    counter_thread.join().unwrap();
}
