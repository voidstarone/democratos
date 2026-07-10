//! Boot banner: print, unmistakably, where data is going.

use app::Services;

use crate::store_kind::StoreKind;

/// Print, unmistakably, where data is going. The most common confusion is
/// running with the in-memory store and losing everything on exit.
pub(crate) async fn report_storage(
    kind: StoreKind,
    data: &str,
    media_dir: &str,
    services: &Services,
) {
    // Communities the node knows about, for the boot banner.
    let community_slugs = || async {
        services
            .demoi
            .list()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|d| format!("d/{}", d.slug))
            .collect::<Vec<_>>()
    };
    match kind {
        StoreKind::Postgres => {
            let slugs = community_slugs().await;
            eprintln!("storage: postgres — shared database (horizontally scalable)");
            eprintln!("   media uploads → {}/", absolute_path(media_dir).display());
            if slugs.is_empty() {
                eprintln!("   no communities yet — create one from the home page once signed in.");
            } else {
                eprintln!(
                    "   {} communit{} on this node: {}",
                    slugs.len(),
                    if slugs.len() == 1 { "y" } else { "ies" },
                    slugs.join(", ")
                );
            }
        }
        StoreKind::Memory => {
            eprintln!("⚠  storage: IN-MEMORY — nothing is saved; all data is lost when this process exits.");
            eprintln!("   To persist, run without `--store memory` (the default `file` store writes to disk).");
        }
        StoreKind::File => {
            let path = absolute_path(data);
            let slugs: Vec<String> = services
                .demoi
                .list()
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|d| format!("d/{}", d.slug))
                .collect();
            eprintln!("storage: file — saving to {}", path.display());
            eprintln!("   media uploads → {}/", absolute_path(media_dir).display());
            if slugs.is_empty() {
                eprintln!("   no communities yet — create one from the home page once signed in.");
            } else {
                eprintln!(
                    "   loaded {} communit{}: {}",
                    slugs.len(),
                    if slugs.len() == 1 { "y" } else { "ies" },
                    slugs.join(", ")
                );
            }
        }
    }
}

/// Resolve `data` to an absolute path for display, without requiring the file
/// to exist yet.
fn absolute_path(data: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(data);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|d| d.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    }
}
