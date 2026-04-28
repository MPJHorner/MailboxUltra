//! MailBox Ultra: a local SMTP fake inbox library + binary.
//!
//! All public modules are re-exported here so integration tests can drive the
//! servers and helpers without going through the `main` shim.

pub mod app;
pub mod assets;
pub mod cli;
pub mod entrypoint;
pub mod message;
pub mod output;
pub mod relay;
pub mod smtp;
pub mod store;
pub mod ui;
pub mod update;
