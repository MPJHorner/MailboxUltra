//! Integration coverage for the persistent settings API. Mirrors the lib
//! tests but goes through the public surface (`load_from`, `save_to`) to
//! catch breakage in serde derives or pub visibility.

use std::path::PathBuf;

use mailbox_ultra::settings::{
    Auth, PersistentSettings, RelaySettings, Theme, CURRENT_SCHEMA_VERSION,
};
use tempfile::TempDir;

fn tmp_path() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    (dir, path)
}

#[test]
fn external_round_trip_is_lossless() {
    let (_dir, path) = tmp_path();
    let original = PersistentSettings {
        schema_version: CURRENT_SCHEMA_VERSION,
        smtp_port: 0,
        bind: "127.0.0.1".parse().unwrap(),
        hostname: "test.example".into(),
        max_message_size: 4096,
        auth: Some(Auth {
            user: "user".into(),
            pass: "pass".into(),
        }),
        buffer_size: 5,
        relay: Some(RelaySettings {
            url: "smtps://relay.example.com:465".into(),
            insecure: false,
        }),
        log_file: Some(PathBuf::from("/var/log/mbu.log")),
        theme: Theme::Light,
    };
    original.save_to(&path).unwrap();
    let read = PersistentSettings::load_from(&path).unwrap();
    assert_eq!(read, original);
}

#[test]
fn json_is_pretty_and_readable() {
    let (_dir, path) = tmp_path();
    PersistentSettings::default().save_to(&path).unwrap();
    let body = std::fs::read_to_string(&path).unwrap();
    // Pretty-printing produces multiline JSON; not a binary blob.
    assert!(body.contains('\n'), "expected pretty-printed JSON");
    assert!(body.contains("\"schema_version\""));
    assert!(body.contains("\"smtp_port\""));
}

#[test]
fn legacy_file_without_schema_version_is_accepted() {
    let (_dir, path) = tmp_path();
    let body = serde_json::json!({
        "smtp_port": 1025,
        "bind": "127.0.0.1",
        "hostname": "MailBoxUltra",
        "max_message_size": 1024,
        "buffer_size": 10
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&body).unwrap()).unwrap();
    let s = PersistentSettings::load_from(&path).unwrap();
    assert_eq!(s.schema_version, CURRENT_SCHEMA_VERSION);
}
