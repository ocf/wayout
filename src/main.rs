use std::collections::HashMap;
use std::thread;

use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1, ext_idle_notifier_v1,
};

use gio::Notification;

const TIMEOUT_SECS: u32 = 20;

struct AppData {
    seats: HashMap<u32, wl_seat::WlSeat>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        &mut self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            // https://docs.rs/wayland-client/0.31.0/wayland_client/protocol/wl_registry/index.html
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => match &interface[..] {
                "wl_seat" => {
                    let seat = registry.bind(name, version, qhandle, *data);
                    self.seats.insert(name, seat);
                }
                "ext_idle_notifier_v1" => {
                    let a = registry.bind::<ext_idle_notifier_v1::ExtIdleNotifierV1, _, _>(
                        name, version, qhandle, *data,
                    );
                    a.get_idle_notification(TIMEOUT_SECS, seat, qh, udata)
                }
                "ext_idle_notification_v1" => {
                    registry.bind::<ext_idle_notification_v1::ExtIdleNotificationV1>(
                        name, version, qhandle, *data,
                    );
                }
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => self.seats.remove(&name),
            _ => {}
        }
    }
}

macro_rules! impl_empty_dispatch {
    ($t:ty) => {
        impl Dispatch<$t> for AppData {
            fn event(
                &mut self,
                proxy: &$t,
                event: <$t as ::wayland_client::Proxy>::Event,
                data: &(),
                conn: &Connection,
                qhandle: &QueueHandle<Self>,
            ) {
            }
        }
    };
}

impl_empty_dispatch!(wl_seat::WlSeat);
impl_empty_dispatch!(ext_idle_notifier_v1::ExtIdleNotifierV1);

impl Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1> for AppData {
    fn event(
        &mut self,
        proxy: &ext_idle_notification_v1::ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => {
                println!("idled");
            }
            ext_idle_notification_v1::Event::Resumed => {
                println!("resumed");
            }
            _ => {}
        }
    }
}

pub fn logout_countdown(seconds: u32) {
    // load icon here lmao
    let mut s = seconds;
    let notif = Notification::new("You still there?");
    while (s >= 0) {
        notif.set_body(format!("Logging you out in %d seconds...", s));
        s -= 1;
    }
}

// The main function of our program
fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let mut event_queue = conn.new_event_queue();
    let qhandle = event_queue.handle();

    let _registry = conn.display().get_registry(&qhandle, ());

    // ExtIdleNotifierV1::get_idle_notification();
    // https://docs.rs/wayland-protocols/0.31.0/wayland_protocols/ext/idle_notify/v1/client/ext_idle_notifier_v1/struct.ExtIdleNotifierV1.html

    loop {
        event_queue.blocking_dispatch(&mut AppData).unwrap();
        let countdown_handle = thread::spawn(logout_countdown(TIMEOUT_SECS));
    }
}
