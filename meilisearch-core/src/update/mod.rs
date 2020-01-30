mod clear_all;
mod customs_update;
mod documents_addition;
mod documents_deletion;
mod settings_update;

pub use self::clear_all::{apply_clear_all, push_clear_all};
pub use self::customs_update::{apply_customs_update, push_customs_update};
pub use self::documents_addition::{
    apply_documents_addition, apply_documents_partial_addition, DocumentsAddition,
};
pub use self::documents_deletion::{apply_documents_deletion, DocumentsDeletion};
pub use self::settings_update::{apply_settings_update, push_settings_update};

use std::cmp;
use std::collections::HashMap;
use std::time::Instant;

use chrono::{DateTime, Utc};
use heed::Result as ZResult;
use log::debug;
use serde::{Deserialize, Serialize};

use crate::{store, DocumentId, MResult};
use crate::database::{MainT, UpdateT};
use crate::settings::SettingsUpdate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Update {
    data: UpdateData,
    enqueued_at: DateTime<Utc>,
}

impl Update {
    fn clear_all() -> Update {
        Update {
            data: UpdateData::ClearAll,
            enqueued_at: Utc::now(),
        }
    }

    fn customs(data: Vec<u8>) -> Update {
        Update {
            data: UpdateData::Customs(data),
            enqueued_at: Utc::now(),
        }
    }

    fn documents_addition(data: Vec<HashMap<String, serde_json::Value>>) -> Update {
        Update {
            data: UpdateData::DocumentsAddition(data),
            enqueued_at: Utc::now(),
        }
    }

    fn documents_partial(data: Vec<HashMap<String, serde_json::Value>>) -> Update {
        Update {
            data: UpdateData::DocumentsPartial(data),
            enqueued_at: Utc::now(),
        }
    }

    fn documents_deletion(data: Vec<DocumentId>) -> Update {
        Update {
            data: UpdateData::DocumentsDeletion(data),
            enqueued_at: Utc::now(),
        }
    }

