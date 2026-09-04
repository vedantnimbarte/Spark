//! Login attempt limiting.
//!
//! Argon2 makes each guess expensive but not impossible; this bounds how many
//! an attacker gets. Keyed on the email being attempted rather than the client
//! address, because the address is trivially varied and a proxy hides it
//! anyway — the cost of that choice is that an attacker can lock a known
//! account out of *fast* logins, not out of the system, since the window is
//! short.
//!
//! ponytail: in-process state, so it resets on restart and does not span
//! replicas. That matches a single-replica control plane; move it to Postgres
//! if the control plane is ever scaled out.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

pub const MAX_ATTEMPTS: u32 = 10;
pub const WINDOW: Duration = Duration::from_secs(15 * 60);

#[derive(Debug)]
struct Attempts {
    count: u32,
    first_seen: Instant,
}

#[derive(Default)]
pub struct RateLimiter {
    entries: Mutex<HashMap<String, Attempts>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an attempt and reports whether it is allowed to proceed.
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let Ok(mut entries) = self.entries.lock() else {
            // A poisoned lock means another thread panicked mid-update. Failing
            // open is the right call: refusing every login would turn a bug
            // into a lockout.
            return true;
        };

        // Bounded cleanup, so the map cannot grow without limit from a stream
        // of distinct emails.
        if entries.len() > 10_000 {
            entries.retain(|_, a| now.duration_since(a.first_seen) < WINDOW);
        }

        let entry = entries.entry(key.to_ascii_lowercase()).or_insert(Attempts {
            count: 0,
            first_seen: now,
        });

        if now.duration_since(entry.first_seen) >= WINDOW {
            entry.count = 0;
            entry.first_seen = now;
        }

        entry.count += 1;
        entry.count <= MAX_ATTEMPTS
    }

    /// Clears the counter after a successful login, so ordinary use never
    /// accumulates toward the limit.
    pub fn reset(&self, key: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(&key.to_ascii_lowercase());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_the_limit_then_refuses() {
        let limiter = RateLimiter::new();
        for i in 1..=MAX_ATTEMPTS {
            assert!(limiter.check("user@example.com"), "attempt {i} should pass");
        }
        assert!(!limiter.check("user@example.com"), "the next one must not");
    }

    #[test]
    fn a_successful_login_clears_the_counter() {
        let limiter = RateLimiter::new();
        for _ in 0..MAX_ATTEMPTS {
            limiter.check("user@example.com");
        }
        limiter.reset("user@example.com");
        assert!(limiter.check("user@example.com"));
    }

    #[test]
    fn accounts_are_limited_independently_and_case_insensitively() {
        let limiter = RateLimiter::new();
        for _ in 0..=MAX_ATTEMPTS {
            limiter.check("victim@example.com");
        }
        assert!(!limiter.check("VICTIM@example.com"), "same account");
        assert!(limiter.check("someone@example.com"), "a different account");
    }
}
