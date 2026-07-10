//! Assemble an [`S3Config`] from the CLI.

use adapter_media_s3::S3Config;

use crate::cli::Cli;

/// Assemble an [`S3Config`] from the CLI, or `None` if the endpoint/credentials
/// are absent (in which case `--media s3` will error with a clear message).
pub(crate) fn s3_config_from(cli: &Cli) -> Option<S3Config> {
    Some(S3Config {
        bucket: cli.s3_bucket.clone(),
        region: cli.s3_region.clone(),
        endpoint: cli.s3_endpoint.clone()?,
        access_key: cli.s3_access_key.clone()?,
        secret_key: cli.s3_secret_key.clone()?,
        uses_path_style: cli.s3_path_style,
        public_base: cli.s3_public_base.clone(),
    })
}
