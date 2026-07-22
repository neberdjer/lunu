use lunu_core::models::Media;
use lunu_core::traits::MergePreview;
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct MediaResponse {
	pub id: String,
	pub asin: Option<String>,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub series_name: Option<String>,
	pub series_sequence: Option<String>,
	pub merged_path: Option<String>,
	pub merge_state: String,
	pub merge_detail: Option<String>,
	pub merge_backup_path: Option<String>,
	pub source: String,
	pub overridden: bool,
	pub matched_by: Option<String>,
}

impl From<Media> for MediaResponse {
	fn from(media: Media) -> Self {
		Self {
			id: media.id,
			asin: media.asin,
			title: media.title,
			author: media.author,
			cover_url: media.cover_url,
			merged_path: media.merged_path,
			merge_state: media.merge_state.as_str().to_string(),
			merge_detail: media.merge_detail,
			merge_backup_path: media.merge_backup_path,
			series_name: media.series_name,
			series_sequence: media.series_sequence,
			source: media.source.as_str().to_string(),
			overridden: media.overridden,
			matched_by: media.matched_by.map(|matched| matched.as_str().to_string()),
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct PreviewChapterResponse {
	pub title: String,
	pub seconds: f64,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct MergePreviewResponse {
	pub output_path: String,
	pub would_skip: Option<String>,
	pub chapters: Vec<PreviewChapterResponse>,
	pub total_seconds: f64,
	pub stream_copy: bool,
	pub source_action: String,
	pub backup_path: Option<String>,
	pub bitrate: String,
}

impl From<MergePreview> for MergePreviewResponse {
	fn from(preview: MergePreview) -> Self {
		let source_action = preview.sources.as_str().to_string();
		let backup_path = preview.sources.backup().map(str::to_string);
		Self {
			output_path: preview.output_path,
			would_skip: preview.skip.map(str::to_string),
			chapters: preview
				.chapters
				.into_iter()
				.map(|chapter| PreviewChapterResponse {
					title: chapter.title,
					seconds: chapter.seconds,
				})
				.collect(),
			total_seconds: preview.total_seconds,
			stream_copy: preview.copyable,
			source_action,
			backup_path,
			bitrate: preview.bitrate,
		}
	}
}
