use std::str::FromStr;

use lunu_core::Result;
use lunu_core::models::ExternalId;

pub(crate) fn parse_wire_id(value: &str) -> Result<ExternalId> {
	if value.contains(':') {
		ExternalId::from_str(value)
	} else {
		Ok(ExternalId::asin(value))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use lunu_core::Error;
	use lunu_core::models::IdScheme;

	#[test]
	fn a_bare_wire_id_reads_as_an_asin() {
		assert_eq!(
			parse_wire_id("1705009050").unwrap(),
			ExternalId::asin("1705009050"),
			"every id a client held before schemes existed was an asin"
		);
	}

	#[test]
	fn a_qualified_wire_id_keeps_its_scheme() {
		assert_eq!(
			parse_wire_id("isbn:9780007487295").unwrap(),
			ExternalId::new(IdScheme::Isbn, "9780007487295")
		);
	}

	#[test]
	fn an_unknown_scheme_is_rejected() {
		assert!(matches!(
			parse_wire_id("olid:OL123M"),
			Err(Error::Validation(_))
		));
	}
}
