pub mod indexer;
pub mod metadata;

pub(crate) mod http;

pub(crate) fn integration_error(error: impl std::fmt::Display) -> lunu_core::Error {
	lunu_core::Error::Integration(error.to_string())
}
