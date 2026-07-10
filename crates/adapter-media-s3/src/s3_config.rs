//! Connection settings for the shared S3/MinIO bucket.

/// How to reach the shared bucket.
pub struct S3Config {
    /// Bucket name, e.g. `democratos-media`.
    pub bucket: String,
    /// Region label. MinIO ignores it but still requires one — `us-east-1` is
    /// the conventional placeholder.
    pub region: String,
    /// Endpoint URL, e.g. `http://minio:9000` (MinIO) or
    /// `https://s3.us-east-1.amazonaws.com` (AWS).
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    /// Path-style addressing (`endpoint/bucket/key`) instead of virtual-host
    /// (`bucket.endpoint/key`). MinIO needs `true`; AWS works either way.
    pub uses_path_style: bool,
    /// If set, media is served directly from this base URL (CDN/public bucket):
    /// `put` returns `<public_base>/<key>` and `get` returns `None`. If `None`,
    /// the app proxies media through its own `/media/:key` route.
    pub public_base: Option<String>,
}
