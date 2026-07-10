//! Command-line driving adapter. Every subcommand maps to an `app::Services`
//! use-case — the very same calls the web handlers make. Run against the
//! text-file store, these commands operate on the same data a `serve` instance
//! would, demonstrating that delivery mechanism and storage are independent.
//!
//! One definition per file: the subcommand set and the dispatcher live in their
//! own leaf modules and are re-exported flat here.

pub mod command;
pub mod dispatch;

pub use command::Command;
pub use dispatch::dispatch;
