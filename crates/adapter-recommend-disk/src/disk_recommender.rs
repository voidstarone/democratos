//! A [`SimilarityIndex`] that serves neighbours from a file on disk.

use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use app::{Result, SimilarityIndex, StoreError};
use domain::{ItemIndex, PostId, Rating};

/// Bytes per serialised neighbour: `u64` post id + `f32` similarity.
const ENTRY_BYTES: usize = 12;

fn store_err(e: impl std::fmt::Display) -> StoreError {
    StoreError::Store(e.to_string())
}

/// Where a post's neighbour blob lives in the file.
#[derive(Clone, Copy)]
struct Span {
    offset: u64,
    len: u32,
}

struct State {
    version: u64,
    /// `None` until the first rebuild has produced a file.
    file: Option<Arc<File>>,
    offsets: std::collections::HashMap<PostId, Span>,
}

/// A [`SimilarityIndex`] that serves neighbours from a file on disk.
pub struct DiskRecommender {
    neighbours_per_post: usize,
    path: PathBuf,
    state: RwLock<State>,
}

impl DiskRecommender {
    /// Create a recommender backed by the file at `path` (created/overwritten on
    /// the first rebuild), keeping `neighbours_per_post` neighbours per post.
    pub fn new(path: impl Into<PathBuf>, neighbours_per_post: usize) -> Self {
        Self {
            neighbours_per_post,
            path: path.into(),
            state: RwLock::new(State {
                version: 0,
                file: None,
                offsets: std::collections::HashMap::new(),
            }),
        }
    }

    /// Create one with the default neighbour count.
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self::new(path, crate::DEFAULT_NEIGHBOURS)
    }

    fn tmp_path(&self) -> PathBuf {
        let mut s = self.path.clone().into_os_string();
        s.push(".tmp");
        PathBuf::from(s)
    }
}

/// Serialise `index` to `path` and return the offset table. Writes to a temp
/// file then renames, so a crash mid-write can't leave a torn index in place.
fn write_index(
    index: &ItemIndex,
    path: &Path,
    tmp: &Path,
) -> std::io::Result<std::collections::HashMap<PostId, Span>> {
    let mut offsets = std::collections::HashMap::with_capacity(index.len());
    let mut file = File::create(tmp)?;
    let mut cursor: u64 = 0;
    // A reusable buffer keeps per-entry allocation out of the hot loop.
    let mut buf: Vec<u8> = Vec::new();
    for (post, neighbours) in index.entries() {
        buf.clear();
        buf.extend_from_slice(&(neighbours.len() as u32).to_le_bytes());
        for &(p, sim) in neighbours {
            buf.extend_from_slice(&p.0.to_le_bytes());
            buf.extend_from_slice(&sim.to_le_bytes());
        }
        file.write_all(&buf)?;
        offsets.insert(
            post,
            Span {
                offset: cursor,
                len: buf.len() as u32,
            },
        );
        cursor += buf.len() as u64;
    }
    file.sync_all()?;
    fs::rename(tmp, path)?;
    Ok(offsets)
}

fn parse_entry(bytes: &[u8]) -> Vec<(PostId, f32)> {
    if bytes.len() < 4 {
        return Vec::new();
    }
    let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    // Never reserve more than the remaining bytes could possibly fill: a corrupt
    // or tampered header claiming billions of entries must not drive a multi-GB
    // allocation (OOM/abort). The read loop below is already bounds-checked.
    let cap = count.min(bytes.len().saturating_sub(4) / ENTRY_BYTES);
    let mut out = Vec::with_capacity(cap);
    for i in 0..count {
        let base = 4 + i * ENTRY_BYTES;
        if base + ENTRY_BYTES > bytes.len() {
            break;
        }
        let id = u64::from_le_bytes(bytes[base..base + 8].try_into().unwrap());
        let sim = f32::from_le_bytes(bytes[base + 8..base + 12].try_into().unwrap());
        out.push((PostId(id), sim));
    }
    out
}

#[async_trait]
impl SimilarityIndex for DiskRecommender {
    async fn rebuild(&self, version: u64, ratings: Vec<Rating>) -> Result<()> {
        let index = ItemIndex::build(&ratings, self.neighbours_per_post);
        let offsets = write_index(&index, &self.path, &self.tmp_path()).map_err(store_err)?;
        let file = File::open(&self.path).map_err(store_err)?;

        let mut state = self.state.write().expect("recommender lock poisoned");
        state.version = version;
        state.offsets = offsets;
        state.file = Some(Arc::new(file));
        Ok(())
    }

    async fn version(&self) -> u64 {
        self.state
            .read()
            .expect("recommender lock poisoned")
            .version
    }

    async fn neighbours(&self, post: PostId) -> Result<Vec<(PostId, f32)>> {
        // Hold the read lock only long enough to copy the span + file handle.
        let (span, file) = {
            let state = self.state.read().expect("recommender lock poisoned");
            match (state.offsets.get(&post), state.file.as_ref()) {
                (Some(span), Some(file)) => (*span, Arc::clone(file)),
                _ => return Ok(Vec::new()),
            }
        };
        let mut bytes = vec![0u8; span.len as usize];
        file.read_exact_at(&mut bytes, span.offset)
            .map_err(store_err)?;
        Ok(parse_entry(&bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::UserId;

    fn ratings() -> Vec<Rating> {
        vec![
            Rating::from_vote(UserId(1), PostId(1), true),
            Rating::from_vote(UserId(1), PostId(2), true),
            Rating::from_vote(UserId(2), PostId(1), true),
            Rating::from_vote(UserId(2), PostId(2), true),
            Rating::from_vote(UserId(2), PostId(3), false),
        ]
    }

    #[tokio::test]
    async fn round_trips_through_disk() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("recidx-test-{}.bin", std::process::id()));
        let _ = fs::remove_file(&path);

        let rec = DiskRecommender::open(&path);
        assert_eq!(rec.version().await, 0);
        assert!(rec.neighbours(PostId(1)).await.unwrap().is_empty()); // no file yet

        rec.rebuild(5, ratings()).await.unwrap();
        assert_eq!(rec.version().await, 5);

        let n1 = rec.neighbours(PostId(1)).await.unwrap();
        assert!(n1.iter().any(|(p, _)| *p == PostId(2)));
        for (_, sim) in &n1 {
            assert!(*sim > 0.0 && *sim <= 1.0 + 1e-6);
        }
        // Unknown post → empty, not an error.
        assert!(rec.neighbours(PostId(999)).await.unwrap().is_empty());

        let _ = fs::remove_file(&path);
    }
}
