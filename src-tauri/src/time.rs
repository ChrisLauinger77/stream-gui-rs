//! Elapsed time for freshness/deadlines that must include system sleep.
//! Short waits still use Tokio timers; callers reconcile these deadlines when
//! they wake. Backward wall-clock steps never rewind observed elapsed time.
use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use tokio::time::Instant;

#[derive(Clone)]
pub(crate) struct Clock(Arc<Mutex<State>>);
struct State {
    origin: SystemTime,
    wall: SystemTime,
    monotonic: Instant,
    elapsed: Duration,
    #[cfg(test)]
    test_wall: Option<SystemTime>,
}
impl Default for Clock {
    fn default() -> Self {
        Self::new(SystemTime::now())
    }
}
impl Clock {
    fn new(wall: SystemTime) -> Self {
        Self(Arc::new(Mutex::new(State {
            origin: wall,
            wall,
            monotonic: Instant::now(),
            elapsed: Duration::ZERO,
            #[cfg(test)]
            test_wall: None,
        })))
    }
    fn sample(state: &mut State) {
        let monotonic = Instant::now();
        #[cfg(not(test))]
        let wall = SystemTime::now();
        #[cfg(test)]
        let wall = state.test_wall.unwrap_or_else(SystemTime::now);
        let elapsed = monotonic
            .saturating_duration_since(state.monotonic)
            .max(wall.duration_since(state.wall).unwrap_or_default());
        state.elapsed = state.elapsed.saturating_add(elapsed);
        state.monotonic = monotonic;
        state.wall = wall;
    }
    pub(crate) fn now(&self) -> Duration {
        let mut state = self.0.lock().expect("clock mutex poisoned");
        Self::sample(&mut state);
        state.elapsed
    }
    // Reconcile Unix reset headers against the same non-rewinding timeline.
    // A backward clock step must not turn a minute's wait into hours.
    pub(crate) fn unix_now(&self) -> SystemTime {
        let mut state = self.0.lock().expect("clock mutex poisoned");
        Self::sample(&mut state);
        state.origin + state.elapsed
    }
    #[cfg(test)]
    pub(crate) fn for_test() -> Self {
        let wall = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let clock = Self::new(wall);
        clock.0.lock().unwrap().test_wall = Some(wall);
        clock
    }
    #[cfg(test)]
    pub(crate) fn advance_wall(&self, duration: Duration) {
        let mut state = self.0.lock().unwrap();
        *state.test_wall.as_mut().expect("test clock") += duration;
    }
    #[cfg(test)]
    pub(crate) fn rewind_wall(&self, duration: Duration) {
        let mut state = self.0.lock().unwrap();
        *state.test_wall.as_mut().expect("test clock") -= duration;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn sleep_counts_once_when_wall_and_monotonic_both_advance() {
        let clock = Clock::for_test();
        let minute = Duration::from_secs(60);
        clock.advance_wall(minute);
        tokio::time::advance(minute).await;
        assert_eq!(clock.now(), minute);
        clock.advance_wall(minute);
        assert_eq!(clock.now(), minute * 2);
    }

    #[tokio::test(start_paused = true)]
    async fn backward_wall_step_never_rewinds_elapsed_or_stops_uptime_progress() {
        let clock = Clock::for_test();
        let origin = clock.unix_now();
        clock.advance_wall(Duration::from_secs(3600));
        assert_eq!(clock.now(), Duration::from_secs(3600));
        clock.rewind_wall(Duration::from_secs(7200));
        assert_eq!(clock.now(), Duration::from_secs(3600));
        tokio::time::advance(Duration::from_secs(60)).await;
        assert_eq!(clock.now(), Duration::from_secs(3660));
        assert_eq!(clock.unix_now(), origin + Duration::from_secs(3660));
    }
}
