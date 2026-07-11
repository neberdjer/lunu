use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct Window {
	start: Instant,
	count: u32,
}

struct State {
	windows: HashMap<String, Window>,
	last_sweep: Instant,
}

pub struct RateLimiter {
	max_attempts: u32,
	window: Duration,
	state: Mutex<State>,
}

impl RateLimiter {
	pub fn new(max_attempts: u32, window: Duration) -> Self {
		Self {
			max_attempts,
			window,
			state: Mutex::new(State {
				windows: HashMap::new(),
				last_sweep: Instant::now(),
			}),
		}
	}

	pub fn check(&self, key: &str) -> bool {
		self.check_at(key, Instant::now())
	}

	fn check_at(&self, key: &str, now: Instant) -> bool {
		let mut state = self.state.lock().unwrap();

		if now.duration_since(state.last_sweep) >= self.window {
			state
				.windows
				.retain(|_, window| now.duration_since(window.start) < self.window);
			state.last_sweep = now;
		}

		let window = state.windows.entry(key.to_string()).or_insert(Window {
			start: now,
			count: 0,
		});
		if now.duration_since(window.start) >= self.window {
			window.start = now;
			window.count = 0;
		}
		window.count += 1;
		window.count <= self.max_attempts
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn allows_up_to_the_limit_then_blocks() {
		let limiter = RateLimiter::new(3, Duration::from_secs(60));
		let now = Instant::now();

		assert!(limiter.check_at("ip", now));
		assert!(limiter.check_at("ip", now));
		assert!(limiter.check_at("ip", now));
		assert!(!limiter.check_at("ip", now));
		assert!(!limiter.check_at("ip", now));
	}

	#[test]
	fn window_resets_after_expiry() {
		let limiter = RateLimiter::new(2, Duration::from_secs(60));
		let now = Instant::now();

		assert!(limiter.check_at("ip", now));
		assert!(limiter.check_at("ip", now));
		assert!(!limiter.check_at("ip", now));

		let later = now + Duration::from_secs(61);
		assert!(limiter.check_at("ip", later));
	}

	#[test]
	fn separate_keys_have_separate_budgets() {
		let limiter = RateLimiter::new(1, Duration::from_secs(60));
		let now = Instant::now();

		assert!(limiter.check_at("a", now));
		assert!(!limiter.check_at("a", now));
		assert!(limiter.check_at("b", now));
	}
}
