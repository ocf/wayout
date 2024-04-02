use std::collections::HashMap;
use std::process::{exit, Command};
use std::sync::mpsc;
use std::time::Duration;

use zbus::blocking::Connection;
use zbus::Result;
use zbus::{proxy, zvariant::Value};

const APP_NAME: &str = "Auto Logout";

// TODO: make this configurable
const IMMUNE_GROUPS: [&str; 2] = ["ocfstaff", "opstaff"];

mod dbus {
    #![allow(clippy::too_many_arguments)] // This is an external API

    use super::*;

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
}

use dbus::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Action {
    LogOut,
    LockSession,
}

impl Action {
    pub fn get_action_for_current_credentials() -> Self {
        let groups = nix::unistd::getgroups().unwrap();
        let is_immune = IMMUNE_GROUPS
            .iter()
            .filter_map(|group_name| {
                nix::unistd::Group::from_name(group_name).unwrap().map(|g| g.gid)
            })
            .any(|immune_gid| groups.iter().any(|gid| *gid == immune_gid));

        if is_immune {
            Action::LockSession
        } else {
            Action::LogOut
        }
    }

    pub fn execute(&self) {
        match self {
            Action::LogOut => {
                Command::new("loginctl")
                    .arg("kill-session")
                    .arg(std::env::var_os("XDG_SESSION_ID").unwrap())
                    .output()
                    .unwrap();
            }

            Action::LockSession => {
                Command::new("loginctl")
                    .arg("lock-session")
                    .arg(std::env::var_os("XDG_SESSION_ID").unwrap())
                    .output()
                    .unwrap();
            }
        }
    }
}

/// A message to start the countdown
pub struct StartCountdown {
    /// A channel to send a signal to cancel the countdown on
    pub cancel_rx: mpsc::Receiver<()>,
}

/// Run the counter application, listening for start signals from the watcher
/// and starting the countdown when they are received. Blocks the thread.
pub fn run(countdown_secs: u64, start_rx: mpsc::Receiver<StartCountdown>) {
    let connection = Connection::session().unwrap();
    let proxy = NotificationsProxyBlocking::new(&connection).unwrap();

    let action = Action::get_action_for_current_credentials();

    // Listen for start signals from the watcher and start the countdown
    while let Ok(StartCountdown { cancel_rx }) = start_rx.recv() {
        if let Err(err) = start_countdown(&action, &proxy, countdown_secs, cancel_rx) {
            eprintln!("Error starting countdown: {}", err);
        };
    }
}

fn start_countdown(
    action: &Action,
    proxy: &NotificationsProxyBlocking,
    countdown_secs: u64,
    cancel_rx: mpsc::Receiver<()>,
) -> Result<()> {
    let mut replaces_id = 0;

    for seconds in (1..=countdown_secs).rev() {
        // Save the ID of the notification so we can update it later
        replaces_id = proxy.notify(
            APP_NAME,
            replaces_id,
            "data-warning",
            "Still there?",
            &format!(
                "{} in {} seconds...",
                match action {
                    Action::LogOut => "Logging you out",
                    Action::LockSession => "Locking your session",
                },
                seconds
            ),
            &[],
            HashMap::new(),
            1100,
        )?;

        // Wait for 1 second, or until the cancel signal is received
        if cancel_rx.recv_timeout(Duration::from_secs(1)).is_ok() {
            cancel_countdown(action, proxy, replaces_id)?;
            return Ok(());
        }
    }

    action.execute();
    exit(0);
}

fn cancel_countdown(
    action: &Action,
    proxy: &NotificationsProxyBlocking,
    replaces_id: u32,
) -> Result<u32> {
    proxy.notify(
        APP_NAME,
        replaces_id,
        "data-success",
        "Still there!",
        &format!(
            "{} has been canceled.",
            match action {
                Action::LogOut => "Logging out",
                Action::LockSession => "Locking session",
            }
        ),
        &[],
        HashMap::new(),
        5000,
    )
}
