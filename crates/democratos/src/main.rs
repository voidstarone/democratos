//! Democratos composition root.
//!
//! This is the *only* place that knows which concrete adapters are in play.
//! `domain`, `app`, and the delivery adapters are all written against traits, so
//! the choices below — which store, which clock, web vs. CLI — are the entire
//! surface area of "swappability". Everything else is wired through ports.

mod backfill;
mod build_media_guard;
mod build_media_store;
mod build_notifier;
mod build_services;
mod cli;
mod fed;
mod init_logging;
mod issuer_command;
mod media_guard_config;
mod media_kind;
mod notifier_kind;
mod parse_admin_subnets;
mod report_storage;
mod run_issuer;
mod s3_config_from;
mod sanitizer_kind;
mod scan_policy;
mod seed;
mod spawn_recommendation_refresher;
mod store_kind;
mod system_clock;
mod top;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;

use adapter_store_postgres::PgStoreConfig;
use app::{
    AccountAuthenticator, AccountMinter, GovernanceWrites, LocalAuthenticator, LocalMinter,
    LocalWrites,
};

use crate::build_notifier::build_notifier;
use crate::build_services::build_services;
use crate::cli::Cli;
use crate::notifier_kind::NotifierKind;
use crate::parse_admin_subnets::parse_admin_subnets;
use crate::media_guard_config::MediaGuardConfig;
use crate::report_storage::report_storage;
use crate::s3_config_from::s3_config_from;
use crate::spawn_recommendation_refresher::spawn_recommendation_refresher;
use crate::top::Top;

