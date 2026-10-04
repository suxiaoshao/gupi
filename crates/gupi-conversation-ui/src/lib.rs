//! Gupi internal conversation-ui capability.
pub mod chrome;
pub mod command_palette;
pub mod composer;
pub mod home;
pub mod host;
#[cfg(feature = "performance")]
pub mod performance;

mod tool_icon;
