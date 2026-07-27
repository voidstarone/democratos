//! The largest single upload the server accepts.

/// The largest single media upload the server accepts, in bytes (25 MB).
///
/// The delivery layer enforces this **while streaming** a multipart field rather
/// than after buffering it, so one oversized file cannot push more than this into
/// memory before it is refused. Together with
/// `MAX_ATTACHMENTS` it also sets the request body cap
/// (`MAX_UPLOAD_BYTES * MAX_ATTACHMENTS + 1 MB` of form overhead), which bounds
/// what a single request can cost the box — the reason this is a `usize`: it is
/// compared against buffer lengths and fed to the body-limit layer.
pub const MAX_UPLOAD_BYTES: usize = 25 * 1024 * 1024;
