use super::*;

#[test]
fn a_url_base_is_normalized_to_one_leading_slash() {
	assert_eq!(normalize_url_base("lunu"), "/lunu");
	assert_eq!(normalize_url_base("/lunu/"), "/lunu");
	assert_eq!(normalize_url_base("//lunu//"), "/lunu");
	assert_eq!(normalize_url_base("/apps/lunu"), "/apps/lunu");
}

#[test]
fn an_empty_or_root_url_base_means_no_prefix() {
	assert_eq!(normalize_url_base(""), "");
	assert_eq!(normalize_url_base("/"), "");
	assert_eq!(normalize_url_base("  "), "");
}

#[test]
fn url_base_segments_are_limited_to_unreserved_characters() {
	assert!(is_valid_url_base(""));
	assert!(is_valid_url_base("/lunu"));
	assert!(is_valid_url_base("/apps/lunu-2.0_beta~1"));
	assert!(
		!is_valid_url_base("/lunu{x}"),
		"braces are actix route metacharacters and must not reach scope()"
	);
	assert!(!is_valid_url_base("/lu nu"));
	assert!(
		!is_valid_url_base("/lunu;v=1"),
		"a semicolon corrupts the cookie path attribute"
	);
	assert!(!is_valid_url_base("/lunu?x"));
	assert!(
		!is_valid_url_base("/apps//lunu"),
		"an empty segment is a dead mount"
	);
}
