//! MailBox Ultra: a native macOS SMTP fake-inbox app.
//!
//! The library exposes the protocol + storage core (`smtp`, `store`, `relay`,
//! `message`) so integration tests can drive it without going through the
//! GUI. The GUI itself lives in `src/main.rs` + `src/gui/` and is not part of
//! the library API.

pub mod message;
pub mod relay;
pub mod settings;
pub mod smtp;
pub mod store;
