use crate::errors::ApiError;
use crate::models::ProfileDocument;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct ProfileStore {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

#[derive(Clone, Debug, Default)]
pub struct InventoryPresentation {
    pub device_presentations: HashMap<String, DevicePresentationRecord>,
    pub controller_labels: HashMap<String, Option<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevicePresentationRecord {
    pub device_id: String,
    pub display_name: Option<String>,
    pub ui_order: Option<u32>,
    pub physical_role: Option<String>,
    pub cluster_label: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ControllerLabelRecord {
    controller_id: String,
    label: Option<String>,
}

impl ProfileStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn list(&self) -> Result<Vec<ProfileDocument>, ApiError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| ApiError::Internal("profile store lock poisoned".to_string()))?;
        Ok(self.read_unlocked()?.profiles)
    }

    pub fn get(&self, id: &str) -> Result<Option<ProfileDocument>, ApiError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| ApiError::Internal("profile store lock poisoned".to_string()))?;
        Ok(self
            .read_unlocked()?
            .profiles
            .into_iter()
            .find(|profile| profile.id == id))
    }

    pub fn create(&self, profile: ProfileDocument) -> Result<ProfileDocument, ApiError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| ApiError::Internal("profile store lock poisoned".to_string()))?;
        let mut file = self.read_unlocked()?;

        if file.profiles.iter().any(|existing| existing.id == profile.id) {
            return Err(ApiError::BadRequest(format!(
                "profile id '{}' already exists",
                profile.id
            )));
        }

        file.profiles.push(profile.clone());
        file.profiles
            .sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        self.write_unlocked(&file)?;
        Ok(profile)
    }

    pub fn update(&self, id: &str, profile: ProfileDocument) -> Result<ProfileDocument, ApiError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| ApiError::Internal("profile store lock poisoned".to_string()))?;
        let mut file = self.read_unlocked()?;

        let Some(existing) = file.profiles.iter_mut().find(|existing| existing.id == id) else {
            return Err(ApiError::NotFound(format!("unknown profile id: {id}")));
        };

        *existing = profile.clone();
        file.profiles
            .sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        self.write_unlocked(&file)?;
        Ok(profile)
    }

    pub fn delete(&self, id: &str) -> Result<(), ApiError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| ApiError::Internal("profile store lock poisoned".to_string()))?;
        let mut file = self.read_unlocked()?;
        let original_len = file.profiles.len();
        file.profiles.retain(|profile| profile.id != id);

        if file.profiles.len() == original_len {
            return Err(ApiError::NotFound(format!("unknown profile id: {id}")));
        }

        self.write_unlocked(&file)
    }

    pub fn inventory_presentation(&self) -> Result<InventoryPresentation, ApiError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| ApiError::Internal("profile store lock poisoned".to_string()))?;
        let file = self.read_unlocked()?;

        Ok(InventoryPresentation {
            device_presentations: file
                .device_presentations
                .into_iter()
                .map(|record| (record.device_id.clone(), record))
                .collect(),
            controller_labels: file
                .controller_labels
                .into_iter()
                .map(|record| (record.controller_id, record.label))
                .collect(),
        })
    }

    pub fn upsert_device_presentation(
        &self,
        presentation: DevicePresentationRecord,
    ) -> Result<DevicePresentationRecord, ApiError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| ApiError::Internal("profile store lock poisoned".to_string()))?;
        let mut file = self.read_unlocked()?;

        if let Some(existing) = file
            .device_presentations
            .iter_mut()
            .find(|existing| existing.device_id == presentation.device_id)
        {
            *existing = presentation.clone();
        } else {
            file.device_presentations.push(presentation.clone());
        }

        file.device_presentations
            .sort_by(|left, right| left.device_id.cmp(&right.device_id));
        self.write_unlocked(&file)?;
        Ok(presentation)
    }

    pub fn upsert_controller_label(
        &self,
        controller_id: &str,
        label: Option<String>,
    ) -> Result<(), ApiError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| ApiError::Internal("profile store lock poisoned".to_string()))?;
        let mut file = self.read_unlocked()?;

        if let Some(existing) = file
            .controller_labels
            .iter_mut()
            .find(|existing| existing.controller_id == controller_id)
        {
            existing.label = label.clone();
        } else {
            file.controller_labels.push(ControllerLabelRecord {
                controller_id: controller_id.to_string(),
                label: label.clone(),
            });
        }

        file.controller_labels
            .retain(|record| record.label.as_ref().is_some_and(|value| !value.is_empty()));
        file.controller_labels
            .sort_by(|left, right| left.controller_id.cmp(&right.controller_id));
        self.write_unlocked(&file)
    }

    fn read_unlocked(&self) -> Result<ProfileStoreFile, ApiError> {
        if !self.path.exists() {
            return Ok(ProfileStoreFile::default());
        }

        let raw = fs::read_to_string(&self.path).map_err(|err| {
            ApiError::Internal(format!(
                "failed to read profile store '{}': {err}",
                self.path.display()
            ))
        })?;

        serde_json::from_str(&raw).map_err(|err| {
            ApiError::Internal(format!(
                "failed to parse profile store '{}': {err}",
                self.path.display()
            ))
        })
    }

    fn write_unlocked(&self, file: &ProfileStoreFile) -> Result<(), ApiError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                ApiError::Internal(format!(
                    "failed to create profile store directory '{}': {err}",
                    parent.display()
                ))
            })?;
        }

        let payload = serde_json::to_vec_pretty(file).map_err(|err| {
            ApiError::Internal(format!(
                "failed to serialize profile store '{}': {err}",
                self.path.display()
            ))
        })?;

        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, payload).map_err(|err| {
            ApiError::Internal(format!(
                "failed to write profile store temp file '{}': {err}",
                temp_path.display()
            ))
        })?;

        if self.path.exists() {
            fs::remove_file(&self.path).map_err(|err| {
                ApiError::Internal(format!(
                    "failed to replace profile store '{}': {err}",
                    self.path.display()
                ))
            })?;
        }

        fs::rename(&temp_path, &self.path).map_err(|err| {
            ApiError::Internal(format!(
                "failed to move profile store temp file into '{}': {err}",
                self.path.display()
            ))
        })?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ProfileStoreFile {
    version: u32,
    #[serde(default)]
    profiles: Vec<ProfileDocument>,
    #[serde(default)]
    device_presentations: Vec<DevicePresentationRecord>,
    #[serde(default)]
    controller_labels: Vec<ControllerLabelRecord>,
}

