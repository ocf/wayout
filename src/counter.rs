use std::sync::mpsc;
use std::time::Duration;

use gio::prelude::{ApplicationExt, ApplicationExtManual};
use gio::{Application, Notification};

const COUNTDOWN_SECS: u64 = 20;
const NOTIFICATION_ID: Option<&str> = Some("idle-countdown");

/// A message to start the countdown
pub struct StartCountdown {
    /// A channel to send a signal to cancel the countdown on
    pub cancel_rx: mpsc::Receiver<()>,
}

/// Run the counter application, listening for start signals from the watcher
/// and starting the countdown when they are received. Blocks the thread.
pub fn run(application_id: &str, start_rx: mpsc::Receiver<StartCountdown>) {
    let application = Application::new(Some(application_id), Default::default());
    application.connect_activate(move |application| listen(application, &start_rx));
    application.run();
}

fn listen(application: &Application, start_rx: &mpsc::Receiver<StartCountdown>) {
    // Listen for start signals from the watcher and start the countdown
    while let Ok(StartCountdown { cancel_rx }) = start_rx.recv() {
        start_countdown(application, cancel_rx);
    }
}

fn start_countdown(application: &Application, cancel_rx: mpsc::Receiver<()>) {
    let notification = Notification::new("You still there?");

    for seconds in (0..COUNTDOWN_SECS).rev() {
        // Update the notification's body to show the remaining time
        // Sending with the same ID will update the existing notification
        notification.set_body(Some(&format!("Logging you out in {} seconds...", seconds)));
        application.send_notification(NOTIFICATION_ID, &notification);

        // Wait for 1 second, or until the cancel signal is received
        if cancel_rx.recv_timeout(Duration::from_secs(1)).is_ok() {
            return cancel_countdown(application);
        }
    }

    log_out();
}

fn cancel_countdown(application: &Application) {
    let notification = Notification::new("You're still there!");
    notification.set_body(Some("Logging out has been canceled."));
    application.send_notification(NOTIFICATION_ID, &notification);
}

fn log_out() {}
