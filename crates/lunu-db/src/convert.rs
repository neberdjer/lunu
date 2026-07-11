use std::str::FromStr;

use chrono::{DateTime, SecondsFormat, Utc};
use lunu_core::{Error, Result};

use crate::db_error;

pub(crate) fn parse_enum<T>(value: &str) -> Result<T>
where
	T: FromStr,
	T::Err: std::fmt::Display,
{
	value.parse::<T>().map_err(db_error)
}

pub(crate) fn format_dt(value: DateTime<Utc>) -> String {
	value.to_rfc3339_opts(SecondsFormat::Micros, true)
}

pub(crate) fn parse_dt(value: &str) -> Result<DateTime<Utc>> {
	DateTime::parse_from_rfc3339(value)
		.map(|value| value.with_timezone(&Utc))
		.map_err(|error| Error::Database(format!("invalid timestamp '{value}': {error}")))
}

pub(crate) fn parse_dt_opt(value: Option<String>) -> Result<Option<DateTime<Utc>>> {
	value.as_deref().map(parse_dt).transpose()
}

pub(crate) fn bool_to_int(value: bool) -> i64 {
	i64::from(value)
}

pub(crate) fn int_to_bool(value: i64) -> bool {
	value != 0
}

pub(crate) fn join_list(items: &[String]) -> String {
	items.join(",")
}

pub(crate) fn split_list(value: &str) -> Vec<String> {
	if value.is_empty() {
		Vec::new()
	} else {
		value.split(',').map(str::to_string).collect()
	}
}
