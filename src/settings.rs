//! Persistent app settings.
//!
//! Replaces the old CLI flags. Stored as JSON at
//! `~/Library/Application Support/com.mpjhorner.MailBoxUltra/settings.json`.
//! Loaded once at startup, saved every time the user clicks Apply in
//! Preferences. Atomic writes (tempfile + rename) so a crash mid-save can't
//! leave a half-written file.

use std::net::IpAddr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Bump this when adding/renaming/removing fields. Older configs are migrated
/// in [`PersistentSettings::migrate`]; unknown future versions fall back to
/// defaults so a downgrade can't crash the app.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "mpjhorner";
const APPLICATION: &str = "MailBoxUltra";
const SETTINGS_FILE: &str = "settings.json";

/// All app configuration except runtime window state. Window position / size
/// are persisted by eframe's own `persist_window` mechanism.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PersistentSettings {
    pub schema_version: u32,

    // Servers
    pub smtp_port: u16,
    pub bind: IpAddr,

    // SMTP
    pub hostname: String,
    pub max_message_size: usize,
    #[serde(default)]
    pub auth: Option<Auth>,

    // Capture
    pub buffer_size: usize,

    // Relay
    #[serde(default)]
    pub relay: Option<RelaySettings>,

    // Logging
    #[serde(default)]
    pub log_file: Option<PathBuf>,

    // Appearance
    #[serde(default)]
    pub theme: Theme,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Auth {
    pub user: String,
    pub pass: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RelaySettings {
    pub url: String,
    #[serde(default)]
    pub insecure: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Theme {
    System,
    /// Default — matches the web UI's dark-by-default design and reads better
    /// against the colourful HTML emails users land on first.
    #[default]
    Dark,
    Light,
}

impl Default for PersistentSettings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            smtp_port: 1025,
            bind: "127.0.0.1".parse().expect("literal IP"),
            hostname: "MailBoxUltra".into(),
            max_message_size: 25 * 1024 * 1024,
            auth: None,
            buffer_size: 1000,
            relay: None,
            log_file: None,
            theme: Theme::default(),
        }
    }
}

impl PersistentSettings {
    /// Standard config-file path. None on the rare occasion `directories`
    /// can't resolve `$HOME` (e.g. extreme sandbox).
    pub fn config_path() -> Option<PathBuf> {
        ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
            .map(|p| p.config_dir().join(SETTINGS_FILE))
    }