#[tokio::main]
async fn main() -> Result<()> {
    // Wire the `tracing` facade to a log file before anything else runs, so
    // operator-critical events (e.g. the CSAM-preservation alert) are recorded
    // instead of silently dropped. See `init_logging` for the path/level knobs.
    init_logging::init_logging()?;

    let cli = Cli::parse();

    // Trusted-issuer key management is offline (root keygen / certify) or touches
    // only the control plane (publish) — none of it needs a services stack, so
    // handle it before building one. Detect by reference so the other commands can
    // still borrow `cli` for `build_services`.
    if matches!(cli.command, Top::Issuer(_)) {
        let node_id = cli.node_id;
        let Top::Issuer(command) = cli.command else { unreachable!() };
        return run_issuer::run_issuer(command, node_id).await;
    }

    // The invite-approval notifier (log or SMTP). Built here so a misconfigured
    // SMTP setup fails the boot before any services are wired.
    let notifier = build_notifier(&cli)?;

    let (services, fed_store) = build_services(
        cli.store,
        &cli.data,
        cli.database_url.as_deref(),
        cli.node_id,
        PgStoreConfig {
            max_connections: cli.db_pool_size,
            statement_timeout_ms: cli.db_statement_timeout_ms,
        },
        cli.media,
        &cli.media_dir,
        s3_config_from(&cli),
        MediaGuardConfig {
            sanitizer: cli.media_sanitizer,
            csam_scan: cli.csam_scan,
            hash_file: cli.csam_hash_file.clone(),
            policy: cli.media_scan_policy.to_app(),
            quarantine_dir: cli.quarantine_dir.clone(),
        },
        cli.recommend_index.as_deref(),
        cli.age_verification,
        cli.require_signatures,
        notifier,
        cli.public_base_url.clone(),
        cli.invite_token_ttl_days,
    )
    .await?;
    report_storage(cli.store, &cli.data, &cli.media_dir, &services).await;

    match cli.command {
        Top::Serve {
            addr,
            refresh_secs,
            dev,
            secure_cookies,
            session_secret,
            federation_addr,
            advertise_url,
            etcd_endpoints,
            cluster_token,
            peers,
            dev_unlock_secret,
            dev_accounts,
        } => {
            // Is this bind loopback-only (a local dev run) or a real, network-
            // reachable deployment? Some fail-closed checks below only bite when
            // exposed, so a bare `cargo run` stays frictionless.
            let loopback = addr.starts_with("127.")
                || addr.starts_with("localhost")
                || addr.starts_with("[::1]")
                || addr.starts_with("::1");

            // A shared session secret makes cookies valid fleet-wide — convenient,
            // but it means ANY node holding it can forge a session for any account
            // WITHOUT the password, which would bypass delegated login entirely. Warn
            // when a federated node uses one, so it is only ever shared across nodes
            // inside the same trust boundary — never handed to an untrusted community
            // node (give those their own secret, so their cookies are local to them).
            let has_shared_session_secret =
                session_secret.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
            if federation_addr.is_some() && has_shared_session_secret {
                eprintln!(
                    "⚠  federation + a shared DEMOCRATOS_SESSION_SECRET: any node holding this \
                     secret can forge a session for any account without its password. Share it \
                     ONLY across nodes in the same trust boundary; give untrusted community nodes \
                     their own secret so their sessions can't be forged fleet-wide."
                );
            }

            // Session-cookie signer: a configured secret makes sessions durable
            // and cluster-wide; its absence falls back to a secure per-process
            // key (with a warning) so a bare run is still unforgeable.
            let session = match session_secret {
                Some(secret) if !secret.is_empty() => {
                    // Fail closed on a placeholder or weak secret when exposed. A
                    // shipped default (e.g. the compose `CHANGE_ME…` value) is
                    // world-known: the session cookie is `uid.exp.HMAC(secret,…)`,
                    // so anyone with the secret forges a cookie for `uid=1` (the
                    // founder) and is instantly that account. Refuse to serve a
                    // forgeable secret on a network bind rather than run insecure.
                    let is_placeholder = secret.starts_with("CHANGE_ME");
                    let is_too_short = secret.len() < 16;
                    if is_placeholder || is_too_short {
                        if !loopback {
                            anyhow::bail!(
                                "DEMOCRATOS_SESSION_SECRET is a placeholder or too short \
                                 (< 16 chars) — it is not actually secret, so session cookies \
                                 would be forgeable and any account (including the founder) \
                                 trivially impersonated. Set a long random value, e.g. \
                                 `openssl rand -hex 32`, or unset it to use a secure \
                                 per-process key."
                            );
                        }
                        eprintln!(
                            "⚠  DEMOCRATOS_SESSION_SECRET looks like a placeholder or is very \
                             short. This is unsafe on any exposed bind — use `openssl rand -hex 32`."
                        );
                    }
                    app::SessionSigner::from_secret(secret.as_bytes())
                }
                _ => {
                    eprintln!(
                        "⚠  no --session-secret (DEMOCRATOS_SESSION_SECRET) set: using a random \
                         per-process key. Sessions reset on restart and won't verify across \
                         federated nodes. Set a shared secret in production."
                    );
                    app::SessionSigner::ephemeral()
                }
            };

            // Open federation forwards a user's governance writes to their
            // community's owner on another, untrusted node. That owner can vouch
            // for nobody — only the user's own signature proves the ballot is
            // theirs — so per-user signatures are MANDATORY whenever federation is
            // enabled, regardless of the `--require-signatures` rollout flag. A
            // key-less account simply cannot act until it enrols a key (there is no
            // node-trusted fallback for a malicious relay to exploit). This closes
            // the forwarded-vote forgery hole; the former opt-out
            // (DEMOCRATOS_ALLOW_UNSIGNED_FEDERATION) is deliberately gone.
            let mut services = services;
            if federation_addr.is_some() {
                services.require_signatures = true;
            }

            // Keep the recommendation model fresh off the request path: a
            // background task rebuilds it at boot and on an interval, so no HTTP
            // request ever pays the (potentially expensive) rebuild cost.
            spawn_recommendation_refresher(services.clone(), refresh_secs);

            // Start the federation runtime (feed server + peer puller + control
            // plane) when configured. It needs the concrete Postgres store and
            // returns the write gateway that routes votes to their owner node.
            // Without federation, votes run locally against `services`.
            let (writes, minter, authenticator): (
                Arc<dyn GovernanceWrites>,
                Arc<dyn AccountMinter>,
                Arc<dyn AccountAuthenticator>,
            ) = if let Some(federation_addr) = federation_addr {
                    // Signature enforcement is already forced on above for any federated
                    // node, so a forwarded write with no valid per-user signature is
                    // rejected by `verify_user_action` on the authoritative owner.
                    let store = fed_store
                        .clone()
                        .ok_or_else(|| anyhow::anyhow!("federation requires --store postgres"))?;
                    let peers = peers
                        .iter()
                        .map(|p| fed::parse_peer::parse_peer(p))
                        .collect::<Result<Vec<_>>>()?;
                    fed::start::start(
                        store,
                        services.clone(),
                        fed::federation_args::FederationArgs {
                            node_id: cli.node_id,
                            federation_addr,
                            advertise_url,
                            etcd_endpoints: fed::parse_endpoints::parse_endpoints(&etcd_endpoints),
                            cluster_token,
                            peers,
                            lease_ttl_secs: 15,
                            poll_interval_secs: 5,
                        },
                    )
                    .await?
                } else {
                    // Single-box: this node owns everything and is its own issuer.
                    (
                        Arc::new(LocalWrites::new(services.clone())),
                        Arc::new(LocalMinter::new(services.clone())),
                        Arc::new(LocalAuthenticator::new(services.clone())),
                    )
                };

            // Warn loudly if a non-loopback bind (i.e. a real deployment) is
            // serving the session cookie without the `Secure` flag: over plain
            // HTTP the cookie is sniffable and the session rideable.
            if !secure_cookies && !loopback {
                eprintln!(
                    "⚠  serving on {addr} without --secure-cookies \
                     (DEMOCRATOS_SECURE_COOKIES): the session cookie will be sent over plain \
                     HTTP. Enable it whenever traffic is TLS-terminated (the normal case behind \
                     the bundled Caddy)."
                );
            }

            // Dev account switcher hardening. The switcher lets a browser act as any
            // *puppet* account with no password, so who can unlock it matters.
            let dev_unlock_secret: Option<std::sync::Arc<str>> = dev_unlock_secret
                .filter(|s| !s.is_empty())
                .map(|s| s.into());
            if dev {
                // Pre-provision the fixed set of content puppets. Each is created if
                // missing and permanently franchise-barred, so the switcher only ever
                // toggles between these and none can ever become a voter.
                for handle in &dev_accounts {
                    let handle = handle.trim();
                    if handle.is_empty() {
                        continue;
                    }
                    match services.ensure_barred_account(handle).await {
                        Ok(u) => println!("dev puppet ready: {} (id {})", u.handle, u.id.0),
                        Err(e) => eprintln!("⚠  could not provision dev puppet '{handle}': {e}"),
                    }
                }
                // A dev-enabled node that is network-reachable with no unlock secret
                // lets *anyone* who can reach it obtain the switcher — refuse it, the
                // same fail-closed stance as the session secret. Loopback stays free.
                if !loopback && dev_unlock_secret.is_none() {
                    anyhow::bail!(
                        "--dev is enabled on a non-loopback bind ({addr}) without \
                         --dev-unlock-secret (DEMOCRATOS_DEV_UNLOCK_SECRET): anyone who can reach \
                         this node could unlock the account switcher and post as any account. Set \
                         a secret, or bind the dev node to loopback only."
                    );
                }
            } else if dev_unlock_secret.is_some() {
                eprintln!("⚠  --dev-unlock-secret is set but --dev is off; the switcher stays disabled.");
            }

            // Invitation-only access + the admin invite review queue.
            //
            // Seed the live toggle from the persisted setting, falling back to the
            // `--invite-only` boot flag the first time it has never been set. The
            // flag only ever seeds; a later console toggle persists and then wins.
            let invite_only_initial = services
                .is_invite_only(cli.invite_only)
                .await
                .unwrap_or(cli.invite_only);
            let invite_only = Arc::new(std::sync::atomic::AtomicBool::new(invite_only_initial));

            let admin_subnets = parse_admin_subnets(cli.admin_subnet.as_deref())?;
            let admin_secret: Option<Arc<str>> = cli
                .admin_secret
                .clone()
                .filter(|s| !s.is_empty())
                .map(|s| s.into());

            // Fail-closed / loud-warning checks for the review queue and delivery.
            if admin_secret.is_none() {
                if cli.admin_subnet.is_some() {
                    eprintln!(
                        "⚠  --admin-subnet is set but no --admin-secret \
                         (DEMOCRATOS_ADMIN_SECRET): the invite review queue stays DISABLED — a \
                         secret is required to open it."
                    );
                } else {
                    eprintln!(
                        "ℹ  invite review queue is disabled; set --admin-secret and \
                         --admin-subnet to enable it."
                    );
                }
            } else if admin_subnets.is_empty() && !loopback {
                eprintln!(
                    "⚠  --admin-secret is set with no --admin-subnet on a non-loopback bind \
                     ({addr}): the review queue is reachable only from loopback (e.g. via an SSH \
                     tunnel). Add --admin-subnet to reach it from your LAN/VPN."
                );
            }
            if invite_only_initial && matches!(cli.notifier, NotifierKind::Log) {
                eprintln!(
                    "⚠  invitation-only is ON but --notifier is 'log': approvals will only PRINT \
                     the invite link to this log, not email it. Use --notifier smtp to send real \
                     email."
                );
            }

            // Choosing the web adapter — equally, this could be any driving adapter.
            adapter_web::serve(
                services,
                writes,
                minter,
                authenticator,
                session,
                &addr,
                dev,
                secure_cookies,
                dev_unlock_secret,
                invite_only,
                admin_subnets,
                admin_secret,
            )
            .await?;
        }
        Top::Cli(command) => {
            // The CLI is single-process: it owns whatever it touches, so writes
            // run locally (no forwarding).
            let writes = LocalWrites::new(services.clone());
            adapter_cli::dispatch(&services, &writes, command).await?;
        }
        // Handled before `build_services` above — it needs no services stack.
        Top::Issuer(_) => unreachable!("issuer commands are dispatched before build_services"),
        Top::Seed => {
            seed::run::run(&services).await?;
        }
        Top::Import { from } => {
            let store = fed_store.ok_or_else(|| {
                anyhow::anyhow!("import requires --store postgres --database-url ...")
            })?;
            let path = from.as_deref().unwrap_or(cli.data.as_str());
            let data = backfill::load::load(path)?;
            let counts = store.import(&data).await?;
            println!(
                "imported {} row(s) from {path}:\n\
                 \tusers {}  communities {}  memberships {}\n\
                 \tproposals {}  votes {}  rules {}\n\
                 \tposts {}  post_votes {}  comments {}\n\
                 \treports {}  trials {}  jury_ballots {}",
                counts.total(),
                counts.users,
                counts.demoi,
                counts.memberships,
                counts.proposals,
                counts.votes,
                counts.rules,
                counts.posts,
                counts.post_votes,
                counts.comments,
                counts.reports,
                counts.trials,
                counts.jury_ballots,
            );
        }
    }
    Ok(())
}