    fn settings(data: SettingsUpdate) -> Update {
        Update {
            data: UpdateData::Settings(data),
            enqueued_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateData {
    ClearAll,
    Customs(Vec<u8>),
    DocumentsAddition(Vec<HashMap<String, serde_json::Value>>),
    DocumentsPartial(Vec<HashMap<String, serde_json::Value>>),
    DocumentsDeletion(Vec<DocumentId>),
    Settings(SettingsUpdate)
}

impl UpdateData {
    pub fn update_type(&self) -> UpdateType {
        match self {
            UpdateData::ClearAll => UpdateType::ClearAll,
            UpdateData::Customs(_) => UpdateType::Customs,
            UpdateData::DocumentsAddition(addition) => UpdateType::DocumentsAddition {
                number: addition.len(),
            },
            UpdateData::DocumentsPartial(addition) => UpdateType::DocumentsPartial {
                number: addition.len(),
            },
            UpdateData::DocumentsDeletion(deletion) => UpdateType::DocumentsDeletion {
                number: deletion.len(),
            },
            UpdateData::Settings(update) => UpdateType::Settings {
                settings: update.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name")]
pub enum UpdateType {
    ClearAll,
    Customs,
    DocumentsAddition { number: usize },
    DocumentsPartial { number: usize },
    DocumentsDeletion { number: usize },
    Settings { settings: SettingsUpdate },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessedUpdateResult {
    pub update_id: u64,
    #[serde(rename = "type")]
    pub update_type: UpdateType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub duration: f64, // in seconds
    pub enqueued_at: DateTime<Utc>,
    pub processed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueuedUpdateResult {
    pub update_id: u64,
    #[serde(rename = "type")]
    pub update_type: UpdateType,
    pub enqueued_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum UpdateStatus {
    Enqueued {
        #[serde(flatten)]
        content: EnqueuedUpdateResult,
    },
    Failed {
        #[serde(flatten)]
        content: ProcessedUpdateResult,
    },
    Processed {
        #[serde(flatten)]
        content: ProcessedUpdateResult,
    },
}

pub fn update_status(
    update_reader: &heed::RoTxn<UpdateT>,
    updates_store: store::Updates,
    updates_results_store: store::UpdatesResults,
    update_id: u64,
) -> MResult<Option<UpdateStatus>> {
    match updates_results_store.update_result(update_reader, update_id)? {
        Some(result) => {
            if result.error.is_some() {
                Ok(Some(UpdateStatus::Failed { content: result }))
            } else {
                Ok(Some(UpdateStatus::Processed { content: result }))
            }
        },
        None => match updates_store.get(update_reader, update_id)? {
            Some(update) => Ok(Some(UpdateStatus::Enqueued {
                content: EnqueuedUpdateResult {
                    update_id,
                    update_type: update.data.update_type(),
                    enqueued_at: update.enqueued_at,
                },
            })),
            None => Ok(None),
        },
    }
}

pub fn next_update_id(
    update_writer: &mut heed::RwTxn<UpdateT>,
    updates_store: store::Updates,
    updates_results_store: store::UpdatesResults,
) -> ZResult<u64> {
    let last_update = updates_store.last_update(update_writer)?;
    let last_update = last_update.map(|(n, _)| n);

    let last_update_results_id = updates_results_store.last_update(update_writer)?;
    let last_update_results_id = last_update_results_id.map(|(n, _)| n);

    let max_update_id = cmp::max(last_update, last_update_results_id);
    let new_update_id = max_update_id.map_or(0, |n| n + 1);

    Ok(new_update_id)
}

pub fn update_task<'a, 'b>(
    writer: &'a mut heed::RwTxn<'b, MainT>,
    index: &store::Index,
    update_id: u64,
    update: Update,
) -> MResult<ProcessedUpdateResult> {
    debug!("Processing update number {}", update_id);

    let Update { enqueued_at, data } = update;

    let (update_type, result, duration) = match data {
        UpdateData::ClearAll => {
            let start = Instant::now();

            let update_type = UpdateType::ClearAll;
            let result = apply_clear_all(
                writer,
                index.main,
                index.documents_fields,
                index.documents_fields_counts,
                index.postings_lists,
                index.docs_words,
            );

            (update_type, result, start.elapsed())
        }
        UpdateData::Customs(customs) => {
            let start = Instant::now();

            let update_type = UpdateType::Customs;
            let result = apply_customs_update(writer, index.main, &customs).map_err(Into::into);

            (update_type, result, start.elapsed())
        }
        UpdateData::DocumentsAddition(documents) => {
            let start = Instant::now();

            let update_type = UpdateType::DocumentsAddition {
                number: documents.len(),
            };

            let result = apply_documents_addition(
                writer,
                index.main,
                index.documents_fields,
                index.documents_fields_counts,
                index.postings_lists,
                index.docs_words,
                documents,
            );

            (update_type, result, start.elapsed())
        }
        UpdateData::DocumentsPartial(documents) => {
            let start = Instant::now();

            let update_type = UpdateType::DocumentsPartial {
                number: documents.len(),
            };

            let result = apply_documents_partial_addition(
                writer,
                index.main,
                index.documents_fields,
                index.documents_fields_counts,
                index.postings_lists,
                index.docs_words,
                documents,
            );

            (update_type, result, start.elapsed())
        }
        UpdateData::DocumentsDeletion(documents) => {
            let start = Instant::now();

            let update_type = UpdateType::DocumentsDeletion {
                number: documents.len(),
            };

            let result = apply_documents_deletion(
                writer,
                index.main,
                index.documents_fields,
                index.documents_fields_counts,
                index.postings_lists,
                index.docs_words,
                documents,
            );

            (update_type, result, start.elapsed())
        }
        UpdateData::Settings(settings) => {
            let start = Instant::now();

            let update_type = UpdateType::Settings {
                settings: settings.clone(),
            };

            let result = apply_settings_update(
                writer,
                index,
                settings,
            );

            (update_type, result, start.elapsed())
        }
    };

    debug!(
        "Processed update number {} {:?} {:?}",
        update_id, update_type, result
    );

    let status = ProcessedUpdateResult {
        update_id,
        update_type,
        error: result.map_err(|e| e.to_string()).err(),
        duration: duration.as_secs_f64(),
        enqueued_at,
        processed_at: Utc::now(),
    };

    Ok(status)
}
