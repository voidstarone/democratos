//! Handle the offline/registry `issuer` subcommands.

use anyhow::{anyhow, Context, Result};

use adapter_control_etcd::EtcdRegistry;
use domain::NodeId;
use federation::{IssuerCert, IssuerRootKeypair, OwnershipRegistry};

use crate::issuer_command::IssuerCommand;

/// Run an `issuer` subcommand. `node_id` is this process's node id, used only when
/// `Publish` connects to the control plane.
pub(crate) async fn run_issuer(command: IssuerCommand, node_id: u16) -> Result<()> {
    match command {
        IssuerCommand::Root => {
            let root = IssuerRootKeypair::generate();
            println!("federation trust root generated:\n");
            println!("  SECRET seed (store OFFLINE, never on a node):");
            println!("    FEDERATION_ROOT_SEED={}", root.seed_hex());
            println!("\n  PUBLIC key (set on every node):");
            println!("    FEDERATION_TRUST_ROOT={}", root.public().to_hex());
        }
        IssuerCommand::Certify {
            node,
            epoch,
            root_seed,
        } => {
            let root = IssuerRootKeypair::from_seed_hex(root_seed.trim())
                .map_err(|e| anyhow!("invalid root seed: {e}"))?;
            let cert = root.certify(node, epoch);
            let json = serde_json::to_string(&cert).context("serialize issuer cert")?;
            eprintln!("certified node {node} as a trusted issuer at epoch {epoch}. Cert JSON:");
            // The cert itself goes to stdout so it can be piped to `issuer publish`.
            println!("{json}");
        }
        IssuerCommand::Publish {
            cert,
            cert_file,
            etcd_endpoints,
        } => {
            let json = match (cert, cert_file) {
                (Some(_), Some(_)) => {
                    return Err(anyhow!("pass only one of --cert or --cert-file"))
                }
                (Some(inline), None) => inline,
                (None, Some(path)) => {
                    std::fs::read_to_string(&path).with_context(|| format!("read {path}"))?
                }
                (None, None) => return Err(anyhow!("provide the cert via --cert or --cert-file")),
            };
            let cert: IssuerCert =
                serde_json::from_str(json.trim()).context("parse issuer cert JSON")?;
            let endpoints: Vec<String> = etcd_endpoints
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            if endpoints.is_empty() {
                return Err(anyhow!(
                    "publishing needs --etcd-endpoints (DEMOCRATOS_ETCD_ENDPOINTS)"
                ));
            }
            // `connect` loads FEDERATION_TRUST_ROOT; `set_issuer_cert` refuses a cert
            // that does not verify against it, so a bad root config fails loudly here.
            let registry = EtcdRegistry::connect(&endpoints, 15, NodeId(node_id))
                .await
                .map_err(|e| anyhow!("connect to control plane: {e}"))?;
            registry
                .set_issuer_cert(&cert)
                .await
                .map_err(|e| anyhow!("publish issuer cert: {e}"))?;
            println!(
                "published trusted-issuer cert for node {} (epoch {})",
                cert.node, cert.epoch
            );
        }
    }
    Ok(())
}
