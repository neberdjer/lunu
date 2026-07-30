use async_trait::async_trait;
use chrono::Utc;
use lunu_core::Result;
use lunu_core::repo::NotificationDeliveryRepo;
use sqlx::Row;

use crate::convert::format_dt;
use crate::{Db, db_error, map_write_error};

pub struct SqlxNotificationDeliveryRepo {
	db: Db,
}

impl SqlxNotificationDeliveryRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

#[async_trait]
impl NotificationDeliveryRepo for SqlxNotificationDeliveryRepo {
	async fn delivered_channels(&self, job_id: &str) -> Result<Vec<String>> {
		let rows = sqlx::query("SELECT channel FROM notification_deliveries WHERE job_id = $1")
			.bind(job_id)
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		rows.iter()
			.map(|row| row.try_get::<String, _>("channel").map_err(db_error))
			.collect()
	}

	async fn record(&self, job_id: &str, channel: &str) -> Result<()> {
		sqlx::query(
			"INSERT INTO notification_deliveries (job_id, channel, delivered_at) \
			 VALUES ($1, $2, $3) ON CONFLICT (job_id, channel) DO NOTHING",
		)
		.bind(job_id)
		.bind(channel)
		.bind(format_dt(Utc::now()))
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn clear(&self, job_id: &str) -> Result<()> {
		sqlx::query("DELETE FROM notification_deliveries WHERE job_id = $1")
			.bind(job_id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn prune_orphaned(&self) -> Result<u64> {
		let pruned = sqlx::query(
			"DELETE FROM notification_deliveries WHERE NOT EXISTS \
			 (SELECT 1 FROM jobs WHERE jobs.id = notification_deliveries.job_id)",
		)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(pruned.rows_affected())
	}
}
