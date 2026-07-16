use std::collections::BTreeMap;

use chrono::Utc;
use lunu_core::Result;
use lunu_core::helpers::matching::normalize;
use lunu_core::models::RequestStatus;
use lunu_core::services::new_id;
use sqlx::Row;

use crate::convert::format_dt;
use crate::repos::WORK_IS_IDENTIFIED;
use crate::{Db, db_error};

const MERGE_DECLINE_DETAIL: &str = "duplicate of an existing request after a work merge";

type WorkGroups = BTreeMap<(String, String), Vec<(String, Option<String>)>>;

pub(crate) async fn run(db: &Db) -> Result<()> {
	normalize_works(db).await?;
	merge_duplicate_works(db).await
}

async fn normalize_works(db: &Db) -> Result<()> {
	let rows = sqlx::query("SELECT id, title, author FROM works WHERE normalized_title IS NULL")
		.fetch_all(db)
		.await
		.map_err(db_error)?;

	for row in &rows {
		let id: String = row.try_get("id").map_err(db_error)?;
		let title: String = row.try_get("title").map_err(db_error)?;
		let author: Option<String> = row.try_get("author").map_err(db_error)?;

		sqlx::query("UPDATE works SET normalized_title = $1, normalized_author = $2 WHERE id = $3")
			.bind(normalize(&title))
			.bind(normalize(author.as_deref().unwrap_or_default()))
			.bind(&id)
			.execute(db)
			.await
			.map_err(db_error)?;
	}

	Ok(())
}

pub(crate) async fn merge_duplicate_works(db: &Db) -> Result<()> {
	let keys = duplicate_keys(db).await?;
	if keys.is_empty() {
		return Ok(());
	}

	let pairs: Vec<String> = (0..keys.len())
		.map(|index| format!("(${}, ${})", index * 2 + 1, index * 2 + 2))
		.collect();
	let sql = format!(
		"SELECT id, cover_url, normalized_title, normalized_author FROM works \
		 WHERE (normalized_title, normalized_author) IN ({}) AND {WORK_IS_IDENTIFIED} \
		 ORDER BY created_at, id",
		pairs.join(", ")
	);
	let mut query = sqlx::query(&sql);
	for (title, author) in keys {
		query = query.bind(title).bind(author);
	}
	let rows = query.fetch_all(db).await.map_err(db_error)?;

	let mut groups = WorkGroups::new();
	for row in &rows {
		let id: String = row.try_get("id").map_err(db_error)?;
		let cover_url: Option<String> = row.try_get("cover_url").map_err(db_error)?;
		let title: String = row.try_get("normalized_title").map_err(db_error)?;
		let author: String = row.try_get("normalized_author").map_err(db_error)?;
		groups
			.entry((title, author))
			.or_default()
			.push((id, cover_url));
	}

	for group in groups.values().filter(|group| group.len() > 1) {
		merge_group(db, group).await?;
	}

	Ok(())
}

async fn duplicate_keys(db: &Db) -> Result<Vec<(String, String)>> {
	let rows = sqlx::query(
		"SELECT normalized_title, normalized_author FROM works \
		 WHERE normalized_title IS NOT NULL AND normalized_author IS NOT NULL \
		 AND normalized_author <> '' \
		 GROUP BY normalized_title, normalized_author HAVING COUNT(*) > 1",
	)
	.fetch_all(db)
	.await
	.map_err(db_error)?;

	rows.iter()
		.map(|row| {
			let title: String = row.try_get("normalized_title").map_err(db_error)?;
			let author: String = row.try_get("normalized_author").map_err(db_error)?;
			Ok((title, author))
		})
		.collect()
}

async fn merge_group(db: &Db, group: &[(String, Option<String>)]) -> Result<()> {
	let (winner, winner_cover) = &group[0];

	for (loser, _) in &group[1..] {
		for sql in [
			"UPDATE work_external_ids SET work_id = $1 WHERE work_id = $2",
			"UPDATE media SET work_id = $1 WHERE work_id = $2",
		] {
			sqlx::query(sql)
				.bind(winner)
				.bind(loser)
				.execute(db)
				.await
				.map_err(db_error)?;
		}
		move_requests(db, winner, loser).await?;
		sqlx::query("DELETE FROM works WHERE id = $1")
			.bind(loser)
			.execute(db)
			.await
			.map_err(db_error)?;
	}

	if winner_cover.is_none()
		&& let Some(cover_url) = group[1..].iter().find_map(|(_, cover)| cover.as_deref())
	{
		sqlx::query("UPDATE works SET cover_url = $1 WHERE id = $2")
			.bind(cover_url)
			.bind(winner)
			.execute(db)
			.await
			.map_err(db_error)?;
	}

	Ok(())
}

async fn move_requests(db: &Db, winner: &str, loser: &str) -> Result<()> {
	let declined = RequestStatus::Declined.as_str();
	let failed = RequestStatus::Failed.as_str();

	sqlx::query(
		"UPDATE requests SET work_id = $1 WHERE work_id = $2 \
		 AND (status IN ($3, $4) OR NOT EXISTS ( \
		 SELECT 1 FROM requests other WHERE other.user_id = requests.user_id \
		 AND other.work_id = $5 AND other.format = requests.format \
		 AND other.status NOT IN ($6, $7)))",
	)
	.bind(winner)
	.bind(loser)
	.bind(declined)
	.bind(failed)
	.bind(winner)
	.bind(declined)
	.bind(failed)
	.execute(db)
	.await
	.map_err(db_error)?;

	let duplicates: Vec<String> = sqlx::query_scalar("SELECT id FROM requests WHERE work_id = $1")
		.bind(loser)
		.fetch_all(db)
		.await
		.map_err(db_error)?;

	for request_id in &duplicates {
		sqlx::query("UPDATE requests SET work_id = $1, status = $2, updated_at = $3 WHERE id = $4")
			.bind(winner)
			.bind(declined)
			.bind(format_dt(Utc::now()))
			.bind(request_id)
			.execute(db)
			.await
			.map_err(db_error)?;
		sqlx::query(
			"INSERT INTO activity (id, request_id, event, detail, at, actor) \
			 VALUES ($1, $2, $3, $4, $5, NULL)",
		)
		.bind(new_id())
		.bind(request_id)
		.bind(declined)
		.bind(MERGE_DECLINE_DETAIL)
		.bind(format_dt(Utc::now()))
		.execute(db)
		.await
		.map_err(db_error)?;
	}

	Ok(())
}
