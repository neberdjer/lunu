pub struct LogControl {
	setter: Box<dyn Fn(&str) -> bool + Send + Sync>,
	current: std::sync::RwLock<String>,
}

impl LogControl {
	pub fn new(initial: &str, setter: Box<dyn Fn(&str) -> bool + Send + Sync>) -> Self {
		Self {
			setter,
			current: std::sync::RwLock::new(initial.to_string()),
		}
	}

	pub fn set(&self, level: &str) -> bool {
		let applied = (self.setter)(level);
		if applied {
			*self.current.write().expect("log level lock") = level.to_string();
		}
		applied
	}

	pub fn current(&self) -> String {
		self.current.read().expect("log level lock").clone()
	}
}
