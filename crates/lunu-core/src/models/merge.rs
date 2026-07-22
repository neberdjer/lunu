use crate::consts::merge::{MERGE_SOURCE_DELETE, MERGE_SOURCE_KEEP, MERGE_SOURCE_MOVE};
use crate::consts::reasons;
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceDisposition {
	Keep,
	Delete,
	Move { backup: String },
}

impl SourceDisposition {
	pub fn as_str(&self) -> &'static str {
		match self {
			SourceDisposition::Keep => MERGE_SOURCE_KEEP,
			SourceDisposition::Delete => MERGE_SOURCE_DELETE,
			SourceDisposition::Move { .. } => MERGE_SOURCE_MOVE,
		}
	}

	pub fn backup(&self) -> Option<&str> {
		match self {
			SourceDisposition::Move { backup } => Some(backup),
			SourceDisposition::Keep | SourceDisposition::Delete => None,
		}
	}

	pub fn resolve(action: &str, backup: Option<String>) -> Result<Self> {
		match action {
			MERGE_SOURCE_KEEP => Ok(SourceDisposition::Keep),
			MERGE_SOURCE_DELETE => Ok(SourceDisposition::Delete),
			MERGE_SOURCE_MOVE => backup
				.map(|backup| SourceDisposition::Move { backup })
				.ok_or_else(|| Error::Validation(reasons::MERGE_BACKUP_NOT_CONFIGURED.to_string())),
			_ => Err(Error::Validation(
				reasons::MERGE_SOURCE_ACTION_UNKNOWN.to_string(),
			)),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn moving_without_a_backup_directory_cannot_be_represented() {
		assert!(matches!(
			SourceDisposition::resolve(MERGE_SOURCE_MOVE, None),
			Err(Error::Validation(reason)) if reason == reasons::MERGE_BACKUP_NOT_CONFIGURED
		));
		assert_eq!(
			SourceDisposition::resolve(MERGE_SOURCE_MOVE, Some("/backup".to_string())).unwrap(),
			SourceDisposition::Move {
				backup: "/backup".to_string()
			}
		);
	}

	#[test]
	fn the_other_actions_never_need_a_backup_directory() {
		for action in [MERGE_SOURCE_KEEP, MERGE_SOURCE_DELETE] {
			assert!(SourceDisposition::resolve(action, None).is_ok());
		}
	}

	#[test]
	fn an_unknown_action_is_rejected_rather_than_guessed() {
		assert!(matches!(
			SourceDisposition::resolve("shred", None),
			Err(Error::Validation(reason)) if reason == reasons::MERGE_SOURCE_ACTION_UNKNOWN
		));
	}
}
