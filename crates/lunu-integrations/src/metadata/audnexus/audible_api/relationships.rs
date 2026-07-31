use lunu_core::Result;
use reqwest::StatusCode;
use serde::Deserialize;

use super::audible_host;
use crate::http::send_with_retry;
use crate::integration_error;

const SERIES: &str = "series";
const CHILD: &str = "child";
const PARENT: &str = "parent";

#[derive(Deserialize)]
struct RelationshipsResponse {
	#[serde(default)]
	product: RelationshipsProduct,
}

#[derive(Deserialize, Default)]
struct RelationshipsProduct {
	#[serde(default)]
	relationships: Vec<Relationship>,
}

#[derive(Deserialize, Clone)]
pub(crate) struct Relationship {
	pub(crate) asin: String,
	#[serde(default)]
	pub(crate) title: Option<String>,
	#[serde(default)]
	pub(crate) sequence: Option<String>,
	#[serde(default)]
	sort: Option<String>,
	#[serde(default)]
	relationship_type: Option<String>,
	#[serde(default)]
	relationship_to_product: Option<String>,
}

impl Relationship {
	pub(crate) fn slot(&self) -> u32 {
		self.sort
			.as_deref()
			.and_then(|sort| sort.parse::<u32>().ok())
			.unwrap_or(u32::MAX)
	}

	fn is_series(&self, to_product: &str) -> bool {
		self.relationship_type.as_deref() == Some(SERIES)
			&& self.relationship_to_product.as_deref() == Some(to_product)
	}
}

async fn fetch(client: &reqwest::Client, region: &str, asin: &str) -> Result<Vec<Relationship>> {
	let url = format!(
		"https://{}/1.0/catalog/products/{asin}",
		audible_host(region)
	);
	let response = send_with_retry(|| {
		client
			.get(&url)
			.query(&[("response_groups", "relationships")])
	})
	.await?;

	if response.status() == StatusCode::NOT_FOUND {
		return Ok(Vec::new());
	}

	let response = response.error_for_status().map_err(integration_error)?;
	let body: RelationshipsResponse = crate::http::bounded_json(response).await?;
	Ok(body.product.relationships)
}

fn series_only(relationships: Vec<Relationship>, to_product: &str) -> Vec<Relationship> {
	relationships
		.into_iter()
		.filter(|entry| entry.is_series(to_product))
		.collect()
}

pub(crate) async fn series_children(
	client: &reqwest::Client,
	region: &str,
	series_asin: &str,
) -> Result<Vec<Relationship>> {
	Ok(series_only(
		fetch(client, region, series_asin).await?,
		CHILD,
	))
}

pub(crate) async fn series_parents(
	client: &reqwest::Client,
	region: &str,
	asin: &str,
) -> Result<Vec<Relationship>> {
	Ok(series_only(fetch(client, region, asin).await?, PARENT))
}

#[cfg(test)]
mod tests {
	use super::*;

	const LOTR_SERIES: &str = r#"{
		"product": {
			"asin": "B009CFOEGK",
			"relationships": [
				{"asin": "B016N9U1XW", "sort": "1", "sequence": "0", "relationship_type": "series", "relationship_to_product": "child"},
				{"asin": "1705009050", "sort": "2", "sequence": "0.5", "relationship_type": "series", "relationship_to_product": "child"},
				{"asin": "B0030EJV3U", "sort": "2", "sequence": "0.5", "relationship_type": "series", "relationship_to_product": "child"},
				{"asin": "B0DNN628VN", "sort": "1", "relationship_type": "component", "relationship_to_product": "child"},
				{"asin": "B002V5BUUG", "sort": "10", "sequence": "1", "title": "The Lord of the Rings", "relationship_type": "series", "relationship_to_product": "parent"}
			]
		}
	}"#;

	fn relationships() -> Vec<Relationship> {
		serde_json::from_str::<RelationshipsResponse>(LOTR_SERIES)
			.unwrap()
			.product
			.relationships
	}

	#[test]
	fn children_exclude_parents_and_non_series_relationships() {
		let children = series_only(relationships(), CHILD);
		let asins: Vec<&str> = children.iter().map(|c| c.asin.as_str()).collect();
		assert_eq!(
			asins,
			["B016N9U1XW", "1705009050", "B0030EJV3U"],
			"a component child is not a series member"
		);
	}

	#[test]
	fn parents_carry_the_series_name_and_position() {
		let parents = series_only(relationships(), PARENT);
		assert_eq!(parents.len(), 1);
		assert_eq!(parents[0].title.as_deref(), Some("The Lord of the Rings"));
		assert_eq!(parents[0].sequence.as_deref(), Some("1"));
	}

	#[test]
	fn slot_orders_numerically_not_lexicographically() {
		let children = series_only(relationships(), CHILD);
		assert_eq!(children[0].slot(), 1);
		assert_eq!(children[1].slot(), 2);
		let parent = &series_only(relationships(), PARENT)[0];
		assert!(
			parent.slot() > children[1].slot(),
			"sort 10 must outrank sort 2, which a string compare would get backwards"
		);
	}

	#[test]
	fn a_missing_sort_sinks_to_the_end() {
		let orphan: Relationship = serde_json::from_str(r#"{"asin": "X"}"#).unwrap();
		assert_eq!(orphan.slot(), u32::MAX);
	}

	#[test]
	fn a_product_with_no_relationships_parses_to_empty() {
		let body: RelationshipsResponse =
			serde_json::from_str(r#"{"product": {"asin": "1705009050"}}"#).unwrap();
		assert!(body.product.relationships.is_empty());
	}
}
