use std::str::FromStr;

use crate::consts::reasons;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
	Audiobook,
	Ebook,
}

impl Format {
	pub fn as_str(&self) -> &'static str {
		match self {
			Format::Audiobook => "audiobook",
			Format::Ebook => "ebook",
		}
	}
}

impl FromStr for Format {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"audiobook" => Ok(Format::Audiobook),
			"ebook" => Ok(Format::Ebook),
			_ => Err(Error::Validation(reasons::FORMAT_UNKNOWN.to_string())),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn format_round_trips_through_its_wire_name() {
		for format in [Format::Audiobook, Format::Ebook] {
			assert_eq!(Format::from_str(format.as_str()).unwrap(), format);
		}
	}

	#[test]
	fn an_unknown_format_is_rejected() {
		assert!(matches!(
			Format::from_str("comic"),
			Err(Error::Validation(_))
		));
	}
}
