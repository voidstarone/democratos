//! Installs the process-wide `tracing` subscriber.
//!
//! `app` and the media-safety / federation adapters record operator-critical
//! events through the `tracing` facade — notably the legally-required
//! CSAM-preservation alert in [`app`]'s `escalate_to_operator`. With no
//! subscriber installed those records are dropped on the floor, so the alerts
//! never reach an operator. This wires the facade to an append-only log file so
//! every event lands somewhere durable.
//!
//! The file path defaults to `democratos.log` in the working directory; override
//! it with `DEMOCRATOS_LOG_FILE`. The level defaults to `info`; override it with
//! the standard `RUST_LOG` filter syntax.

use std::fs::OpenOptions;
use std::sync::Mutex;

use anyhow::{anyhow, Context, Result};
use tracing_subscriber::EnvFilter;

pub fn init_logging() -> Result<()> {
    let path = std::env::var("DEMOCRATOS_LOG_FILE")
        .unwrap_or_else(|_| "democratos.log".to_string());

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening log file {path}"))?;

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // `Mutex<File>` serialises concurrent writers so events from parallel tasks
    // never interleave mid-line. ANSI colour codes are off — this is a file, not
    // a terminal.
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(Mutex::new(file))
        .try_init()
        .map_err(|e| anyhow!("installing tracing subscriber: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::init_logging;

    // Proves the facade→file path end to end: install the subscriber against a
    // temp file, emit an event, and confirm it was written. A test runs in a
    // fresh process, so the global `try_init` succeeds here.
    #[test]
    fn writes_events_to_the_configured_file() {
        let path = std::env::temp_dir().join("democratos-init-logging-test.log");
        let _ = std::fs::remove_file(&path);
        std::env::set_var("DEMOCRATOS_LOG_FILE", &path);
        std::env::set_var("RUST_LOG", "info");

        init_logging().expect("subscriber installs");
        tracing::error!(marker = "csam-preservation", "operator alert");

        let contents = std::fs::read_to_string(&path).expect("log file exists");
        assert!(
            contents.contains("operator alert") && contents.contains("csam-preservation"),
            "event not written to log file; got: {contents:?}"
        );
    }
}
