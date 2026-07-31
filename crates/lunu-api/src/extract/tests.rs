use std::net::{IpAddr, Ipv4Addr};

use actix_web::http::Method;
use actix_web::http::header::HeaderMap;
use lunu_config::BootstrapConfig;

use super::{forward_auth_username, key_scope_permits};

fn scopes(values: &[&str]) -> Vec<String> {
	values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn an_unscoped_key_keeps_full_user_authority() {
	assert!(key_scope_permits(&[], &Method::GET));
	assert!(key_scope_permits(&[], &Method::POST));
	assert!(key_scope_permits(&[], &Method::DELETE));
}

#[test]
fn a_read_scope_allows_only_safe_methods() {
	let read = scopes(&["read"]);
	assert!(key_scope_permits(&read, &Method::GET));
	assert!(key_scope_permits(&read, &Method::HEAD));
	assert!(
		!key_scope_permits(&read, &Method::POST),
		"a read-only key must not be able to mutate"
	);
	assert!(!key_scope_permits(&read, &Method::DELETE));
}

#[test]
fn write_and_admin_scopes_cover_reads_and_writes() {
	for held in [
		scopes(&["write"]),
		scopes(&["admin"]),
		scopes(&["read", "write"]),
	] {
		assert!(key_scope_permits(&held, &Method::GET));
		assert!(key_scope_permits(&held, &Method::POST));
		assert!(key_scope_permits(&held, &Method::DELETE));
	}
}

fn config(header: Option<&str>, proxies: &[IpAddr]) -> BootstrapConfig {
	BootstrapConfig {
		bind: "127.0.0.1:8080".to_string(),
		database_url: "sqlite::memory:".to_string(),
		master_key: "test-master-key-value".to_string(),
		workers: 1,
		trusted_proxy_hops: 0,
		trusted_client_ip_header: None,
		secure_cookies: false,
		url_base: String::new(),
		forward_auth_header: header.map(str::to_string),
		forward_auth_proxies: proxies.to_vec(),
		shutdown_timeout_secs: 30,
	}
}

fn headers(name: &str, value: &str) -> HeaderMap {
	let mut map = HeaderMap::new();
	map.insert(name.parse().unwrap(), value.parse().unwrap());
	map
}

const PROXY: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));
const STRANGER: IpAddr = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9));

#[test]
fn the_header_is_trusted_only_from_a_listed_proxy() {
	let config = config(Some("remote-user"), &[PROXY]);
	let headers = headers("remote-user", "alice");

	assert_eq!(
		forward_auth_username(&config, Some(PROXY), &headers).as_deref(),
		Some("alice")
	);
	assert_eq!(
		forward_auth_username(&config, Some(STRANGER), &headers),
		None,
		"a spoofed header from any other address must never authenticate"
	);
	assert_eq!(forward_auth_username(&config, None, &headers), None);
}

#[test]
fn forward_auth_stays_off_unless_fully_configured() {
	let headers = headers("remote-user", "alice");
	assert_eq!(
		forward_auth_username(&config(None, &[PROXY]), Some(PROXY), &headers),
		None
	);
	assert_eq!(
		forward_auth_username(&config(Some("remote-user"), &[]), Some(PROXY), &headers),
		None
	);
}

#[test]
fn a_blank_asserted_username_is_ignored() {
	let config = config(Some("remote-user"), &[PROXY]);
	assert_eq!(
		forward_auth_username(&config, Some(PROXY), &headers("remote-user", "  ")),
		None
	);
}
