const ENTITIES: &[(&str, &str)] = &[
	("&nbsp;", " "),
	("&lt;", "<"),
	("&gt;", ">"),
	("&quot;", "\""),
	("&#39;", "'"),
	("&apos;", "'"),
	("&amp;", "&"),
];

pub(super) fn strip_html(value: Option<String>) -> Option<String> {
	let value = value?;
	if !value.contains('<') && !value.contains('&') {
		return non_empty(value.trim().to_string());
	}

	let mut text = String::with_capacity(value.len());
	let mut in_tag = false;
	for ch in value.chars() {
		match ch {
			'<' => in_tag = true,
			'>' if in_tag => {
				in_tag = false;
				text.push(' ');
			}
			_ if !in_tag => text.push(ch),
			_ => {}
		}
	}

	for (entity, replacement) in ENTITIES {
		if text.contains(entity) {
			text = text.replace(entity, replacement);
		}
	}

	non_empty(text.split_whitespace().collect::<Vec<_>>().join(" "))
}

pub(super) fn normalize_date(value: Option<String>) -> Option<String> {
	let value = value?;
	let trimmed = value.trim();
	let date = trimmed.split('T').next().unwrap_or(trimmed);
	if is_iso_date(date) {
		return Some(date.to_string());
	}
	non_empty(trimmed.to_string())
}

fn is_iso_date(value: &str) -> bool {
	value.len() == 10
		&& value.bytes().enumerate().all(|(index, byte)| match index {
			4 | 7 => byte == b'-',
			_ => byte.is_ascii_digit(),
		})
}

fn non_empty(value: String) -> Option<String> {
	(!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn strip(value: &str) -> Option<String> {
		strip_html(Some(value.to_string()))
	}

	#[test]
	fn plain_text_survives_untouched() {
		assert_eq!(
			strip("A great adventure."),
			Some("A great adventure.".into())
		);
	}

	#[test]
	fn tags_are_removed_without_joining_words() {
		assert_eq!(
			strip("<p>Bilbo Baggins</p><p>lives in a hole.</p>"),
			Some("Bilbo Baggins lives in a hole.".into())
		);
	}

	#[test]
	fn entities_are_decoded() {
		assert_eq!(
			strip("<p>Tolkien &amp; Serkis &quot;read&quot; it &#39;aloud&#39;</p>"),
			Some("Tolkien & Serkis \"read\" it 'aloud'".into())
		);
	}

	#[test]
	fn an_encoded_ampersand_does_not_double_decode() {
		assert_eq!(
			strip("&amp;lt;not a tag&amp;gt;"),
			Some("&lt;not a tag&gt;".into())
		);
	}

	#[test]
	fn markup_only_input_becomes_none() {
		assert_eq!(strip("<p></p>"), None);
		assert_eq!(strip_html(None), None);
	}

	#[test]
	fn audnex_timestamps_and_audible_dates_agree() {
		let audnex = normalize_date(Some("1999-12-16T00:00:00.000Z".into()));
		let audible = normalize_date(Some("1999-12-16".into()));
		assert_eq!(
			audnex, audible,
			"the same release must not differ by source"
		);
		assert_eq!(audnex.as_deref(), Some("1999-12-16"));
	}

	#[test]
	fn an_unparseable_date_is_kept_rather_than_dropped() {
		assert_eq!(
			normalize_date(Some("Winter 1999".into())),
			Some("Winter 1999".into())
		);
		assert_eq!(normalize_date(Some("   ".into())), None);
	}
}
