use crate::{
    domain::{AppError, ErrorCode, Result},
    time::Clock,
};
use reqwest::header::HeaderMap;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, UNIX_EPOCH},
};
use tokio::sync::Notify;
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
    reset: Option<Duration>,
}
#[derive(Default)]
pub struct RateLimiter {
    budget: Mutex<Budget>,
    changed: Notify,
    clock: Clock,
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
        self.reserve_with_priority(cancel, false).await
    }
    pub(crate) async fn reserve_with_priority(
        self: &Arc<Self>,
        cancel: &CancellationToken,
        background: bool,
    ) -> Result<Reservation> {
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
                if budget.reset.is_some_and(|reset| reset <= self.clock.now()) {
                    // Probe the new window conservatively rather than inventing
                    // a fresh full budget before Twitch confirms its headers.
                    budget.snapshot.remaining = None;
                    budget.reset = None;
                }
                let available = budget
                    .snapshot
                    .remaining
                    .map_or(budget.snapshot.in_flight == 0, |n| n > 0);
                // Background work never queues ahead of browsing or consumes the
                // last tenth of a known window (at least one, at most 50 points).
                // Unknown/reset windows still allow one conservative probe.
                if background
                    && (!available
                        || budget.snapshot.in_flight >= 3
                        || budget.snapshot.remaining.is_some_and(|n| {
                            n <= budget
                                .snapshot
                                .limit
                                .unwrap_or(10)
                                .div_ceil(10)
                                .clamp(1, 50)
                        }))
                {
                    return Err(crate::twitch_http::error(ErrorCode::RateLimited));
                }
                if available && budget.snapshot.in_flight < 4 {
                    if let Some(remaining) = &mut budget.snapshot.remaining {
                        *remaining -= 1;
                    }
                    budget.snapshot.in_flight += 1;
                    return Ok(Reservation { rate: self.clone() });
                }
                budget.reset
            };
            // A short monotonic wait bounds resume reconciliation latency even
            // on platforms whose monotonic clock pauses during suspend.
            tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(cancelled()),
                _ = &mut changed => {},
                _ = async { match wait { Some(deadline) => tokio::time::sleep(deadline.saturating_sub(self.clock.now()).min(Duration::from_secs(1))).await, None => std::future::pending().await } } => {},
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
        let now = self
            .clock
            .unix_now()
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
                        self.clock.now() + Duration::from_secs(reset.saturating_sub(now).max(1)),
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
            let until = self.clock.now() + Duration::from_secs(delay);
            budget.reset = Some(budget.reset.map_or(until, |old| old.max(until)));
        }
        drop(budget);
        self.changed.notify_waiters();
    }
}
fn cancelled() -> AppError {
    AppError::new(ErrorCode::Cancelled, "The Twitch request was cancelled.")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limited(clock: &Clock) -> (Arc<RateLimiter>, HeaderMap) {
        let rate = Arc::new(RateLimiter {
            clock: clock.clone(),
            ..Default::default()
        });
        let reset = clock
            .unix_now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 60;
        let mut headers = HeaderMap::new();
        headers.insert("ratelimit-limit", "10".parse().unwrap());
        headers.insert("ratelimit-remaining", "0".parse().unwrap());
        headers.insert("ratelimit-reset", reset.to_string().parse().unwrap());
        rate.observe(&headers, true);
        (rate, headers)
    }

    #[tokio::test(start_paused = true)]
    async fn suspend_past_unix_reset_allows_one_probe_without_additional_uptime() {
        let clock = Clock::for_test();
        let (rate, _) = limited(&clock);
        let monotonic = tokio::time::Instant::now();
        clock.advance_wall(Duration::from_secs(120));
        let cancel = CancellationToken::new();
        let _probe = rate.reserve(&cancel).await.unwrap();
        assert_eq!(tokio::time::Instant::now(), monotonic);
        assert_eq!(rate.snapshot().remaining, None);
        assert_eq!(rate.snapshot().in_flight, 1);
        // An uncertain new window grants only one probe, not a fresh full budget.
        let another = rate.reserve(&cancel);
        tokio::pin!(another);
        assert!(
            tokio::time::timeout(Duration::from_millis(1), &mut another)
                .await
                .is_err()
        );
        cancel.cancel();
        assert_eq!(another.await.err().unwrap().code, ErrorCode::Cancelled);
    }

    #[tokio::test(start_paused = true)]
    async fn already_waiting_reservation_reconciles_suspend_within_one_second() {
        let clock = Clock::for_test();
        let (rate, _) = limited(&clock);
        let cancel = CancellationToken::new();
        let pending = rate.reserve(&cancel);
        tokio::pin!(pending);
        assert!(
            tokio::time::timeout(Duration::from_millis(1), &mut pending)
                .await
                .is_err()
        );
        let resumed = tokio::time::Instant::now();
        clock.advance_wall(Duration::from_secs(120));
        let _probe = tokio::time::timeout(Duration::from_secs(1), pending)
            .await
            .unwrap()
            .unwrap();
        assert!(resumed.elapsed() <= Duration::from_secs(1));
    }

    #[tokio::test(start_paused = true)]
    async fn backward_wall_clock_and_repeated_headers_do_not_extend_reset_wait() {
        let clock = Clock::for_test();
        let (rate, headers) = limited(&clock);
        tokio::time::advance(Duration::from_secs(30)).await;
        clock.rewind_wall(Duration::from_secs(7200));
        rate.observe(&headers, true);
        let cancel = CancellationToken::new();
        let pending = rate.reserve(&cancel);
        tokio::pin!(pending);
        let start = tokio::time::Instant::now();
        assert!(
            tokio::time::timeout(Duration::from_secs(29), &mut pending)
                .await
                .is_err()
        );
        let _probe = tokio::time::timeout(Duration::from_secs(1), pending)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(start.elapsed(), Duration::from_secs(30));
    }
}