impl Default for ProfileStoreFile {
    fn default() -> Self {
        Self {
            version: 2,
            profiles: Vec::new(),
            device_presentations: Vec::new(),
            controller_labels: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProfileStore;
    use crate::errors::ApiError;
    use crate::models::{
        ProfileDocument, ProfileFanDocument, ProfileLightingDocument, ProfileMetadataDocument,
        ProfileTargetsDocument,
    };
    use std::fs;

    fn sample_profile(name: &str) -> ProfileDocument {
        ProfileDocument {
            id: name.to_string(),
            name: format!("Profile {name}"),
            description: Some("test profile".to_string()),
            targets: ProfileTargetsDocument {
                mode: "all".to_string(),
                device_ids: Vec::new(),
            },
            lighting: Some(ProfileLightingDocument {
                enabled: true,
                color: Some("#112233".to_string()),
                effect: Some("Static".to_string()),
                brightness_percent: Some(50),
                speed: None,
                direction: None,
                scope: None,
            }),
            fans: Some(ProfileFanDocument {
                enabled: true,
                mode: "manual".to_string(),
                percent: Some(40),
            }),
            metadata: ProfileMetadataDocument {
                created_at: "2026-03-13T12:00:00Z".to_string(),
                updated_at: "2026-03-13T12:00:00Z".to_string(),
            },
        }
    }

    #[test]
    fn list_returns_empty_when_store_file_is_missing() {
        let tempdir = tempfile::tempdir().expect("create temp dir");
        let store = ProfileStore::new(tempdir.path().join("profiles.json"));

        let profiles = store.list().expect("list profiles");

        assert!(profiles.is_empty());
    }

    #[test]
    fn create_update_delete_roundtrip_updates_store_file() {
        let tempdir = tempfile::tempdir().expect("create temp dir");
        let path = tempdir.path().join("profiles.json");
        let store = ProfileStore::new(path.clone());

        let created = store
            .create(sample_profile("night-mode"))
            .expect("create profile");
        assert!(path.exists());
        assert_eq!(created.id, "night-mode");

        let mut updated_profile = created.clone();
        updated_profile.name = "Night Shift".to_string();
        updated_profile.metadata.updated_at = "2026-03-13T13:00:00Z".to_string();
        store
            .update("night-mode", updated_profile.clone())
            .expect("update profile");

        let listed = store.list().expect("list profiles after update");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Night Shift");

        store.delete("night-mode").expect("delete profile");
        assert!(store.list().expect("list profiles after delete").is_empty());
    }

    #[test]
    fn list_returns_internal_error_for_invalid_json() {
        let tempdir = tempfile::tempdir().expect("create temp dir");
        let path = tempdir.path().join("profiles.json");
        fs::write(&path, "{not-valid-json").expect("write malformed json");
        let store = ProfileStore::new(path.clone());

        let err = store.list().expect_err("expected parse failure");

        match err {
            ApiError::Internal(message) => {
                assert!(message.contains("failed to parse profile store"));
                assert!(message.contains(&path.display().to_string()));
            }
            other => panic!("expected internal error, got {other:?}"),
        }
    }

    #[test]
    fn update_and_delete_return_not_found_for_unknown_profile_ids() {
        let tempdir = tempfile::tempdir().expect("create temp dir");
        let store = ProfileStore::new(tempdir.path().join("profiles.json"));

        let update_err = store
            .update("missing", sample_profile("missing"))
            .expect_err("update should fail");
        match update_err {
            ApiError::NotFound(message) => assert_eq!(message, "unknown profile id: missing"),
            other => panic!("expected not found error, got {other:?}"),
        }

        let delete_err = store.delete("missing").expect_err("delete should fail");
        match delete_err {
            ApiError::NotFound(message) => assert_eq!(message, "unknown profile id: missing"),
            other => panic!("expected not found error, got {other:?}"),
        }
    }

    #[test]
    fn create_rejects_duplicate_profile_ids() {
        let tempdir = tempfile::tempdir().expect("create temp dir");
        let store = ProfileStore::new(tempdir.path().join("profiles.json"));
        store
            .create(sample_profile("night-mode"))
            .expect("create initial profile");

        let err = store
            .create(sample_profile("night-mode"))
            .expect_err("duplicate create should fail");

        match err {
            ApiError::BadRequest(message) => {
                assert_eq!(message, "profile id 'night-mode' already exists")
            }
            other => panic!("expected bad request error, got {other:?}"),
        }
    }
}
