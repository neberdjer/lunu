use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lunu_core::Result;
use lunu_core::models::{NotificationKind, UserNotification};
use lunu_core::repo::UserNotificationRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{fetch_count, map_rows};
use crate::convert::{format_dt, parse_dt, parse_dt_opt, parse_enum};
use crate::{Db, db_error};

pub struct SqlxUserNotificationRepo {
	db: Db,
}

impl SqlxUserNotificationRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_notification(row: &AnyRow) -> Result<UserNotification> {
	let kind: String = row.try_get("kind").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let read_at: Option<String> = row.try_get("read_at").map_err(db_error)?;

	Ok(UserNotification {
		id: row.try_get("id").map_err(db_error)?,
		user_id: row.try_get("user_id").map_err(db_error)?,
		kind: parse_enum::<NotificationKind>(&kind)?,
		request_id: row.try_get("request_id").map_err(db_error)?,
		title: row.try_get("title").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
		read_at: parse_dt_opt(read_at)?,
	})
}

#[async_trait]
impl UserNotificationRepo for SqlxUserNotificationRepo {
	async fn create(&self, notification: &UserNotification) -> Result<()> {
		sqlx::query(
			"INSERT INTO user_notifications (id, user_id, kind, request_id, title, created_at, read_at) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7)",
		)
		.bind(&notification.id)
		.bind(&notification.user_id)
		.bind(notification.kind.as_str())
		.bind(notification.request_id.as_deref())
		.bind(&notification.title)
		.bind(format_dt(notification.created_at))
		.bind(notification.read_at.map(format_dt))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn list_for_user(
		&self,
		user_id: &str,
		limit: i64,
		offset: i64,
	) -> Result<Vec<UserNotification>> {
		let rows = sqlx::query(
			"SELECT * FROM user_notifications WHERE user_id = $1 \
			 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
		)
		.bind(user_id)
		.bind(limit)
		.bind(offset)
		.fetch_all(&self.db)
		.await
		.map_err(db_error)?;
		map_rows(rows, map_notification)
	}

	async fn count_for_user(&self, user_id: &str) -> Result<i64> {
		fetch_count(
			&self.db,
			sqlx::query("SELECT COUNT(*) AS count FROM user_notifications WHERE user_id = $1")
				.bind(user_id),
		)
		.await
	}

	async fn unread_count(&self, user_id: &str) -> Result<i64> {
		fetch_count(
			&self.db,
			sqlx::query(
				"SELECT COUNT(*) AS count FROM user_notifications \
				 WHERE user_id = $1 AND read_at IS NULL",
			)
			.bind(user_id),
		)
		.await
	}

	async fn mark_read(&self, user_id: &str, id: &str, at: DateTime<Utc>) -> Result<bool> {
		let result = sqlx::query(
			"UPDATE user_notifications SET read_at = $1 \
			 WHERE id = $2 AND user_id = $3 AND read_at IS NULL",
		)
		.bind(format_dt(at))
		.bind(id)
		.bind(user_id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(result.rows_affected() > 0)
	}

	async fn mark_all_read(&self, user_id: &str, at: DateTime<Utc>) -> Result<u64> {
		let result = sqlx::query(
			"UPDATE user_notifications SET read_at = $1 \
			 WHERE user_id = $2 AND read_at IS NULL",
		)
		.bind(format_dt(at))
		.bind(user_id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(result.rows_affected())
	}
}
