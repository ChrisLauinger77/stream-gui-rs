//! A deliberate bounded scan, using the same page keys as foreground Following.
use super::*;
use crate::config::validate_broadcaster_id;

pub(crate) const MAX_MONITOR_PAGES: usize = 100;

impl<A: TwitchApi + 'static> HelixClient<A> {
    pub(crate) async fn monitor_followed(
        &self,
        session_id: u64,
        policy: CachePolicy,
        cancel: &CancellationToken,
    ) -> Result<Vec<Stream>> {
        let lease = self.auth.lease_for_session(session_id).await?;
        let mut session = Some(RequestSession {
            id: session_id,
            user_id: lease.user_id,
            cancel: lease.cancel.clone(),
            background: true,
        });
        let mut request = PageRequest::first(30)?;
        let mut cursors = HashSet::new();
        let mut ids = HashSet::new();
        let mut streams = Vec::new();
        for index in 0..MAX_MONITOR_PAGES {
            if index > 0 {
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return Err(error(ErrorCode::Cancelled)),
                    _ = lease.cancel.cancelled() => return Err(error(ErrorCode::Unauthenticated)),
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {},
                }
            }
            let page = self
                .get_bound::<Stream>(
                    "streams/followed",
                    request.query(false)?,
                    true,
                    CacheClass::Live,
                    policy,
                    cancel,
                    &mut session,
                )
                .await?
                .value;
            if page.data.len() > 30 {
                return Err(error(ErrorCode::InvalidResponse));
            }
            let next = page.next(30)?;
            for stream in page.data {
                validate_broadcaster_id(&stream.user_id)
                    .map_err(|_| error(ErrorCode::InvalidResponse))?;
                validate_broadcaster_id(&stream.id)
                    .map_err(|_| error(ErrorCode::InvalidResponse))?;
                if stream.stream_type != "live" {
                    return Err(error(ErrorCode::InvalidResponse));
                }
                if ids.insert(stream.id.clone()) {
                    streams.push(stream);
                }
            }
            if cancel.is_cancelled() || lease.cancel.is_cancelled() {
                return Err(error(ErrorCode::Cancelled));
            }
            match next {
                None => return Ok(streams),
                Some(next) => {
                    if !cursors.insert(page.pagination.cursor) {
                        return Err(error(ErrorCode::Incomplete));
                    }
                    request = next;
                }
            }
        }
        // Never publish the prefix of a scan as a complete/offline baseline.
        Err(error(ErrorCode::Incomplete))
    }
}
