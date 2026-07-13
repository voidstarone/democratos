//! Filesystem quarantine: preserves blocked media out of the public store.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use app::{MediaError, MediaQuarantine};

/// A [`MediaQuarantine`] that writes blocked bytes to a restricted directory the
/// public store and CDN never read from, and appends an incident record to a log.
///
/// This exists to *preserve* — not bin — content the pipeline refuses, because a
/// provider aware of apparent CSAM must retain it for a NCMEC CyberTipline report
/// (18 U.S.C. §2258A) rather than destroy it. Files are written with owner-only
/// permissions; the directory itself should live on storage only trusted operators
/// can reach. Nothing here ever serves or indexes what it holds.
pub struct DirQuarantine {
    dir: PathBuf,
}

impl DirQuarantine {
    /// Open (creating if absent) the quarantine directory, tightening it to
    /// owner-only access where the platform supports it.
    pub fn new(dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        restrict_permissions(&dir, 0o700)?;
        Ok(Self { dir })
    }

    fn incident_log(&self) -> PathBuf {
        self.dir.join("incidents.log")
    }
}

#[async_trait]
impl MediaQuarantine for DirQuarantine {
    async fn preserve(
        &self,
        content_type: &str,
        bytes: &[u8],
        reason: &str,
    ) -> Result<String, MediaError> {
        // Content-address the held file so identical blocked uploads coalesce and
        // the id is stable across reports.
        let digest = Sha256::digest(bytes);
        let id: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        let ext = app::extension_for(content_type).unwrap_or("bin");
        let path = self.dir.join(format!("{id}.{ext}"));

        if !path.exists() {
            std::fs::write(&path, bytes)
                .map_err(|e| MediaError::Store(format!("quarantine write: {e}")))?;
            restrict_permissions(&path, 0o600)
                .map_err(|e| MediaError::Store(format!("quarantine chmod: {e}")))?;
        }

        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let record = format!(
            "{{\"unix_time\":{secs},\"id\":\"{id}\",\"content_type\":\"{}\",\"reason\":\"{}\",\"bytes\":{}}}\n",
            json_escape(content_type),
            json_escape(reason),
            bytes.len(),
        );
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.incident_log())
            .map_err(|e| MediaError::Store(format!("quarantine log open: {e}")))?;
        log.write_all(record.as_bytes())
            .map_err(|e| MediaError::Store(format!("quarantine log write: {e}")))?;

        Ok(id)
    }
}

/// Minimal JSON string escaping for the fields we write (control-free ASCII in
/// practice, but be safe against quotes/backslashes).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

#[cfg(unix)]
fn restrict_permissions(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path, _mode: u32) -> std::io::Result<()> {
    // No portable owner-only mode elsewhere; rely on the directory's ACLs.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn preserves_bytes_and_writes_an_incident() {
        let dir = std::env::temp_dir().join(format!("dq-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let q = DirQuarantine::new(&dir).unwrap();

        let id = q
            .preserve("image/png", b"blocked-bytes", "safety-match:test:sha256")
            .await
            .unwrap();
        assert_eq!(id.len(), 64);
        assert!(dir.join(format!("{id}.png")).exists());

        let log = std::fs::read_to_string(dir.join("incidents.log")).unwrap();
        assert!(log.contains(&id));
        assert!(log.contains("safety-match:test:sha256"));

        // Idempotent: preserving the same bytes again doesn't duplicate the file.
        let id2 = q.preserve("image/png", b"blocked-bytes", "again").await.unwrap();
        assert_eq!(id, id2);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
