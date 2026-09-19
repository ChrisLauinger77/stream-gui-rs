//! Service-owned followed-live monitoring. No webview or native API dependency.
mod state;
use crate::{
    config::SettingsStore,
    domain::{ErrorCode, Result},
    helix::HelixClient,
    time::Clock,
    twitch::{AuthService, api::TwitchApi},
};
use serde::Serialize;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{sync::Notify, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum MonitorPhase {
    Disabled,
    SignedOut,
    Paused,
    Baseline,
    Running,
    Recovering,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorStatus {
    pub phase: MonitorPhase,
    pub paused: bool,
    pub live_count: Option<u32>,
    pub stale: bool,
    pub error: Option<ErrorCode>,
    pub retry_in_seconds: u32,
    pub notification_error: bool,
}
impl Default for MonitorStatus {
    fn default() -> Self {
        Self {
            phase: MonitorPhase::Disabled,
            paused: false,
            live_count: None,
            stale: false,
            error: None,
            retry_in_seconds: 0,
            notification_error: false,
        }
    }
}

#[derive(Clone)]
pub struct LiveNotification {
    pub auth_session_id: String,
    pub broadcaster_id: String,
    pub display_name: String,
    pub title: String,
    pub category: String,
    pub session_cancel: CancellationToken,
    pub monitor_cancel: CancellationToken,
}
impl LiveNotification {
    pub fn cancelled(&self) -> bool {
        self.session_cancel.is_cancelled() || self.monitor_cancel.is_cancelled()
    }
}
/// Native adapters enqueue bounded work and recheck both cancellations at dispatch/click.
pub trait NotificationSink: Send + Sync {
    fn deliver(&self, notification: LiveNotification) -> Result<()>;
}
struct Unavailable;
impl NotificationSink for Unavailable {
    fn deliver(&self, _: LiveNotification) -> Result<()> {
        Err(crate::domain::AppError::new(
            ErrorCode::Notification,
            "Desktop notifications are unavailable.",
        ))
    }
}
struct Control {
    paused: bool,
    cancel: CancellationToken,
}
pub struct Monitor {
    control: Mutex<Control>,
    status: Mutex<MonitorStatus>,
    changed: Notify,
    stop: CancellationToken,
    started: AtomicBool,
    task: Mutex<Option<JoinHandle<()>>>,
    sink: Mutex<Arc<dyn NotificationSink>>,
    clock: Clock,
    #[cfg(test)]
    ticks: tokio::sync::watch::Sender<u64>,
}
impl Default for Monitor {
    fn default() -> Self {
        Self {
            control: Mutex::new(Control {
                paused: false,
                cancel: CancellationToken::new(),
            }),
            status: Mutex::default(),
            changed: Notify::new(),
            stop: CancellationToken::new(),
            started: AtomicBool::new(false),
            task: Mutex::default(),
            sink: Mutex::new(Arc::new(Unavailable)),
            clock: Clock::default(),
            #[cfg(test)]
            ticks: tokio::sync::watch::channel(0).0,
        }
    }
}
impl Monitor {
    #[cfg(test)]
    pub(crate) fn for_test() -> Self {
        Self {
            clock: Clock::for_test(),
            ..Default::default()
        }
    }
    #[cfg(test)]
    pub(crate) async fn test_advance(&self, seconds: u64) {
        let mut ticks = self.ticks.subscribe();
        self.test_elapse(seconds);
        self.changed.notify_one();
        tokio::time::timeout(Duration::from_secs(3), ticks.changed())
            .await
            .unwrap()
            .unwrap();
    }
    #[cfg(test)]
    pub(crate) fn test_elapse(&self, seconds: u64) {
        self.clock.advance_wall(Duration::from_secs(seconds));
    }

    pub fn snapshot(&self) -> MonitorStatus {
        self.status.lock().expect("monitor mutex poisoned").clone()
    }
    pub fn set_sink(&self, sink: Arc<dyn NotificationSink>) {
        *self.sink.lock().expect("notification mutex poisoned") = sink;
    }
    pub fn notification_failed(&self) {
        self.status
            .lock()
            .expect("monitor mutex poisoned")
            .notification_error = true;
    }
    pub fn pause(&self, paused: bool) -> MonitorStatus {
        let mut control = self.control.lock().expect("monitor control poisoned");
        if control.paused != paused {
            control.paused = paused;
            control.cancel.cancel();
            control.cancel = CancellationToken::new();
        }
        let mut status = self.status.lock().expect("monitor mutex poisoned");
        status.paused = paused;
        status.phase = if paused {
            MonitorPhase::Paused
        } else {
            MonitorPhase::Baseline
        };
        status.stale = true;
        self.changed.notify_one();
        status.clone()
    }
    pub fn reconfigure(&self) {
        let mut control = self.control.lock().expect("monitor control poisoned");
        control.cancel.cancel();
        control.cancel = CancellationToken::new();
        self.changed.notify_one();
    }
    pub fn start<A: TwitchApi + 'static>(
        self: &Arc<Self>,
        auth: Arc<AuthService<A>>,
        helix: Arc<HelixClient<A>>,
        settings: Arc<SettingsStore>,
    ) {
        if self.started.swap(true, Ordering::SeqCst) || self.stop.is_cancelled() {
            return;
        }
        let owner = self.clone();
        let mut task = self.task.lock().expect("monitor task poisoned");
        *task = Some(tokio::spawn(async move {
            owner.run(auth, helix, settings).await;
        }));
    }
    pub async fn shutdown(&self) {
        self.stop.cancel();
        self.reconfigure();
        let task = self.task.lock().expect("monitor task poisoned").take();
        if let Some(task) = task {
            let _ = task.await;
        }
        *self.status.lock().expect("monitor mutex poisoned") = MonitorStatus {
            phase: MonitorPhase::Stopped,
            ..Default::default()
        };
    }
    async fn run<A: TwitchApi + 'static>(
        &self,
        auth: Arc<AuthService<A>>,
        helix: Arc<HelixClient<A>>,
        settings: Arc<SettingsStore>,
    ) {
        let mut machine = state::Transitions::default();
        let mut session = None;
        let mut generation = CancellationToken::new();
        let mut next = Duration::ZERO;
        let mut last_tick = self.clock.now();
        let mut failures: u32 = 0;
        let jitter = u64::from(uuid::Uuid::new_v4().as_bytes()[0]) % 11;
        loop {
            if self.stop.is_cancelled() {
                break;
            }
            let now = self.clock.now();
            // Checking once a second detects suspend on clocks that include it as
            // well as those that pause. A long scheduler stall is treated safely too.
            if now.saturating_sub(last_tick) > Duration::from_secs(10) {
                machine.rebaseline();
                next = now;
            }
            last_tick = now;
            let identity = auth.monitor_session();
            let id = identity.as_ref().map(|(id, _)| *id);
            if id != session {
                machine = state::Transitions::default();
                session = id;
                next = now;
                failures = 0;
            }
            let (paused, control) = {
                let control = self.control.lock().expect("monitor control poisoned");
                (control.paused, control.cancel.clone())
            };
            if generation.is_cancelled() {
                machine.rebaseline();
                next = now;
                failures = 0;
            }
            generation = control;
            let prefs = settings.snapshot().background;
            let inactive = if !prefs.monitoring_enabled {
                Some(MonitorPhase::Disabled)
            } else if paused {
                Some(MonitorPhase::Paused)
            } else if id.is_none() {
                Some(MonitorPhase::SignedOut)
            } else {
                None
            };
            if let Some(phase) = inactive {
                machine.rebaseline();
                *self.status.lock().expect("monitor mutex poisoned") = MonitorStatus {
                    phase,
                    paused,
                    ..Default::default()
                };
            } else if now >= next {
                let (id, session_cancel) = identity.expect("active session");
                self.status.lock().expect("monitor mutex poisoned").phase = if machine.ready() {
                    MonitorPhase::Running
                } else {
                    MonitorPhase::Baseline
                };
                let heartbeat = Mutex::new(self.clock.now());
                let mut result = tokio::select! {
                    biased;
                    _ = self.stop.cancelled() => break,
                    _ = generation.cancelled() => continue,
                    _ = session_cancel.cancelled() => continue,
                    _ = suspend_gap(&self.clock, &heartbeat) => {
                        // A response spanning suspend cannot establish current state.
                        helix.invalidate(crate::helix::cache::CacheClass::Live);
                        Err(crate::twitch_http::error(ErrorCode::Timeout))
                    },
                    result = tokio::time::timeout(Duration::from_secs(45), helix.monitor_followed(id, &generation)) => result.unwrap_or_else(|_| Err(crate::twitch_http::error(ErrorCode::Timeout))),
                };
                if generation.is_cancelled() || session_cancel.is_cancelled() {
                    continue;
                }
                let finished = self.clock.now();
                // A response can wake us before the heartbeat timer after suspend.
                // Reconcile before accepting it or overwriting the elapsed boundary.
                if suspend_since(finished, &heartbeat) {
                    helix.invalidate(crate::helix::cache::CacheClass::Live);
                    result = Err(crate::twitch_http::error(ErrorCode::Timeout));
                }
                last_tick = finished;
                match result {
                    Ok(streams) => {
                        let events = machine.accept(streams);
                        failures = 0;
                        let mut notification_error = false;
                        for stream in events
                            .into_iter()
                            .filter(|stream| settings.notifications(&stream.user_id))
                            .take(10)
                        {
                            let event = LiveNotification {
                                auth_session_id: id.to_string(),
                                broadcaster_id: stream.user_id,
                                display_name: compact(&stream.user_name, 100),
                                title: compact(&stream.title, 240),
                                category: compact(&stream.game_name, 80),
                                session_cancel: session_cancel.clone(),
                                monitor_cancel: generation.clone(),
                            };
                            if event.cancelled() {
                                break;
                            }
                            notification_error |= self
                                .sink
                                .lock()
                                .expect("notification mutex poisoned")
                                .deliver(event)
                                .is_err();
                        }
                        *self.status.lock().expect("monitor mutex poisoned") = MonitorStatus {
                            phase: MonitorPhase::Running,
                            live_count: Some(machine.live_count()),
                            notification_error,
                            ..Default::default()
                        };
                        next = finished
                            + Duration::from_secs(u64::from(prefs.interval_seconds) + jitter);
                    }
                    Err(error) => {
                        machine.rebaseline();
                        failures = failures.saturating_add(1);
                        let delay = backoff(failures, prefs.interval_seconds, jitter);
                        next = finished + delay;
                        let mut status = self.status.lock().expect("monitor mutex poisoned");
                        status.phase = MonitorPhase::Recovering;
                        status.stale = true;
                        status.error = Some(error.code);
                    }
                }
            }
            self.status
                .lock()
                .expect("monitor mutex poisoned")
                .retry_in_seconds = next
                .saturating_sub(self.clock.now())
                .as_secs()
                .min(u32::MAX.into()) as u32;
            #[cfg(test)]
            self.ticks.send_modify(|tick| *tick += 1);
            tokio::select! {
                biased;
                _ = self.stop.cancelled() => break,
                _ = self.changed.notified() => {},
                _ = tokio::time::sleep(Duration::from_secs(1)) => {},
            }
        }
    }
}
fn suspend_since(now: Duration, heartbeat: &Mutex<Duration>) -> bool {
    now.saturating_sub(*heartbeat.lock().expect("monitor heartbeat poisoned"))
        > Duration::from_secs(10)
}
async fn suspend_gap(clock: &Clock, heartbeat: &Mutex<Duration>) {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let current = clock.now();
        if suspend_since(current, heartbeat) {
            return;
        }
        *heartbeat.lock().expect("monitor heartbeat poisoned") = current;
    }
}
fn compact(text: &str, max: usize) -> String {
    text.chars().filter(|c| !c.is_control()).take(max).collect()
}
fn backoff(failures: u32, interval: u32, jitter: u64) -> Duration {
    Duration::from_secs(
        (u64::from(interval) * 2_u64.pow(failures.saturating_sub(1).min(4))).min(900) + jitter,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn ordinary_slow_scan_keeps_a_current_heartbeat() {
        let clock = Clock::for_test();
        let heartbeat = Mutex::new(clock.now());
        let guard = suspend_gap(&clock, &heartbeat);
        tokio::pin!(guard);
        for _ in 0..20 {
            tokio::select! {
                biased;
                _ = &mut guard => panic!("ordinary elapsed uptime is not suspend"),
                _ = tokio::time::sleep(Duration::from_secs(1)) => {},
            }
        }
        assert!(!suspend_since(clock.now(), &heartbeat));
    }
    #[test]
    fn retry_delays_are_bounded_and_jitter_does_not_accumulate() {
        assert_eq!(backoff(1, 60, 7), Duration::from_secs(67));
        assert_eq!(backoff(2, 60, 7), Duration::from_secs(127));
        assert_eq!(backoff(30, 300, 10), Duration::from_secs(910));
        assert_eq!(backoff(1, 60, 7), Duration::from_secs(67));
        assert_eq!(compact("Hello\n\0World", 7), "HelloWo");
    }
}
