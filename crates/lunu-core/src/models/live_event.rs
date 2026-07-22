use crate::models::{Activity, Download, Media, UserNotification};

#[derive(Debug, Clone)]
pub enum LiveEvent {
	Activity(Activity),
	Progress(Download),
	Notification(UserNotification),
	Merge(Box<Media>),
}
