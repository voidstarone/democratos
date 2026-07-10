//! Where uploaded media is stored.

use clap::ValueEnum;

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum MediaKind {
    /// Uploaded bytes on this node's local disk. Single-box: an upload accepted
    /// here is invisible to other nodes.
    Local,
    /// Shared S3/MinIO bucket. The scalable media store: every node reads and
    /// writes one bucket, so any node serves any upload. Requires `--s3-*`.
    S3,
    /// Host NO media on this node — opt out of media storage entirely. Uploads are
    /// refused and media URLs 404; text/governance are unaffected. For a small,
    /// slow, storage-light box. To still *display* media, use `s3` pointed at
    /// another box's shared bucket instead.
    None,
}
