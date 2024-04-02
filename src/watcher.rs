use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::Event::{
    Idled, Resumed,
};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::{
    self, ExtIdleNotificationV1,
};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1;

use crate::counter::StartCountdown;

/// Run the watcher application, listening for idle notifications from the
/// Wayland server and starting the countdown when all seats are idle.
/// Blocks the thread.
pub fn run(wait_millis: u32, start_tx: mpsc::Sender<StartCountdown>) {
    // Connect to the Wayland server and create an event queue
    let connection = Connection::connect_to_env().unwrap();
    let mut event_queue: EventQueue<WatcherState> = connection.new_event_queue();
    let qhandle = event_queue.handle();
    let _registry = connection.display().get_registry(&qhandle, ());

    // Create a state object and start the event loop
    let mut state = WatcherState::new(wait_millis, start_tx);
    loop {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
}

struct WatcherState {
    wait_millis: u32,
    start_tx: mpsc::Sender<StartCountdown>,
    cancel_tx: Option<mpsc::Sender<()>>,
    seats: HashMap<u32, WlSeat>,
    idle_seats: HashSet<u32>,
    notifier: Option<ExtIdleNotifierV1>,
}

impl WatcherState {
    pub fn new(wait_millis: u32, start_tx: mpsc::Sender<StartCountdown>) -> Self {
        Self {
            wait_millis,
            start_tx,
            cancel_tx: None,
            seats: HashMap::new(),
            idle_seats: HashSet::new(),
            notifier: None,
        }
    }
}

impl WatcherState {
    fn global(
        &mut self,
        registry: &WlRegistry,
        qhandle: &QueueHandle<Self>,
        name: u32,
        interface: String,
        version: u32,
    ) {
        match &interface[..] {
            "wl_seat" => {
                // A seat is a group of input devices, such as a keyboard and a
                // pointer. We need to bind to it to get a WlSeat struct, which
                // we'll need to set up idle notifications.
                let seat = registry.bind(name, version, qhandle, ());
                self.get_idle_notification(&seat, qhandle, name);
                self.seats.insert(name, seat);
                self.resumed(name);
            }

            "ext_idle_notifier_v1" => {
                // The notifier is the object that we use to set up idle
                // notifications. We only need to bind to it once, so we store
                // it in an Option field.
                let notifier: ExtIdleNotifierV1 = registry.bind(name, version, qhandle, ());
                self.notifier = Some(notifier);
                for seat in self.seats.values() {
                    self.get_idle_notification(seat, qhandle, name);
                }
            }

            _ => {}
        }
    }

    fn global_remove(&mut self, name: u32) {
        // We need to remove the seat from our list of seats when it's
        // destroyed, so that we don't try to watch it anymore.
        self.seats.remove(&name);
        self.idle_seats.remove(&name);
    }

    fn idled(&mut self, seat_name: u32) {
        // When a seat becomes idle, we add it to a set of idle seats. We'll
        // use this set to check if all seats are idle, and if so, to start the
        // logout countdown.
        self.idle_seats.insert(seat_name);
        if self.idle_seats.len() == self.seats.len() {
            let (cancel_tx, cancel_rx) = mpsc::channel();
            self.start_tx.send(StartCountdown { cancel_rx }).unwrap();
            self.cancel_tx = Some(cancel_tx);
        }
    }

    fn resumed(&mut self, seat_name: u32) {
        // When a seat is no longer idle, we remove it from the set of idle
        // seats. If the countdown has already started, we'll cancel it.
        self.idle_seats.remove(&seat_name);
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
    }

    fn get_idle_notification(&self, seat: &WlSeat, qhandle: &QueueHandle<Self>, name: u32) {
        if let Some(notifier) = &self.notifier {
            notifier.get_idle_notification(self.wait_millis, seat, qhandle, name);
        }
    }
}

// ========================
// Dispatch implementations
// ========================
// These are implemented to delegate to the simpler methods above, to reduce
// the amount of clutter in the event method.

// Dispatch implementation for WlRegistry.
// This is the singleton object that we use to bind to other objects.
// We're interested in the Global and GlobalRemove events, which tell us when
// new objects are created and destroyed.

impl Dispatch<WlRegistry, ()> for WatcherState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global { name, interface, version } => {
                state.global(registry, qhandle, name, interface, version)
            }
            wl_registry::Event::GlobalRemove { name } => state.global_remove(name),
            _ => {}
        }
    }
}

// Dispatch implementation for ExtIdleNotificationV1.
// Each *notification* is a configuration that tells the *notifier* to send
// events when a given seat becomes idle or resumes after a given timeout.

impl Dispatch<ExtIdleNotificationV1, u32> for WatcherState {
    fn event(
        state: &mut Self,
        _proxy: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        data: &u32,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            Idled => state.idled(*data),
            Resumed => state.resumed(*data),
            _ => {}
        }
    }
}

// Empty Dispatch implementations for WlSeat and ExtIdleNotifierV1.
// We need a Dispatch implementation to bind to an object, but we don't need to
// handle any events for these objects, so we can just leave the method empty.

macro_rules! impl_empty_dispatch {
    ($t:ty) => {
        impl Dispatch<$t, ()> for WatcherState {
            fn event(
                _: &mut Self,
                _: &$t,
                _: <$t as ::wayland_client::Proxy>::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    };
}

impl_empty_dispatch!(WlSeat);
impl_empty_dispatch!(ExtIdleNotifierV1);
