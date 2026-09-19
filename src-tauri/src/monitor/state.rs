use crate::helix::models::Stream;
use std::collections::{HashMap, HashSet};

const MAX_SEEN: usize = 16_000;
#[derive(Default)]
pub(super) struct Transitions {
    baseline: bool,
    // History is independent of current presence. The same Twitch session can
    // disappear for multiple complete polls and still must not notify twice.
    seen: HashSet<(String, String)>,
    missing: HashMap<String, u8>,
}
impl Transitions {
    pub fn ready(&self) -> bool {
        self.baseline
    }
    pub fn rebaseline(&mut self) {
        self.baseline = false;
    }
    pub fn live_count(&self) -> u32 {
        self.missing.len() as u32
    }
    pub fn accept(&mut self, streams: Vec<Stream>) -> Vec<Stream> {
        if self.seen.len() + streams.len() > MAX_SEEN {
            self.seen.clear();
            self.rebaseline();
        }
        if !self.baseline {
            self.missing.clear();
        }
        let present: HashSet<_> = streams.iter().map(|s| s.user_id.clone()).collect();
        self.missing.retain(|id, missing| {
            if present.contains(id) {
                *missing = 0;
            } else {
                *missing += 1;
            }
            *missing < 2
        });
        let mut events = Vec::new();
        for stream in streams {
            self.missing.insert(stream.user_id.clone(), 0);
            if self
                .seen
                .insert((stream.user_id.clone(), stream.id.clone()))
                && self.baseline
            {
                events.push(stream);
            }
        }
        self.baseline = true;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stream(id: &str, user: &str) -> Stream {
        let mut stream: Stream = serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../tests/fixtures/helix/stream.json"
            ))
            .unwrap()["data"][0]
                .clone(),
        )
        .unwrap();
        stream.id = id.into();
        stream.user_id = user.into();
        stream
    }
    #[test]
    fn baseline_duplicates_missing_offline_reappearance_and_restart() {
        let mut state = Transitions::default();
        assert!(
            state
                .accept(vec![stream("100", "1"), stream("100", "1")])
                .is_empty()
        );
        assert_eq!(state.live_count(), 1);
        assert_eq!(
            state
                .accept(vec![
                    stream("100", "1"),
                    stream("200", "2"),
                    stream("200", "2")
                ])
                .len(),
            1
        );
        assert!(
            state
                .accept(vec![stream("100", "1"), stream("200", "2")])
                .is_empty()
        );
        assert!(state.accept(vec![]).is_empty());
        assert_eq!(state.live_count(), 2); // One absence is uncertain.
        state.accept(vec![]);
        assert_eq!(state.live_count(), 0); // Two complete polls establish absence.
        assert!(state.accept(vec![stream("100", "1")]).is_empty());
        assert_eq!(state.accept(vec![stream("101", "1")]).len(), 1);
    }
    #[test]
    fn pause_sleep_and_recovery_suppress_unobserved_transitions() {
        for _reason in ["pause", "sleep", "network", "partial", "429"] {
            let mut state = Transitions::default();
            state.accept(vec![stream("100", "1")]);
            state.rebaseline();
            assert_eq!(state.live_count(), 1);
            assert!(
                state
                    .accept(vec![stream("100", "1"), stream("200", "2")])
                    .is_empty()
            );
            assert_eq!(state.accept(vec![stream("300", "3")]).len(), 1);
        }
    }
    #[test]
    fn history_capacity_rebaselines_without_replaying_old_events() {
        let mut state = Transitions {
            seen: (0..MAX_SEEN).map(|n| ("1".into(), n.to_string())).collect(),
            baseline: true,
            ..Default::default()
        };
        assert!(state.accept(vec![stream("90000", "1")]).is_empty());
        assert_eq!(state.seen.len(), 1);
    }
}