#[cfg(test)]
mod background_tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn background_yields_low_budget_and_capacity_then_can_probe_after_sleep() {
        let clock = Clock::for_test();
        let rate = Arc::new(RateLimiter {
            clock: clock.clone(),
            ..Default::default()
        });
        let cancel = CancellationToken::new();
        let mut headers = HeaderMap::new();
        headers.insert("ratelimit-limit", "100".parse().unwrap());
        headers.insert("ratelimit-remaining", "10".parse().unwrap());
        headers.insert("ratelimit-reset", "1800000060".parse().unwrap());
        rate.observe(&headers, false);
        assert_eq!(
            rate.reserve_with_priority(&cancel, true)
                .await
                .err()
                .unwrap()
                .code,
            ErrorCode::RateLimited
        );
        let foreground = rate.reserve(&cancel).await.unwrap();
        assert_eq!(rate.snapshot().remaining, Some(9));
        drop(foreground);
        clock.advance_wall(Duration::from_secs(61));
        let background = rate.reserve_with_priority(&cancel, true).await.unwrap();
        assert!(rate.snapshot().remaining.is_none());
        assert_eq!(
            rate.reserve_with_priority(&cancel, true)
                .await
                .err()
                .unwrap()
                .code,
            ErrorCode::RateLimited
        );
        headers.insert("ratelimit-reset", "1800000120".parse().unwrap());
        headers.insert("ratelimit-remaining", "99".parse().unwrap());
        rate.observe(&headers, false);
        let first = rate.reserve(&cancel).await.unwrap();
        let second = rate.reserve(&cancel).await.unwrap();
        assert_eq!(
            rate.reserve_with_priority(&cancel, true)
                .await
                .err()
                .unwrap()
                .code,
            ErrorCode::RateLimited
        );
        let third = rate.reserve(&cancel).await.unwrap();
        assert_eq!(rate.snapshot().in_flight, 4);
        drop((background, first, second, third));
        rate.observe(&HeaderMap::new(), true);
        assert_eq!(
            rate.reserve_with_priority(&cancel, true)
                .await
                .err()
                .unwrap()
                .code,
            ErrorCode::RateLimited
        );
        tokio::time::advance(Duration::from_secs(60)).await;
        assert!(rate.reserve_with_priority(&cancel, true).await.is_ok());
    }
}
