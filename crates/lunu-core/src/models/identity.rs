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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExternalId {
	pub scheme: IdScheme,
	pub value: String,
}

impl ExternalId {
	pub fn new(scheme: IdScheme, value: impl Into<String>) -> Self {
		Self {
			scheme,
			value: value.into(),
		}
	}

	pub fn asin(value: impl Into<String>) -> Self {
		Self::new(IdScheme::Asin, value)
	}

	pub fn isbn(value: impl Into<String>) -> Self {
		Self::new(IdScheme::Isbn, value)
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
	fn ids_of_different_schemes_are_never_equal() {
		assert_ne!(
			ExternalId::asin("9780007487295"),
			ExternalId::isbn("9780007487295"),
			"the same digits under two schemes are two different books"
		);
	}
}
