use clap::Parser;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod counter;
mod watcher;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Idle logout duration, including countdown
    #[arg(short = 't', long, default_value = "10m")]
    timeout: humantime::Duration,

    /// Countdown notification duration
    #[arg(short = 'c', long, default_value = "3m")]
    countdown: humantime::Duration,
}

fn main() {
    // Parse command-line arguments. Then, calculate the number of seconds
    // to count down for (rounding down to a second boundary) and the
    // duration to wait for the user to become idle.
    let args = Args::parse();
    let countdown_secs = args.countdown.min(*args.timeout).as_secs();
    let wait_millis = args.timeout.saturating_sub(Duration::from_secs(countdown_secs)).as_millis();
    let wait_millis: u32 = wait_millis.try_into().unwrap();

    // Create a channel for the watcher to send the start signal to the counter
    let (start_tx, start_rx) = mpsc::channel();

    // We run the watcher and counter in separate threads, since the counter
    // needs to be able to run its own event loop.
    let watcher_thread = thread::spawn(move || watcher::run(wait_millis, start_tx));
    let counter_thread = thread::spawn(move || counter::run(countdown_secs, start_rx));

    // Wait for both threads to finish
    watcher_thread.join().unwrap();
    counter_thread.join().unwrap();
}
