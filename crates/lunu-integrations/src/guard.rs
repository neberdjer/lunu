use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use lunu_core::Result;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

use crate::integration_error;

pub(crate) const MAX_FETCH_BYTES: usize = 16 * 1024 * 1024;

fn is_blocked(ip: IpAddr) -> bool {
	match ip {
		IpAddr::V4(v4) => {
			let octets = v4.octets();
			v4.is_private()
				|| v4.is_loopback()
				|| v4.is_link_local()
				|| v4.is_unspecified()
				|| v4.is_broadcast()
				|| v4.is_documentation()
				|| octets[0] == 0
				|| octets[0] >= 240
				|| (octets[0] == 100 && (octets[1] & 0b1100_0000) == 64)
				|| (octets[0] == 198 && (octets[1] & 0xfe) == 18)
				|| (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
		}
		IpAddr::V6(v6) => {
			if v6.is_loopback() || v6.is_unspecified() {
				return true;
			}
			if let Some(v4) = v6.to_ipv4() {
				return is_blocked(IpAddr::V4(v4));
			}
			let seg = v6.segments();
			(seg[0] & 0xfe00) == 0xfc00
				|| (seg[0] & 0xffc0) == 0xfe80
				|| (seg[0] == 0x0064 && seg[1] == 0xff9b)
		}
	}
}

fn url_is_blocked(url: &reqwest::Url) -> bool {
	url.host_str()
		.map(|host| host.trim_start_matches('[').trim_end_matches(']'))
		.and_then(|host| host.parse::<IpAddr>().ok())
		.is_some_and(is_blocked)
}

pub(crate) fn url_is_allowed(url: &reqwest::Url) -> bool {
	matches!(url.scheme(), "http" | "https") && !url_is_blocked(url)
}

fn blocked_redirect() -> Box<dyn std::error::Error + Send + Sync> {
	"redirect target is a blocked or non-http address".into()
}

pub(crate) fn guarded_redirect(max: usize) -> reqwest::redirect::Policy {
	reqwest::redirect::Policy::custom(move |attempt| {
		if attempt.previous().len() >= max {
			return attempt.stop();
		}
		if !url_is_allowed(attempt.url()) {
			return attempt.error(blocked_redirect());
		}
		attempt.follow()
	})
}

pub(crate) struct PublicOnlyResolver;

impl Resolve for PublicOnlyResolver {
	fn resolve(&self, name: Name) -> Resolving {
		Box::pin(async move {
			let host = name.as_str().to_string();
			let resolved = tokio::net::lookup_host((host.as_str(), 0))
				.await
				.map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { Box::new(error) })?;
			let allowed: Vec<SocketAddr> = resolved.filter(|addr| !is_blocked(addr.ip())).collect();
			if allowed.is_empty() {
				return Err::<Addrs, _>(
					"host resolves only to a private or reserved address".into(),
				);
			}
			Ok(Box::new(allowed.into_iter()) as Addrs)
		})
	}
}

pub(crate) fn public_only_dns() -> Arc<PublicOnlyResolver> {
	Arc::new(PublicOnlyResolver)
}

pub(crate) async fn bounded_bytes(response: reqwest::Response, max: usize) -> Result<Vec<u8>> {
	let content_length = response.content_length();
	if content_length.is_some_and(|len| len as usize > max) {
		return Err(integration_error("response body exceeds the allowed size"));
	}
	let mut response = response;
	let mut body = Vec::with_capacity(content_length.map_or(0, |len| (len as usize).min(max)));
	while let Some(chunk) = response.chunk().await.map_err(integration_error)? {
		if body.len() + chunk.len() > max {
			return Err(integration_error("response body exceeds the allowed size"));
		}
		body.extend_from_slice(&chunk);
	}
	Ok(body)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn internal_and_reserved_addresses_are_blocked() {
		for ip in [
			"127.0.0.1",
			"169.254.169.254",
			"10.1.2.3",
			"192.168.0.1",
			"172.16.5.5",
			"100.64.0.1",
			"0.0.0.0",
			"198.18.0.1",
			"192.0.0.1",
			"240.0.0.1",
			"::1",
			"fe80::1",
			"fc00::1",
			"::ffff:127.0.0.1",
			"::7f00:1",
			"64:ff9b::7f00:1",
		] {
			assert!(
				is_blocked(ip.parse::<IpAddr>().unwrap()),
				"{ip} must be treated as internal"
			);
		}
	}

	#[test]
	fn public_addresses_are_allowed() {
		for ip in [
			"1.1.1.1",
			"8.8.8.8",
			"93.184.216.34",
			"2606:4700:4700::1111",
		] {
			assert!(
				!is_blocked(ip.parse::<IpAddr>().unwrap()),
				"{ip} is a public address and must be allowed"
			);
		}
	}
}
