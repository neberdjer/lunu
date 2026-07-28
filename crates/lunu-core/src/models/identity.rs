use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::consts::reasons;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdScheme {
	Asin,
	Isbn,
}

impl IdScheme {
	pub fn as_str(&self) -> &'static str {
		match self {
			IdScheme::Asin => "asin",
			IdScheme::Isbn => "isbn",
		}
	}
}

impl fmt::Display for IdScheme {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(self.as_str())
	}
}

impl FromStr for IdScheme {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"asin" => Ok(IdScheme::Asin),
			"isbn" => Ok(IdScheme::Isbn),
			_ => Err(Error::Validation(reasons::ID_SCHEME_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalId {
	pub scheme: IdScheme,
	pub value: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub region: Option<String>,
}

impl PartialEq for ExternalId {
	fn eq(&self, other: &Self) -> bool {
		self.scheme == other.scheme && self.value == other.value
	}
}

impl Eq for ExternalId {}

impl std::hash::Hash for ExternalId {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.scheme.hash(state);
		self.value.hash(state);
	}
}

impl ExternalId {
	pub fn new(scheme: IdScheme, value: impl Into<String>) -> Self {
		Self {
			scheme,
			value: value.into(),
			region: None,
		}
	}

	pub fn asin(value: impl Into<String>) -> Self {
		Self::new(IdScheme::Asin, value)
	}

	pub fn isbn(value: impl Into<String>) -> Self {
		Self::new(IdScheme::Isbn, value)
	}

	pub fn asin_in_region(value: impl Into<String>, region: Option<String>) -> Self {
		Self::asin(value).in_region(region)
	}

	pub fn in_region(mut self, region: Option<String>) -> Self {
		self.region = region.filter(|value| !value.trim().is_empty());
		self
	}

	pub fn is(&self, scheme: IdScheme) -> bool {
		self.scheme == scheme
	}

	pub fn value_for(&self, scheme: IdScheme) -> Option<&str> {
		self.is(scheme).then_some(self.value.as_str())
	}
}

impl fmt::Display for ExternalId {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{}:{}", self.scheme, self.value)
	}
}

impl FromStr for ExternalId {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		let Some((scheme, rest)) = value.split_once(':') else {
			return Err(Error::Validation(reasons::ID_SCHEME_UNKNOWN.to_string()));
		};
		Ok(Self::new(IdScheme::from_str(scheme)?, rest))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn scheme_round_trips_through_its_wire_name() {
		for scheme in [IdScheme::Asin, IdScheme::Isbn] {
			assert_eq!(IdScheme::from_str(scheme.as_str()).unwrap(), scheme);
		}
	}

	#[test]
	fn an_unknown_scheme_is_rejected_rather_than_guessed() {
		assert!(matches!(
			IdScheme::from_str("olid"),
			Err(Error::Validation(_))
		));
	}

	#[test]
	fn display_is_qualified_so_a_log_line_says_which_dialect() {
		assert_eq!(ExternalId::asin("B123").to_string(), "asin:B123");
		assert_eq!(ExternalId::isbn("978").to_string(), "isbn:978");
	}

	#[test]
	fn the_region_is_provenance_not_identity() {
		assert_eq!(
			ExternalId::asin("B123"),
			ExternalId::asin_in_region("B123", Some("de".to_string())),
			"the same asin is the same book regardless of which region resolved it, or work \
			 lookups keyed on the id miss whenever a region is stamped on"
		);
	}

	#[test]
	fn ids_of_different_schemes_are_never_equal() {
		assert_ne!(
			ExternalId::asin("9780007487295"),
			ExternalId::isbn("9780007487295"),
			"the same digits under two schemes are two different books"
		);
	}

	#[test]
	fn a_wire_id_round_trips_through_display() {
		for id in [ExternalId::asin("B123"), ExternalId::isbn("978")] {
			assert_eq!(ExternalId::from_str(&id.to_string()).unwrap(), id);
		}
	}

	#[test]
	fn a_bare_value_is_not_an_external_id() {
		assert!(
			matches!(
				ExternalId::from_str("1705009050"),
				Err(Error::Validation(_))
			),
			"the bare form is wire back-compat, and that policy belongs to the api boundary"
		);
	}

	#[test]
	fn an_unknown_wire_scheme_is_rejected_not_guessed() {
		assert!(matches!(
			ExternalId::from_str("olid:OL123M"),
			Err(Error::Validation(_))
		));
	}
}
