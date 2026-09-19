//! Synthetic freedesktop server, exclusively inside the test's private D-Bus session.
use gio::glib::{self, variant::ToVariant};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

const PATH: &str = "/org/freedesktop/Notifications";
const INTERFACE: &str = "org.freedesktop.Notifications";
const XML: &str = r#"<node><interface name="org.freedesktop.Notifications">
<method name="GetCapabilities"><arg direction="out" type="as"/></method>
<method name="Notify"><arg direction="in" type="s"/><arg direction="in" type="u"/>
<arg direction="in" type="s"/><arg direction="in" type="s"/><arg direction="in" type="s"/>
<arg direction="in" type="as"/><arg direction="in" type="a{sv}"/><arg direction="in" type="i"/>
<arg direction="out" type="u"/></method>
<method name="CloseNotification"><arg direction="in" type="u"/></method>
<signal name="ActionInvoked"><arg type="u"/><arg type="s"/></signal>
</interface><interface name="org.kde.StatusNotifierWatcher">
<method name="RegisterStatusNotifierItem"><arg direction="in" type="s"/></method>
<method name="RegisterStatusNotifierHost"><arg direction="in" type="s"/></method>
<property name="ProtocolVersion" type="i" access="read"/>
<property name="RegisteredStatusNotifierItems" type="as" access="read"/>
<property name="IsStatusNotifierHostRegistered" type="b" access="read"/>
</interface></node>"#;

pub struct Server {
    connection: gio::DBusConnection,
    received: Arc<Mutex<u32>>,
    closed: Arc<Mutex<Vec<u32>>>,
    stop: Arc<AtomicBool>,
    host: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Server {
    pub fn start() -> Self {
        let received = Arc::new(Mutex::new(0));
        let closed = Arc::new(Mutex::new(vec![]));
        let stop = Arc::new(AtomicBool::new(false));
        let host = Arc::new(AtomicBool::new(false));
        let registered = host.clone();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let (calls, closures, stopping) = (received.clone(), closed.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            let context = glib::MainContext::new();
            context
                .with_thread_default(|| {
                    let connection =
                        gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).unwrap();
                    let reply = connection
                        .call_sync(
                            Some("org.freedesktop.DBus"),
                            "/org/freedesktop/DBus",
                            "org.freedesktop.DBus",
                            "RequestName",
                            Some(&(INTERFACE, 4_u32).to_variant()),
                            None,
                            gio::DBusCallFlags::NONE,
                            1000,
                            gio::Cancellable::NONE,
                        )
                        .unwrap();
                    assert_eq!(
                        reply.get::<(u32,)>(),
                        Some((1,)),
                        "fixture must own its private notification server"
                    );
                    let info = gio::DBusNodeInfo::for_xml(XML).unwrap();
                    let watcher = "org.kde.StatusNotifierWatcher";
                    connection
                        .call_sync(
                            Some("org.freedesktop.DBus"),
                            "/org/freedesktop/DBus",
                            "org.freedesktop.DBus",
                            "RequestName",
                            Some(&(watcher, 4_u32).to_variant()),
                            None,
                            gio::DBusCallFlags::NONE,
                            1000,
                            gio::Cancellable::NONE,
                        )
                        .unwrap();
                    let watcher_registration = connection
                        .register_object(
                            "/StatusNotifierWatcher",
                            &info.lookup_interface(watcher).unwrap(),
                            |_, _, _, _, _, _, invocation| {
                                invocation.return_value(Some(&().to_variant()))
                            },
                            move |_, _, _, _, property| match property {
                                "IsStatusNotifierHostRegistered" => {
                                    registered.load(Ordering::SeqCst).to_variant()
                                }
                                "RegisteredStatusNotifierItems" => {
                                    Vec::<String>::new().to_variant()
                                }
                                "ProtocolVersion" => 0_i32.to_variant(),
                                _ => panic!("unexpected indicator property"),
                            },
                            |_, _, _, _, _, _| false,
                        )
                        .unwrap();
                    let registration = connection
                        .register_object(
                            PATH,
                            &info.lookup_interface(INTERFACE).unwrap(),
                            move |_, _, _, _, method, params, invocation| match method {
                                "GetCapabilities" => invocation.return_value(Some(
                                    &(vec!["actions", "body-markup"],).to_variant(),
                                )),
                                "Notify" => {
                                    assert_eq!(
                                        params.child_value(0).get::<String>().unwrap(),
                                        "Stream GUI RS"
                                    );
                                    assert_eq!(
                                        params.child_value(3).get::<String>().unwrap(),
                                        "Synthetic channel is live"
                                    );
                                    assert_eq!(
                                        params.child_value(4).get::<String>().unwrap(),
                                        "&lt;Test&gt; &amp; title\nCategory"
                                    );
                                    assert_eq!(
                                        params.child_value(5).get::<Vec<String>>().unwrap(),
                                        ["default", "Show channel"]
                                    );
                                    let mut count = calls.lock().unwrap();
                                    *count += 1;
                                    invocation.return_value(Some(&(*count,).to_variant()));
                                }
                                "CloseNotification" => {
                                    closures
                                        .lock()
                                        .unwrap()
                                        .push(params.get::<(u32,)>().unwrap().0);
                                    invocation.return_value(Some(&().to_variant()));
                                }
                                _ => panic!("unexpected notification method"),
                            },
                            |_, _, _, _, _| ().to_variant(),
                            |_, _, _, _, _, _| false,
                        )
                        .unwrap();
                    sender.send(connection.clone()).unwrap();
                    while !stopping.load(Ordering::SeqCst) {
                        for _ in 0..64 {
                            if !context.pending() {
                                break;
                            }
                            context.iteration(false);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                    connection.unregister_object(registration).unwrap();
                    connection.unregister_object(watcher_registration).unwrap();
                })
                .unwrap();
        });
        Self {
            connection: receiver.recv().unwrap(),
            received,
            closed,
            stop,
            host,
            thread: Some(thread),
        }
    }
    pub fn delivered(&self) -> u32 {
        *self.received.lock().unwrap()
    }
    pub fn set_host(&self, registered: bool) {
        self.host.store(registered, Ordering::SeqCst);
    }
    pub fn closed(&self, id: u32) -> bool {
        self.closed.lock().unwrap().contains(&id)
    }
    pub fn click(&self, id: u32) {
        self.connection
            .emit_signal(
                None,
                PATH,
                INTERFACE,
                "ActionInvoked",
                Some(&(id, "default").to_variant()),
            )
            .unwrap();
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().unwrap().join().unwrap();
    }
}
