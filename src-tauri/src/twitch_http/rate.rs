use crate::domain::{AppError, ErrorCode, Result};
use reqwest::header::HeaderMap;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{sync::Notify, time::Instant};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, Default)]
pub struct RateSnapshot {
    pub limit: Option<u32>,
    pub remaining: Option<u32>,
    pub reset_at: Option<u64>,
    pub in_flight: u32,
}
#[derive(Default)]
struct Budget {
    snapshot: RateSnapshot,
    reset: Option<Instant>,
}
#[derive(Default)]
pub struct RateLimiter {
    budget: Mutex<Budget>,
    changed: Notify,
}
pub struct Reservation {
    rate: Arc<RateLimiter>,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        self.rate
            .budget
            .lock()
            .expect("rate mutex poisoned")
            .snapshot
            .in_flight -= 1;
        self.rate.changed.notify_waiters();
    }
}
impl RateLimiter {
    pub fn snapshot(&self) -> RateSnapshot {
        self.budget
            .lock()
            .expect("rate mutex poisoned")
            .snapshot
            .clone()
    }
    pub async fn reserve(self: &Arc<Self>, cancel: &CancellationToken) -> Result<Reservation> {
        loop {
            // Register before examining state so a completed request cannot
            // notify between unlocking the budget and registering our waiter.
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let wait = {
                let mut budget = self.budget.lock().expect("rate mutex poisoned");
                if cancel.is_cancelled() {
                    return Err(cancelled());
                }
                if budget.reset.is_some_and(|reset| reset <= Instant::now()) {
                    // Probe the new window conservatively rather than inventing
                    // a fresh full budget before Twitch confirms its headers.
                    budget.snapshot.remaining = None;
                    budget.reset = None;
                }
                let available = budget
                    .snapshot
                    .remaining
                    .map_or(budget.snapshot.in_flight == 0, |n| n > 0);
                if available && budget.snapshot.in_flight < 4 {
                    if let Some(remaining) = &mut budget.snapshot.remaining {
                        *remaining -= 1;
                    }
                    budget.snapshot.in_flight += 1;
                    return Ok(Reservation { rate: self.clone() });
                }
                budget.reset
            };
            tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(cancelled()),
                _ = &mut changed => {},
                _ = async { match wait { Some(deadline) => tokio::time::sleep_until(deadline).await, None => std::future::pending().await } } => {},
            }
        }
    }
    pub fn observe(&self, headers: &HeaderMap, limited: bool) {
        let parse = |key: &str| {
            headers
                .get(key)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
        };
        let mut budget = self.budget.lock().expect("rate mutex poisoned");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if let (Some(limit), Some(remaining), Some(reset)) = (
            parse("ratelimit-limit"),
            parse("ratelimit-remaining"),
            parse("ratelimit-reset"),
        ) {
            if limit > 0
                && limit <= 100_000
                && remaining <= limit
                && reset <= now.saturating_add(86400)
            {
                let newer = budget
                    .snapshot
                    .reset_at
                    .is_none_or(|previous| reset > previous);
                let same = budget.snapshot.reset_at == Some(reset);
                if newer || same {
                    let observed = (remaining as u32)
                        .saturating_sub(budget.snapshot.in_flight.saturating_sub(1));
                    budget.snapshot.remaining = Some(if newer {
                        observed
                    } else {
                        budget
                            .snapshot
                            .remaining
                            .map_or(observed, |local| local.min(observed))
                    });
                    budget.snapshot.limit = Some(limit as u32);
                    budget.snapshot.reset_at = Some(reset);
                    budget.reset = Some(
                        Instant::now() + Duration::from_secs(reset.saturating_sub(now).max(1)),
                    );
                }
            }
        }
        if limited {
            budget.snapshot.remaining = Some(0);
            let delay = parse("ratelimit-reset")
                .map(|reset| reset.saturating_sub(now))
                .or_else(|| parse("retry-after"))
                .unwrap_or(1)
                .clamp(1, 86400);
            let until = Instant::now() + Duration::from_secs(delay);
            budget.reset = Some(budget.reset.map_or(until, |old| old.max(until)));
        }
        drop(budget);
        self.changed.notify_waiters();
    }
}
fn cancelled() -> AppError {
    AppError::new(ErrorCode::Cancelled, "The Twitch request was cancelled.")
}
