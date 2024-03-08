use std::collections::HashMap;
use std::process::{exit, Command};
use std::sync::mpsc;
use std::time::Duration;

use zbus::blocking::Connection;
use zbus::Result;
use zbus::{proxy, zvariant::Value};

const APP_NAME: &str = "Auto Logout";
const APP_ICON: &str = "data-warning";
const COUNTDOWN_SECS: u64 = 20;

#[proxy(
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    /// Call the org.freedesktop.Notifications.Notify D-Bus method
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, &Value<'_>>,
        expire_timeout: i32,
    ) -> Result<u32>;
}

/// A message to start the countdown
pub struct StartCountdown {
    /// A channel to send a signal to cancel the countdown on
    pub cancel_rx: mpsc::Receiver<()>,
}

/// Run the counter application, listening for start signals from the watcher
/// and starting the countdown when they are received. Blocks the thread.
pub fn run(start_rx: mpsc::Receiver<StartCountdown>) {
    let connection = Connection::session().unwrap();
    let proxy = NotificationsProxyBlocking::new(&connection).unwrap();

    // Listen for start signals from the watcher and start the countdown
    while let Ok(StartCountdown { cancel_rx }) = start_rx.recv() {
        if let Err(err) = start_countdown(&proxy, cancel_rx) {
            eprintln!("Error starting countdown: {}", err);
        };
    }
}

fn start_countdown(
    proxy: &NotificationsProxyBlocking,
    cancel_rx: mpsc::Receiver<()>,
) -> Result<()> {
    let mut replaces_id = 0;

    for seconds in (1..=COUNTDOWN_SECS).rev() {
        // Save the ID of the notification so we can update it later
        replaces_id = proxy.notify(
            APP_NAME,
            replaces_id,
            APP_ICON,
            "Still there?",
            &format!("Logging you out in {} seconds...", seconds),
            &[],
            HashMap::new(),
            1100,
        )?;

        // Wait for 1 second, or until the cancel signal is received
        if cancel_rx.recv_timeout(Duration::from_secs(1)).is_ok() {
            cancel_countdown(proxy, replaces_id)?;
            return Ok(());
        }
    }

    log_out();
    exit(0);
}

fn cancel_countdown(proxy: &NotificationsProxyBlocking, replaces_id: u32) -> Result<u32> {
    proxy.notify(
        APP_NAME,
        replaces_id,
        APP_ICON,
        "Still there!",
        "Logging out has been canceled.",
        &[],
        HashMap::new(),
        5000,
    )
}

fn log_out() {
    Command::new("loginctl")
        .arg("terminate-session")
        .arg(std::env::var_os("XDG_SESSION_ID").unwrap())
        .output()
        .unwrap();
}
