use super::*;

const ITEM: &str = r#"{
	"id": "li_abc",
	"path": "/audiobooks/Foundation",
	"media": { "metadata": {
		"title": "Foundation",
		"authorName": "Isaac Asimov",
		"asin": "B002V0QK4C",
		"seriesName": "Foundation #1"
	}}
}"#;

#[test]
fn a_malformed_item_is_skipped_without_losing_the_rest_of_the_page() {
	let body = r#"{"results": [
		{ "id": "li_good1", "media": { "metadata": { "title": "Good One" }}},
		{ "media": { "metadata": { "title": "No Id At All" }}},
		{ "id": 12345 },
		{ "id": "li_good2", "media": { "metadata": { "title": "Good Two" }}}
	]}"#;

	let parsed: ItemsResponse =
		serde_json::from_str(body).expect("a bad item must not fail the page");
	let ids: Vec<&str> = parsed.results.iter().map(|item| item.id.as_str()).collect();
	assert_eq!(
		ids,
		vec!["li_good1", "li_good2"],
		"good items survive, malformed ones are dropped"
	);
}

#[test]
fn parses_minified_abs_item_into_library_item() {
	let item: AbsItem = serde_json::from_str(ITEM).unwrap();
	let mapped = into_item("https://abs.example.com", item);
	assert_eq!(mapped.abs_item_id, "li_abc");
	assert_eq!(mapped.asin.as_deref(), Some("B002V0QK4C"));
	assert_eq!(mapped.title, "Foundation");
	assert_eq!(mapped.author.as_deref(), Some("Isaac Asimov"));
	assert_eq!(mapped.series_name.as_deref(), Some("Foundation"));
	assert_eq!(mapped.series_sequence.as_deref(), Some("1"));
	assert_eq!(
		mapped.cover_url.as_deref(),
		Some("https://abs.example.com/api/items/li_abc/cover")
	);
}

#[test]
fn item_without_asin_still_maps() {
	let item: AbsItem =
		serde_json::from_str(r#"{ "id": "li_x", "media": { "metadata": { "title": "Mystery" }}}"#)
			.unwrap();
	let mapped = into_item("https://abs.example.com", item);
	assert!(mapped.asin.is_none());
	assert_eq!(mapped.title, "Mystery");
	assert!(mapped.series_name.is_none());
}

#[test]
fn parses_series_name_variants() {
	assert_eq!(
		parse_series_name(Some("Foundation #1".to_string())),
		(Some("Foundation".to_string()), Some("1".to_string()))
	);
	assert_eq!(
		parse_series_name(Some("Sword of Truth".to_string())),
		(Some("Sword of Truth".to_string()), None)
	);
	assert_eq!(
		parse_series_name(Some("Foundation #1.5".to_string())),
		(Some("Foundation".to_string()), Some("1.5".to_string()))
	);
	assert_eq!(
		parse_series_name(Some("Series A #1, Series B #2".to_string())),
		(Some("Series A".to_string()), Some("1".to_string()))
	);
	assert_eq!(
		parse_series_name(Some("Star Wars #7 Legends".to_string())),
		(Some("Star Wars #7 Legends".to_string()), None)
	);
	assert_eq!(parse_series_name(Some(String::new())), (None, None));
	assert_eq!(parse_series_name(None), (None, None));
}