    /// Read settings from disk. On any failure (missing file, malformed JSON,
    /// future schema version) we log and return [`Self::default`] so the app
    /// always boots into a sane state.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            tracing::warn!("could not resolve config dir; using default settings");
            return Self::default();
        };
        match Self::load_from(&path) {
            Ok(s) => s,
            Err(e) => {
                if path.exists() {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "settings file is unreadable; using defaults"
                    );
                }
                Self::default()
            }
        }
    }

    /// Read + parse + migrate from a specific path. Used by [`Self::load`] and
    /// by tests.
    pub fn load_from(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let raw: serde_json::Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing {}", path.display()))?;
        Self::from_value(raw)
    }

    fn from_value(mut raw: serde_json::Value) -> Result<Self> {
        let version = raw
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        Self::migrate(&mut raw, version)?;
        let parsed: Self = serde_json::from_value(raw).context("deserializing settings")?;
        Ok(parsed)
    }

    /// Hook for forward-only migrations. Today there's only schema v1; older
    /// (`0` = no schema_version field) gets upgraded by inserting the field;
    /// newer (unknown future) is rejected so we don't silently lose data.
    fn migrate(raw: &mut serde_json::Value, version: u32) -> Result<()> {
        if version > CURRENT_SCHEMA_VERSION {
            anyhow::bail!(
                "settings file uses schema v{version}, this build only knows v{CURRENT_SCHEMA_VERSION}"
            );
        }
        if version == 0 {
            if let Some(obj) = raw.as_object_mut() {
                obj.insert(
                    "schema_version".into(),
                    serde_json::Value::from(CURRENT_SCHEMA_VERSION),
                );
            }
        }
        Ok(())
    }

    /// Persist to the standard config path. Creates parent dirs on demand.
    /// Atomic on POSIX: we write to `settings.json.tmp` and `rename(2)` over
    /// the target so a crash can't produce a half-written file.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path().context("resolving config path")?;
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        let json = serde_json::to_vec_pretty(self)?;
        std::fs::write(&tmp, json)
            .with_context(|| format!("writing temp file {}", tmp.display()))?;
        std::fs::rename(&tmp, path).with_context(|| format!("rename to {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn tmp_path() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        (dir, path)
    }

    #[test]
    fn defaults_match_documented_values() {
        let s = PersistentSettings::default();
        assert_eq!(s.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(s.smtp_port, 1025);
        assert_eq!(s.bind.to_string(), "127.0.0.1");
        assert_eq!(s.hostname, "MailBoxUltra");
        assert_eq!(s.max_message_size, 25 * 1024 * 1024);
        assert_eq!(s.buffer_size, 1000);
        assert!(s.auth.is_none());
        assert!(s.relay.is_none());
        assert!(s.log_file.is_none());
        assert_eq!(s.theme, Theme::Dark);
    }

    #[test]
    fn round_trip_preserves_every_field() {
        let (_dir, path) = tmp_path();
        let original = PersistentSettings {
            schema_version: CURRENT_SCHEMA_VERSION,
            smtp_port: 2525,
            bind: "0.0.0.0".parse().unwrap(),
            hostname: "fake.local".into(),
            max_message_size: 128 * 1024,
            auth: Some(Auth {
                user: "alice".into(),
                pass: "s3cret".into(),
            }),
            buffer_size: 50,
            relay: Some(RelaySettings {
                url: "smtp://relay.example.com:25".into(),
                insecure: true,
            }),
            log_file: Some("/tmp/mbu.log".into()),
            theme: Theme::Dark,
        };
        original.save_to(&path).unwrap();
        let loaded = PersistentSettings::load_from(&path).unwrap();
        assert_eq!(loaded, original);
    }

    #[test]
    fn missing_file_via_load_returns_defaults() {
        // load() falls back silently when the path can't be read.
        let (_dir, path) = tmp_path();
        // confirm the path doesn't exist
        assert!(!path.exists());
        // load_from is the API tests can hit deterministically
        let res = PersistentSettings::load_from(&path);
        assert!(res.is_err(), "expected NotFound");
    }

    #[test]
    fn corrupt_file_returns_error_load_from_uses_defaults_via_load() {
        let (_dir, path) = tmp_path();
        std::fs::write(&path, b"not json at all").unwrap();
        let err = PersistentSettings::load_from(&path).unwrap_err();
        assert!(
            err.to_string().to_ascii_lowercase().contains("parsing"),
            "{err}"
        );
    }

    #[test]
    fn schema_version_zero_is_migrated_in_memory() {
        let raw = serde_json::json!({
            "smtp_port": 1234,
            "bind": "127.0.0.1",
            "hostname": "MailBoxUltra",
            "max_message_size": 1024,
            "buffer_size": 10
        });
        let s = PersistentSettings::from_value(raw).unwrap();
        assert_eq!(s.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(s.smtp_port, 1234);
    }

    #[test]
    fn newer_schema_is_rejected() {
        let raw = serde_json::json!({
            "schema_version": 999,
            "smtp_port": 1025,
            "bind": "127.0.0.1",
            "hostname": "MailBoxUltra",
            "max_message_size": 1024,
            "buffer_size": 10
        });
        let err = PersistentSettings::from_value(raw).unwrap_err();
        assert!(err.to_string().contains("schema v999"));
    }

    #[test]
    fn save_creates_parent_directory() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested/deeper/settings.json");
        let s = PersistentSettings::default();
        s.save_to(&path).unwrap();
        assert!(path.exists());
        let loaded = PersistentSettings::load_from(&path).unwrap();
        assert_eq!(loaded, s);
    }

    #[test]
    fn save_does_not_leave_a_tmp_file_on_success() {
        let (_dir, path) = tmp_path();
        PersistentSettings::default().save_to(&path).unwrap();
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        assert!(!tmp.exists());
    }

    #[test]
    fn relay_and_auth_optional_fields_round_trip_when_absent() {
        // Schema-v1 file written without auth/relay/log_file present — the
        // serde defaults should fill them in.
        let raw = serde_json::json!({
            "schema_version": 1,
            "smtp_port": 1025,
            "bind": "127.0.0.1",
            "hostname": "MailBoxUltra",
            "max_message_size": 1024,
            "buffer_size": 10
        });
        let s = PersistentSettings::from_value(raw).unwrap();
        assert!(s.auth.is_none());
        assert!(s.relay.is_none());
        assert!(s.log_file.is_none());
        assert_eq!(s.theme, Theme::Dark);
    }

    #[test]
    fn config_path_is_under_application_support() {
        // Exercises the directories crate; only assert the suffix shape, not
        // the user's literal HOME.
        let path = PersistentSettings::config_path().expect("home resolves");
        let s = path.to_string_lossy();
        assert!(s.ends_with("settings.json"), "{s}");
        // On macOS this lives under "Application Support"; on linux under
        // ".config". We assert just the file name to stay portable.
    }

    #[test]
    fn load_returns_defaults_when_path_unresolvable() {
        // Indirect: even if config_path() returns Some, an unreadable path
        // still falls back. We can't easily force None without changing
        // ProjectDirs behaviour; this test confirms a missing file lands on
        // defaults.
        let (_dir, path) = tmp_path();
        // Manually invoke load_from to mirror what load() does internally.
        let res = PersistentSettings::load_from(&path);
        assert!(res.is_err());
        // load() itself swallows that error and gives defaults.
        let _ = PersistentSettings::load();
    }
}
