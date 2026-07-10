//! S3/MinIO media roundtrip against a live bucket. Gated on `TEST_S3_ENDPOINT`
//! (e.g. `http://127.0.0.1:59000`); `TEST_S3_ACCESS_KEY` / `TEST_S3_SECRET_KEY`
//! default to MinIO's `minioadmin`/`minioadmin`.
//!
//! Run a MinIO for it with:
//!   docker run -d --name minio -p 59000:9000 \
//!     -e MINIO_ROOT_USER=minioadmin -e MINIO_ROOT_PASSWORD=minioadmin \
//!     minio/minio server /data

use adapter_media_s3::{S3Config, S3MediaStore};
use app::MediaStore;

fn config(bucket: &str, public_base: Option<String>) -> Option<S3Config> {
    let endpoint = std::env::var("TEST_S3_ENDPOINT").ok()?;
    Some(S3Config {
        bucket: bucket.to_string(),
        region: "us-east-1".to_string(),
        endpoint,
        access_key: std::env::var("TEST_S3_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".into()),
        secret_key: std::env::var("TEST_S3_SECRET_KEY").unwrap_or_else(|_| "minioadmin".into()),
        uses_path_style: true,
        public_base,
    })
}

// A 1x1 PNG is not needed — media_key hashes the bytes, not the pixels; any bytes
// stored under an accepted content type roundtrip.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n-fake-but-accepted-by-content-type";

#[tokio::test]
async fn proxied_put_get_roundtrips_and_dedupes() {
    let Some(cfg) = config("democratos-media-test", None) else {
        eprintln!("skipping: TEST_S3_ENDPOINT not set");
        return;
    };
    let store = S3MediaStore::new(cfg).expect("build store");
    store.ensure_bucket().await.expect("create bucket");

    // put → a proxied /media/<key> URL.
    let url = store.put("image/png", PNG.to_vec()).await.expect("put");
    assert!(url.starts_with("/media/"), "proxied URL, got {url}");
    let key = url.strip_prefix("/media/").unwrap();

    // get → the exact bytes and a sensible content type.
    let (ct, bytes) = store.get(key).await.expect("get").expect("present");
    assert_eq!(ct, "image/png");
    assert_eq!(bytes, PNG);

    // Content addressing: an identical upload returns the same key (dedupe).
    let url2 = store
        .put("image/png", PNG.to_vec())
        .await
        .expect("put again");
    assert_eq!(url, url2, "identical bytes dedupe to one object");

    // A missing key is absence, not an error.
    assert!(store
        .get("00000000deadbeef.png")
        .await
        .expect("get missing")
        .is_none());
    // A traversal-ish key is refused as absent.
    assert!(store.get("../secret").await.expect("bad key").is_none());
}

#[tokio::test]
async fn direct_mode_returns_a_public_url_and_does_not_proxy() {
    let Some(cfg) = config(
        "democratos-media-test",
        Some("https://cdn.example/m".into()),
    ) else {
        eprintln!("skipping: TEST_S3_ENDPOINT not set");
        return;
    };
    let store = S3MediaStore::new(cfg).expect("build store");
    store.ensure_bucket().await.expect("create bucket");

    let url = store.put("image/png", PNG.to_vec()).await.expect("put");
    assert!(
        url.starts_with("https://cdn.example/m/"),
        "CDN URL, got {url}"
    );

    // In direct mode the app never proxies bytes — get() is a no-op absence.
    let key = url.rsplit('/').next().unwrap();
    assert!(
        store.get(key).await.expect("get").is_none(),
        "direct mode does not proxy"
    );
}
