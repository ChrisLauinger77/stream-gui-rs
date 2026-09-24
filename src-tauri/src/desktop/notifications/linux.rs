//! One GIO worker owns D-Bus notifications and action subscriptions. No waiter
//! thread per notification; the registry and native call deadlines are bounded.
use super::*;
use gio::{
    glib::{self, variant::ToVariant},
    prelude::*,
};
use std::collections::HashMap;

type Records = Arc<Mutex<HashMap<u32, (LiveNotification, Instant)>>>;
pub(super) struct Worker {
    shared: Arc<Shared>,
    context: glib::MainContext,
    proxy: Option<gio::DBusProxy>,
    records: Records,
}
impl Worker {
    pub fn new(shared: Arc<Shared>) -> Self {
        let mut worker = Self {
            shared,
            context: glib::MainContext::new(),
            proxy: None,
            records: Arc::default(),
        };
        worker.refresh(false);
        worker
    }
    pub fn refresh(&mut self, _: bool) {
        if self.proxy.is_none() {
            let records = self.records.clone();
            let shared = self.shared.clone();
            self.proxy = self
                .context
                .with_thread_default(|| {
                    let proxy = gio::DBusProxy::for_bus_sync(
                        gio::BusType::Session,
                        gio::DBusProxyFlags::DO_NOT_LOAD_PROPERTIES,
                        None,
                        "org.freedesktop.Notifications",
                        "/org/freedesktop/Notifications",
                        "org.freedesktop.Notifications",
                        gio::Cancellable::NONE,
                    )
                    .ok()?;
                    let actions = records.clone();
                    proxy.connect_g_signal(move |_, _, signal, params| {
                        if signal == "ActionInvoked" {
                            let event = params
                                .get::<(u32, String)>()
                                .filter(|(_, action)| action == "default")
                                .and_then(|(id, _)| {
                                    actions
                                        .lock()
                                        .expect("notification registry poisoned")
                                        .remove(&id)
                                });
                            if let Some((event, _)) = event {
                                shared.activate(&event);
                            }
                        } else if signal == "NotificationClosed" {
                            if let Some((id, _)) = params.get::<(u32, u32)>() {
                                actions
                                    .lock()
                                    .expect("notification registry poisoned")
                                    .remove(&id);
                            }
                        }
                    });
                    // A replacement server can reuse IDs. Old actions cannot select new records.
                    proxy.connect_g_name_owner_notify(move |_| {
                        records
                            .lock()
                            .expect("notification registry poisoned")
                            .clear();
                    });
                    Some(proxy)
                })
                .ok()
                .flatten();
        }
        let caps = self
            .proxy
            .as_ref()
            .and_then(|p| {
                p.call_sync(
                    "GetCapabilities",
                    None,
                    gio::DBusCallFlags::NONE,
                    2000,
                    gio::Cancellable::NONE,
                )
                .ok()
            })
            .and_then(|v| v.get::<(Vec<String>,)>());
        self.shared.permission(
            if caps.is_some() {
                NotificationPermission::OsManaged
            } else {
                NotificationPermission::Unavailable
            },
            caps.is_some_and(|(caps,)| caps.iter().any(|v| v == "actions")),
        );
        let host = self
            .proxy
            .as_ref()
            .and_then(|p| {
                p.connection()
                    .call_sync(
                        Some("org.kde.StatusNotifierWatcher"),
                        "/StatusNotifierWatcher",
                        "org.freedesktop.DBus.Properties",
                        "Get",
                        Some(
                            &(
                                "org.kde.StatusNotifierWatcher",
                                "IsStatusNotifierHostRegistered",
                            )
                                .to_variant(),
                        ),
                        None,
                        gio::DBusCallFlags::NONE,
                        1000,
                        gio::Cancellable::NONE,
                    )
                    .ok()
            })
            .and_then(|v| v.get::<(glib::Variant,)>())
            .and_then(|(value,)| value.get::<bool>())
            .unwrap_or(false);
        self.shared
            .state
            .lock()
            .expect("native notification state poisoned")
            .tray_host = host;
    }
    pub fn deliver(&mut self, event: LiveNotification) -> Result<()> {
        self.tick();
        let Some(proxy) = &self.proxy else {
            return Err(delivery_error());
        };
        if self
            .records
            .lock()
            .expect("notification registry poisoned")
            .len()
            >= 32
        {
            return Err(delivery_error());
        }
        let clicks = self
            .shared
            .state
            .lock()
            .expect("native notification state poisoned")
            .clicks;
        let locale = self.shared.locale();
        let actions = if clicks {
            vec!["default", localization::text(locale, "native.showChannel")]
        } else {
            vec![]
        };
        let mut hints = HashMap::new();
        hints.insert("desktop-entry", "Stream GUI RS".to_variant());
        if event.cancelled() || self.shared.stop.is_cancelled() {
            return Ok(());
        }
        let response = proxy
            .call_sync(
                "Notify",
                Some(
                    &(
                        "Stream GUI RS",
                        0_u32,
                        "stream-gui-rs",
                        escaped(&localization::named(
                            locale,
                            "native.live",
                            &event.display_name,
                        )),
                        escaped(&format!("{}\n{}", event.title, event.category)),
                        actions,
                        hints,
                        10_000_i32,
                    )
                        .to_variant(),
                ),
                gio::DBusCallFlags::NONE,
                2000,
                gio::Cancellable::NONE,
            )
            .map_err(|_| delivery_error())?;
        let (id,) = response.get::<(u32,)>().ok_or_else(delivery_error)?;
        self.records
            .lock()
            .expect("notification registry poisoned")
            .insert(id, (event, Instant::now()));
        self.tick();
        Ok(())
    }
    pub fn tick(&mut self) {
        // Bound event processing even if an unrelated bus peer is noisy.
        for _ in 0..64 {
            if !self.context.pending() {
                break;
            }
            self.context.iteration(false);
        }
        let expired: Vec<_> = self
            .records
            .lock()
            .expect("notification registry poisoned")
            .iter()
            .filter(|(_, (event, sent))| {
                event.cancelled()
                    || self.shared.stop.is_cancelled()
                    || sent.elapsed() > Duration::from_secs(900)
            })
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            self.records
                .lock()
                .expect("notification registry poisoned")
                .remove(&id);
            if let Some(proxy) = &self.proxy {
                let _ = proxy.call_sync(
                    "CloseNotification",
                    Some(&(id,).to_variant()),
                    gio::DBusCallFlags::NONE,
                    250,
                    gio::Cancellable::NONE,
                );
            }
        }
    }
    pub fn shutdown(&mut self) {
        self.tick();
    }
}
