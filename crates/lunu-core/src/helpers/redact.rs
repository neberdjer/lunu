const MASK: &str = "[redacted]";
const SENSITIVE_NEEDLES: &[&str] = &[
	"apikey=",
	"api_key=",
	"token=",
	"password=",
	"secret=",
	"bearer ",
];

pub fn redact(message: &str) -> String {
	let lower = message.to_ascii_lowercase();
	let has_needle = SENSITIVE_NEEDLES
		.iter()
		.any(|needle| lower.contains(needle));
	if !has_needle && !message.contains("://") {
		return message.to_string();
	}

	let mut redacted = mask_userinfo(message);
	for needle in SENSITIVE_NEEDLES {
		redacted = mask_after(&redacted, needle);
	}
	redacted
}

fn value_end(text: &str) -> usize {
	text.find(|c: char| c == '&' || c == '"' || c == '\'' || c.is_whitespace())
		.unwrap_or(text.len())
}

fn mask_after(text: &str, needle: &str) -> String {
	let lower = text.to_ascii_lowercase();
	let mut result = String::with_capacity(text.len());
	let mut cursor = 0;

	while let Some(found) = lower[cursor..].find(needle) {
		let start = cursor + found + needle.len();
		result.push_str(&text[cursor..start]);
		result.push_str(MASK);
		cursor = start + value_end(&text[start..]);
	}
	result.push_str(&text[cursor..]);
	result
}

fn mask_userinfo(text: &str) -> String {
	let mut result = String::with_capacity(text.len());
	let mut cursor = 0;

	while let Some(found) = text[cursor..].find("://") {
		let start = cursor + found + 3;
		result.push_str(&text[cursor..start]);
		let rest = &text[start..];
		let authority_end = rest
			.find(|c: char| c == '/' || c.is_whitespace())
			.unwrap_or(rest.len());
		match rest[..authority_end].rfind('@') {
			Some(at) => {
				result.push_str(MASK);
				cursor = start + at;
			}
			None => cursor = start,
		}
	}
	result.push_str(&text[cursor..]);
	result
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sensitive_query_params_are_masked() {
		assert_eq!(
			redact("call failed: https://sab:8080/api?mode=queue&apikey=abc123&output=json"),
			"call failed: https://sab:8080/api?mode=queue&apikey=[redacted]&output=json"
		);
		assert_eq!(redact("Token=xyz at end"), "Token=[redacted] at end");
	}

	#[test]
	fn url_userinfo_is_masked() {
		assert_eq!(
			redact("get http://admin:hunter2@qbit:8080/api/v2 failed"),
			"get http://[redacted]@qbit:8080/api/v2 failed"
		);
		assert_eq!(
			redact("plain http://qbit:8080/api stays"),
			"plain http://qbit:8080/api stays"
		);
	}

	#[test]
	fn bearer_tokens_are_masked() {
		assert_eq!(
			redact("header Authorization: Bearer eyJhbGci.rest"),
			"header Authorization: Bearer [redacted]"
		);
	}

	#[test]
	fn ordinary_messages_pass_through() {
		let message = "imported 3 items into /library/Author/Book";
		assert_eq!(redact(message), message);
	}
}
