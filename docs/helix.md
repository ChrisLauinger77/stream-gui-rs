# Rust Helix client contracts

The implementation follows the [Twitch API reference](https://dev.twitch.tv/docs/api/reference/) and [API guide](https://dev.twitch.tv/docs/api/guide/), checked on 2026-09-18. The old Streamlink Twitch GUI adapter was consulted for repeated query parameters, ID batching and URL size limits. Its Ember models and token ownership were not copied; stream `id` remains the actual Twitch stream ID, distinct from `user_id`.

## Entry points and ownership

`Services` constructs one `TwitchHttp` pool, one `AuthService`, and one `HelixClient`. Foreground browsing and the Rust monitor share these exact instances. They do not create independent pools, rate budgets or caches.

| Rust method | Helix path | Cache class |
| --- | --- | --- |
| `users`, `users_by_ids`, `account` | `users` | Metadata |
| `channels`, `channels_by_ids` | `channels` | Metadata |
| `followed_channels` | `channels/followed` | Live |
| `streams`, `streams_by_user_ids` | `streams` | Live |
| `followed_streams` | `streams/followed` | Live |
| `games`, `games_by_ids` | `games` | Categories |
| `top_games` | `games/top` | Live |
| `search_categories` | `search/categories` | Categories |
| `search_channels` | `search/channels` | Live |
| `teams` | `teams` | Metadata |
| `channel_teams` | `teams/channel` | Metadata |

Models use current wire fields, opaque string IDs, nullable team images and optional deprecated fields. Unknown fields are ignored. User email is deliberately omitted from the model/cache. `Account` is a separate safe IPC DTO containing only ID, login, display name and a validated optional profile image URL. Teams' `info` may contain HTML; future UI must treat it as untrusted content.

Every request obtains an internal auth lease. Followed endpoints derive `user_id` from that lease and require `user:read:follows`. Frontend input cannot select an arbitrary URL, access token, or account authorization. Endpoint methods accept a `CancellationToken`; rate waits, requests, bodies and retry delays are cancellable. Dropping a caller does not interrupt a refresh whose result must still be persisted. Logout invalidates leases and prevents their responses from populating usable cache entries.

## HTTP, errors and rate budget

Helix GET requests set sensitive `Authorization: Bearer` and `Client-Id` headers. OAuth uses the same reqwest connection pool and its protocol-specific forms/validation header. Production base URLs are fixed; loopback overrides exist only under `cfg(test)`. Redirects are disabled. Connection timeout is 5 seconds, total request timeout 15 seconds; bodies are capped at 4 MiB (64 KiB for OAuth).

Errors distinguish unauthenticated, unauthorized/insufficient scope, rate limited, network unavailable, timeout, invalid response, Twitch server failure, cancelled, and invalid input. Messages are fixed safe text, never provider bodies or request URLs. Auth additionally distinguishes denied/expired grants, invalid tokens, configuration and storage failures.

A GET gets at most one retry after 250 ms for network/timeouts or HTTP 500/502/503/504. Other 4xx, 429, malformed responses and non-transient server statuses are not blindly retried. OAuth exchanges are never automatically retried. A Helix 401 instead invokes coordinated refresh once; a second 401 clears authentication. These are separate finite bounds, not an unlimited retry loop. A logical operation retains its original session across refresh, lease reacquisition and batch chunks. Logout or account replacement terminates that operation; it cannot retry with the replacement account. Already-dispatched rotation still finishes safely under the auth owner.

The shared coordinator tracks `Ratelimit-Limit`, `Ratelimit-Remaining` and Unix `Ratelimit-Reset`. A reservation accounts for requests already in flight. Out-of-order same-window responses cannot increase the local budget; older-window responses are ignored. At most four requests run concurrently with known budget, and one with unknown budget. When the window expires, one request probes the new budget. This deliberately favors conservative under-use over excess requests.

HTTP 429 immediately reports `rate_limited` and blocks subsequent reservations until reset (or bounded Retry-After/fallback delay when headers are absent). Waiters sleep or await notification/cancellation; there is no busy-wait or independent frontend retry loop. A long rate wait has no arbitrary overall timeout and remains cancellable. Callers can inspect `rate_status()`. Reset deadlines use the shared suspend-aware clock helper; already-waiting reservations reconcile at least once per second of uptime. Unix reset calculations use its non-rewinding wall-time projection, so a backward clock adjustment cannot turn an existing short wait into hours. An elapsed reset permits one probe until new headers confirm the budget.

## Pagination and batches

`Page<T>` retains `data`, optional total and opaque cursor. `PageRequest` validates page size 1–100, cursor length at most 2,048 bytes, and one cursor direction; unsupported backwards pagination fails before sending. `Page::next` returns a request description, never fetches it automatically. Dynamic stream rankings can move between pages; callers should deduplicate by the **stream ID** when assembling a live paginated view.

ID batch helpers accept at most 1,000 input IDs, deduplicate them, and split into chunks of at most 100 plus a conservative encoded-query limit. Endpoint methods also validate their limits and cap encoded queries at 7,000 bytes. Batches execute sequentially through the shared coordinator; they do not fetch unlimited pages.

`BatchResult` separates successful data, missing IDs (successful complete response only), stale IDs, incomplete IDs (a cursor indicates more results), and failed ID chunks with categorized errors. `complete()` is false for failures or incomplete pagination. Cancellation, auth failures and rate limiting stop further dispatch while explicitly marking remaining chunks failed. Other per-chunk failures can retain successes from subsequent bounded chunks. Live results omitted from a complete response can represent offline users; a failed chunk must not be interpreted as offline.

## Cache

The in-memory Rust cache stores typed public model data only, after dropping unknown response fields. Keys include session identity, authenticated user, endpoint and encoded query, never tokens. Valid authorization is required even for a cache hit. A new login cannot reuse a previous session's entries. Logout clears the cache through the command adapter; lease cancellation also protects direct backend callers from using old entries, which remain bounded until eviction.

| Class | TTL | Intended data |
| --- | --- | --- |
| Live | 15 seconds | Streams, follows, channel search, category rankings |
| Metadata | 5 minutes | Users, channels, teams |
| Categories | 1 hour | Games and category search |

Capacity is 128 entries, 8 MiB of cached payloads total, and 2 MiB per entry, with oldest-inserted eviction. Oversized responses can be returned within the HTTP body cap but are not cached. Cache keys/entry overhead have separate fixed bounds from query/entry limits. Expired entries may remain within capacity for explicit stale reads.

`Fresh` returns an unexpired hit or fetches; `Refresh` bypasses lookup; `AllowStale` may return an expired hit with explicit `Freshness::Stale` and age. Cache age counts time spent asleep. Observed age never decreases after a backward clock adjustment. No silent stale-on-error or background revalidation occurs. Invalidation is explicit by class or whole cache. Future consumers can share metadata and choose refresh/stale behavior without building another authoritative frontend cache.

## Background followed-live scans

`monitor_followed` pins the original authenticated request session and reads 30 records per page using the same query keys as Following. The monitor supplies an explicit cache policy: ordinary scans use `Fresh`; every new quiet baseline uses `Refresh` on every page. Startup, Resume, suspend recovery and other baseline resets therefore fetch observations after their boundary, even when pre-boundary pages remain warm. This bypasses lookup without clearing unrelated cache entries; successful pages still populate the shared cache. It waits 100 ms between pages, caps a scan at 100 pages/3,000 input records, rejects repeated cursors and malformed live identities, and deduplicates by actual stream ID. A scan must reach the end; capacity, timeout, cancellation, rate limits or partial failure never establish a complete baseline or offline status. The service deadline is 45 seconds.

Background reservations use the existing coordinator without entering its foreground wait queue. They defer when capacity is constrained (three or more requests in flight) or remaining budget is at/below 10% of the limit, clamped to 1–50 requests. Unknown/reset windows retain the original single-probe rule. A deferred background scan reports rate limiting and uses monitor backoff; interactive callers retain the coordinator's normal cancellable wait behavior. This is conservative priority, not a separate token budget or scheduler.

Successful monitor pages can serve foreground Following; simultaneous cache misses are not coalesced, but share the same concurrency/rate bounds. See [monitor ownership and recovery](architecture.md#background-monitor-and-native-lifecycle).
