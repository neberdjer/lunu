pub fn escape(value: &str) -> String {
	value
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn escape_neutralizes_markup() {
		assert_eq!(escape("<b>&\"'"), "&lt;b&gt;&amp;&quot;&#39;");
	}
}
