use actix_web::HttpRequest;
use actix_web::http::header::HeaderMap;
use lunu_config::BootstrapConfig;

const X_FORWARDED_FOR: &str = "x-forwarded-for";

pub(crate) fn client_ip(req: &HttpRequest, config: &BootstrapConfig) -> String {
	resolve(req.headers(), peer(req), config)
}

fn peer(req: &HttpRequest) -> Option<String> {
	req.peer_addr().map(|addr| addr.ip().to_string())
}

fn resolve(headers: &HeaderMap, peer: Option<String>, config: &BootstrapConfig) -> String {
	let forwarded = if let Some(name) = &config.trusted_client_ip_header {
		single_header(headers, name)
	} else if config.trusted_proxy_hops > 0 {
		forwarded_for(headers, config.trusted_proxy_hops)
	} else {
		None
	};

	forwarded.or(peer).unwrap_or_default()
}

fn single_header(headers: &HeaderMap, name: &str) -> Option<String> {
	let value = headers.get(name)?.to_str().ok()?.trim();
	if value.is_empty() {
		None
	} else {
		Some(value.to_string())
	}
}

fn forwarded_for(headers: &HeaderMap, hops: usize) -> Option<String> {
	let raw = headers.get(X_FORWARDED_FOR)?.to_str().ok()?;
	let parts: Vec<&str> = raw
		.split(',')
		.map(str::trim)
		.filter(|part| !part.is_empty())
		.collect();

	let index = parts.len().checked_sub(hops)?;
	Some(parts[index].to_string())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn config(hops: usize, header: Option<&str>) -> BootstrapConfig {
		BootstrapConfig {
			bind: "127.0.0.1:8080".to_string(),
			database_url: "sqlite::memory:".to_string(),
			master_key: "test-master-key-value".to_string(),
			workers: 1,
			trusted_proxy_hops: hops,
			trusted_client_ip_header: header.map(str::to_string),
			secure_cookies: false,
			url_base: String::new(),
		}
	}

	fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
		let mut map = HeaderMap::new();
		for (name, value) in pairs {
			map.insert(name.parse().unwrap(), value.parse().unwrap());
		}
		map
	}

	#[test]
	fn direct_default_ignores_forwarded_headers() {
		let resolved = resolve(
			&headers(&[("x-forwarded-for", "1.1.1.1, 9.9.9.9")]),
			Some("203.0.113.7".to_string()),
			&config(0, None),
		);
		assert_eq!(resolved, "203.0.113.7");
	}

	#[test]
	fn one_hop_takes_rightmost_ignoring_spoofed_prefix() {
		let resolved = resolve(
			&headers(&[("x-forwarded-for", "6.6.6.6, 2.2.2.2, 203.0.113.9")]),
			Some("10.0.0.1".to_string()),
			&config(1, None),
		);
		assert_eq!(resolved, "203.0.113.9");
	}

	#[test]
	fn two_hops_walks_two_from_the_right() {
		let resolved = resolve(
			&headers(&[("x-forwarded-for", "6.6.6.6, 203.0.113.9, 172.16.0.2")]),
			Some("10.0.0.1".to_string()),
			&config(2, None),
		);
		assert_eq!(resolved, "203.0.113.9");
	}

	#[test]
	fn fewer_entries_than_hops_fails_closed_to_peer() {
		let resolved = resolve(
			&headers(&[("x-forwarded-for", "6.6.6.6")]),
			Some("10.0.0.1".to_string()),
			&config(2, None),
		);
		assert_eq!(resolved, "10.0.0.1");
	}

	#[test]
	fn missing_forwarded_header_fails_closed_to_peer() {
		let resolved = resolve(
			&headers(&[]),
			Some("10.0.0.1".to_string()),
			&config(1, None),
		);
		assert_eq!(resolved, "10.0.0.1");
	}

	#[test]
	fn trusted_single_header_is_used_directly() {
		let resolved = resolve(
			&headers(&[
				("cf-connecting-ip", "203.0.113.5"),
				("x-forwarded-for", "6.6.6.6"),
			]),
			Some("10.0.0.1".to_string()),
			&config(0, Some("cf-connecting-ip")),
		);
		assert_eq!(resolved, "203.0.113.5");
	}

	#[test]
	fn trusted_single_header_absent_fails_closed_to_peer() {
		let resolved = resolve(
			&headers(&[("x-forwarded-for", "6.6.6.6")]),
			Some("10.0.0.1".to_string()),
			&config(0, Some("cf-connecting-ip")),
		);
		assert_eq!(resolved, "10.0.0.1");
	}
}
