//! Rate limiting for the sign-in endpoint.
//!
//! Guessing a password is cheap for an attacker and expensive for us — argon2
//! is deliberately slow — so `/api/sign_in` gets a per-address budget. Nothing
//! else on `/api` is touched.
//!
//! The counters live in memory: a single process, no dependency, no database
//! write on the busiest unauthenticated path in the app. The trade is that they
//! reset when the process restarts and are not shared between replicas, which
//! is the right trade for one container and the wrong one for a fleet.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{CONTENT_TYPE, RETRY_AFTER};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

/// The one endpoint this guards.
const SIGN_IN_PATH: &str = "/api/sign_in";

/// `server_fn` marks every failed call with this header. It is the only honest
/// success signal we have: a plain form post — an `ActionForm` submitted before
/// hydration — answers `302` whether the password was right or not, so the
/// status code alone cannot tell a failure from a success.
const SERVER_FN_ERROR: &str = "serverfnerror";

/// Failed sign-ins one address may make per [`WINDOW`] before it is refused.
pub const MAX_FAILURES: u32 = 5;
pub const WINDOW: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, Debug)]
struct Attempts {
    failures: u32,
    window_start: Instant,
}

/// Per-address failure counters, shared with the middleware through axum state.
#[derive(Clone, Default)]
pub struct LoginLimiter {
    inner: Arc<Mutex<HashMap<IpAddr, Attempts>>>,
}

impl LoginLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// A poisoned lock means some other thread panicked mid-update; the counters
    /// are still readable and refusing every sign-in afterwards would be worse.
    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<IpAddr, Attempts>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// How long this address has to wait, if it is out of budget.
    ///
    /// `now` is a parameter rather than read inside so the whole policy can be
    /// tested without sleeping.
    pub fn retry_after(&self, ip: IpAddr, now: Instant) -> Option<Duration> {
        let map = self.map();
        let attempts = map.get(&ip)?;
        let elapsed = now.saturating_duration_since(attempts.window_start);
        (attempts.failures >= MAX_FAILURES && elapsed < WINDOW).then(|| WINDOW - elapsed)
    }

    pub fn record_failure(&self, ip: IpAddr, now: Instant) {
        let mut map = self.map();
        // An attacker rotating source addresses would otherwise grow this map
        // without bound, which is its own denial of service.
        map.retain(|_, a| now.saturating_duration_since(a.window_start) < WINDOW);

        let entry = map.entry(ip).or_insert(Attempts {
            failures: 0,
            window_start: now,
        });
        if now.saturating_duration_since(entry.window_start) >= WINDOW {
            *entry = Attempts {
                failures: 0,
                window_start: now,
            };
        }
        entry.failures = entry.failures.saturating_add(1);
    }

    /// Signing in successfully clears the budget: a shopper who mistypes twice
    /// and then gets it right should not be carrying that for a minute.
    pub fn record_success(&self, ip: IpAddr) {
        self.map().remove(&ip);
    }

    /// How many addresses are being tracked. Exists for the pruning test.
    pub fn tracked(&self) -> usize {
        self.map().len()
    }
}

/// Which address to charge for this attempt.
///
/// `X-Forwarded-For` is **opt-in**. Behind a proxy the peer address is the
/// proxy's and everyone shares one budget, but trusting the header by default
/// is worse: anyone could invent a new address per request and never be limited
/// at all. Same shape as `APP_SECURE_COOKIES` in [`crate::auth`].
fn client_ip(req: &Request, peer: SocketAddr) -> IpAddr {
    if std::env::var("APP_TRUST_FORWARDED_FOR").as_deref() == Ok("1") {
        if let Some(forwarded) = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(',').next())
            .map(str::trim)
            .and_then(|value| value.parse::<IpAddr>().ok())
        {
            return forwarded;
        }
    }
    peer.ip()
}

pub async fn throttle_sign_in(
    State(limiter): State<LoginLimiter>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    // This layer wraps the whole router (see the note in `main.rs`), so every
    // request passes through here and all but one leaves immediately.
    if req.uri().path() != SIGN_IN_PATH {
        return next.run(req).await;
    }

    let ip = client_ip(&req, peer);
    let now = Instant::now();

    if let Some(wait) = limiter.retry_after(ip, now) {
        return too_many(wait);
    }

    let response = next.run(req).await;
    if response.headers().contains_key(SERVER_FN_ERROR) {
        limiter.record_failure(ip, now);
    } else {
        limiter.record_success(ip);
    }
    response
}

/// The refusal.
///
/// The body has to be in `server_fn`'s `Type|message` wire format. The client
/// treats any 400–599 as an error and decodes the body with that codec, so a
/// plain `"Too many attempts"` would surface in the sign-in form as
/// *`Invalid format: missing delimiter`* rather than as the message.
fn too_many(wait: Duration) -> Response {
    let secs = wait.as_secs().max(1);
    let body = format!("ServerError|Too many sign-in attempts. Try again in {secs} seconds.");

    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    if let Ok(value) = HeaderValue::from_str(&secs.to_string()) {
        headers.insert(RETRY_AFTER, value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(last: u8) -> IpAddr {
        IpAddr::from([203, 0, 113, last])
    }

    #[test]
    fn a_quiet_address_is_never_refused() {
        let limiter = LoginLimiter::new();
        assert_eq!(limiter.retry_after(ip(1), Instant::now()), None);
    }

    #[test]
    fn the_budget_runs_out_only_on_the_last_failure() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        for _ in 0..MAX_FAILURES - 1 {
            limiter.record_failure(ip(1), now);
            assert_eq!(limiter.retry_after(ip(1), now), None);
        }
        limiter.record_failure(ip(1), now);
        assert!(limiter.retry_after(ip(1), now).is_some());
    }

    #[test]
    fn one_address_running_out_does_not_lock_out_another() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        for _ in 0..MAX_FAILURES {
            limiter.record_failure(ip(1), now);
        }
        assert!(limiter.retry_after(ip(1), now).is_some());
        assert_eq!(limiter.retry_after(ip(2), now), None);
    }

    #[test]
    fn the_budget_returns_when_the_window_passes() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        for _ in 0..MAX_FAILURES {
            limiter.record_failure(ip(1), now);
        }
        assert!(limiter.retry_after(ip(1), now).is_some());
        assert_eq!(limiter.retry_after(ip(1), now + WINDOW), None);
    }

    #[test]
    fn the_wait_shrinks_as_the_window_elapses() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        for _ in 0..MAX_FAILURES {
            limiter.record_failure(ip(1), now);
        }
        let early = limiter.retry_after(ip(1), now).unwrap();
        let later = limiter
            .retry_after(ip(1), now + Duration::from_secs(30))
            .unwrap();
        assert!(later < early, "{later:?} should be less than {early:?}");
    }

    #[test]
    fn signing_in_successfully_clears_the_budget() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        for _ in 0..MAX_FAILURES {
            limiter.record_failure(ip(1), now);
        }
        assert!(limiter.retry_after(ip(1), now).is_some());
        limiter.record_success(ip(1));
        assert_eq!(limiter.retry_after(ip(1), now), None);
    }

    #[test]
    fn stale_addresses_are_pruned_rather_than_accumulating() {
        let limiter = LoginLimiter::new();
        let start = Instant::now();
        // An attacker walking through source addresses.
        for n in 0..50 {
            limiter.record_failure(ip(n), start);
        }
        assert_eq!(limiter.tracked(), 50);
        // One later attempt, and everything older than the window is dropped.
        limiter.record_failure(ip(200), start + WINDOW + Duration::from_secs(1));
        assert_eq!(limiter.tracked(), 1);
    }
}
