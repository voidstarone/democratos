//! Dispatch a parsed [`Command`] to the matching `app::Services` use-case.

use anyhow::{anyhow, Result};

use app::{EnfranchiseOutcome, Services};
use domain::{
    build_comment_tree, CommentId, CommentNode, Demos, Media, Phase, PostId, ProposalId,
    ProposalKind, ProposalStatus, ReportId, ReportReason, ReportTarget, TrialId, User,
};

use crate::Command;

pub async fn dispatch(
    services: &Services,
    writes: &dyn app::GovernanceWrites,
    command: Command,
) -> Result<()> {
    match command {
        Command::Register { handle } => {
            let u = services.register_user(&handle).await?;
            println!("registered {} (id {})", u.handle, u.id);
        }
        Command::Found {
            founder,
            slug,
            name,
        } => {
            let f = user(services, &founder).await?;
            let d = services.found_demos(f.id, &slug, &name).await?;
            println!("founded d/{} — {} (founder {})", d.slug, d.name, f.handle);
        }
        Command::Join { handle, slug } => {
            let u = user(services, &handle).await?;
            let d = demos(services, &slug).await?;
            services.join(u.id, d.id).await?;
            println!("{} joined d/{}", u.handle, d.slug);
        }
        Command::Contribute {
            handle,
            slug,
            amount,
        } => {
            let u = user(services, &handle).await?;
            let d = demos(services, &slug).await?;
            services.record_contribution(u.id, d.id, amount).await?;
            println!(
                "recorded {amount} contribution for {} in d/{}",
                u.handle, d.slug
            );
        }
        Command::Enfranchise { handle, slug } => {
            let u = user(services, &handle).await?;
            let d = demos(services, &slug).await?;
            match services.request_enfranchisement(u.id, d.id).await? {
                EnfranchiseOutcome::Admitted => {
                    println!("{} is now a VOTER of d/{}", u.handle, d.slug)
                }
                EnfranchiseOutcome::Queued => {
                    println!(
                        "{} is eligible but QUEUED (rate cap) in d/{}",
                        u.handle, d.slug
                    )
                }
                EnfranchiseOutcome::NotEligible(e) => {
                    println!("{} is NOT yet eligible in d/{}:", u.handle, d.slug);
                    for unmet in &e.unmet {
                        println!("  - {unmet:?}");
                    }
                }
            }
        }
        Command::Propose {
            proposer,
            slug,
            target,
        } => {
            let u = user(services, &proposer).await?;
            let d = demos(services, &slug).await?;
            let p = services
                .open_proposal(u.id, d.id, ProposalKind::RemoveContent { target })
                .await?;
            println!("opened proposal #{} in d/{}", p.id, d.slug);
        }
        Command::Vote {
            handle,
            proposal,
            choice,
        } => {
            let u = user(services, &handle).await?;
            let aye = matches!(choice.as_str(), "aye" | "yes" | "y");
            // The CLI is a trusted local operator tool, not an untrusted node
            // relaying a browser's action, so it casts unsigned; the use-case
            // still enforces a signature for any account that has enrolled a key.
            writes
                .cast_vote(ProposalId(proposal), u.id, aye, None)
                .await?;
            println!(
                "{} voted {} on #{proposal}",
                u.handle,
                if aye { "aye" } else { "nay" }
            );
        }
        Command::Close { proposal } => {
            let status = services.close_proposal(ProposalId(proposal)).await?;
            match status {
                ProposalStatus::Passed { effective_at } => {
                    println!("#{proposal} PASSED (effective at {})", effective_at.0)
                }
                ProposalStatus::Failed => println!("#{proposal} FAILED"),
                ProposalStatus::Open => println!("#{proposal} still open"),
            }
        }
        Command::Show { slug } => show(services, &slug).await?,

        Command::Post {
            author,
            slug,
            kind,
            title,
            body,
            tags,
        } => {
            let u = user(services, &author).await?;
            let d = demos(services, &slug).await?;
            // The CLI keeps its single-media shape: `body` is the text for a text
            // post, or the media URL for an image/video (captioned by the title).
            let (post_body, media) = match kind.as_str() {
                "text" => (body, Vec::new()),
                "image" => (String::new(), vec![Media::image(body, title.clone())]),
                "video" => (String::new(), vec![Media::video(body, title.clone())]),
                other => return Err(anyhow!("unknown post kind '{other}' (text|image|video)")),
            };
            let tags = domain::normalize_tags(tags.as_deref().unwrap_or(""));
            let p = services
                .create_post(u.id, d.id, &title, &post_body, media, tags)
                .await?;
            println!("posted #{} [{}] in d/{}", p.id, p.kind_label(), d.slug);
        }
        Command::Search {
            query,
            demos: slug,
            tag,
        } => {
            let scope = match &slug {
                Some(s) => app::SearchScope::Demos(demos(services, s).await?.id),
                None => app::SearchScope::All,
            };
            let results = services.search(&query, scope, tag.as_deref()).await?;
            if !results.communities.is_empty() {
                println!("communities:");
                for d in &results.communities {
                    println!("  d/{} — {}", d.slug, d.name);
                }
            }
            if results.posts.is_empty() {
                println!("(no matching posts)");
            } else {
                println!("posts:");
                for p in &results.posts {
                    let tags = if p.tags.is_empty() {
                        String::new()
                    } else {
                        format!("  #{}", p.tags.join(" #"))
                    };
                    println!("  #{} [{}] {}{}", p.id, p.kind_label(), p.title, tags);
                }
            }
        }
        Command::Comment {
            author,
            post,
            body,
            parent,
        } => {
            let u = user(services, &author).await?;
            let c = services
                .comment(u.id, PostId(post), parent.map(CommentId), &body)
                .await?;
            println!("commented #{} on post #{post}", c.id);
        }
        Command::Feed { slug } => {
            let d = demos(services, &slug).await?;
            let posts = services.list_posts(d.id).await?;
            if posts.is_empty() {
                println!("(no posts)");
            }
            for p in posts {
                let mark = if p.removed { " [removed]" } else { "" };
                println!(
                    "#{} [{}] {}{} — by user #{}",
                    p.id,
                    p.kind_label(),
                    p.title,
                    mark,
                    p.author
                );
            }
        }
        Command::Upvote { user: handle, post } => {
            let u = user(services, &handle).await?;
            let score = writes
                .vote_post(PostId(post), u.id, Some(true), None)
                .await?;
            println!("upvoted post #{post} (score now {score})");
        }
        Command::Downvote { user: handle, post } => {
            let u = user(services, &handle).await?;
            let score = writes
                .vote_post(PostId(post), u.id, Some(false), None)
                .await?;
            println!("downvoted post #{post} (score now {score})");
        }
        Command::Home { user: handle } => {
            let u = user(services, &handle).await?;
            let feed = services.feed(u.id).await?;
            if feed.is_empty() {
                println!("(your feed is empty — join communities and upvote posts)");
            }
            for item in feed {
                println!(
                    "[{:+}] d/{} · #{} {}{} — by user #{}",
                    item.score,
                    item.community_slug,
                    item.post.id,
                    item.post.title,
                    nsfw_tag(item.post.is_nsfw),
                    item.post.author
                );
            }
        }
        Command::Top => {
            let top = services.top_posts().await?;
            if top.is_empty() {
                println!("(no posts yet)");
            }
            for item in top {
                println!(
                    "[{:+}] d/{} · #{} {}{} — by user #{}",
                    item.score,
                    item.community_slug,
                    item.post.id,
                    item.post.title,
                    nsfw_tag(item.post.is_nsfw),
                    item.post.author
                );
            }
        }
        Command::Recommend {
            user: handle,
            limit,
        } => {
            let u = user(services, &handle).await?;
            // One-shot process: no background refresher, so build the model once
            // (a no-op if already current) before reading from it.
            services.refresh_recommendations().execute().await?;
            let recs = services.recommend_feed().execute(u.id, limit).await?;
            if recs.is_empty() {
                println!(
                    "(no recommendations yet — upvote a few posts so we can find users like you)"
                );
            }
            for r in recs {
                println!(
                    "[{:.2}] d/{} · #{} {}{} — by user #{}",
                    r.affinity,
                    r.community_slug,
                    r.post.id,
                    r.post.title,
                    nsfw_tag(r.post.is_nsfw),
                    r.post.author
                );
            }
        }
        Command::Thread { post } => {
            let p = services
                .posts
                .get(PostId(post))
                .await?
                .ok_or_else(|| anyhow!("no such post: {post}"))?;
            println!(
                "#{} [{}] {}{}",
                p.id,
                p.kind_label(),
                p.title,
                nsfw_tag(p.is_nsfw)
            );
            if !p.body.is_empty() {
                println!("  {}", p.body);
            }
            for m in &p.media {
                println!("  [{}] {} — {}", m.kind_label(), m.url, m.caption);
            }
            let tree = build_comment_tree(services.comments_for(PostId(post)).await?);
            print_comments(&tree, 1);
        }

        Command::ProposeRule {
            proposer,
            slug,
            text,
        } => {
            let u = user(services, &proposer).await?;
            let d = demos(services, &slug).await?;
            let p = services
                .open_proposal(u.id, d.id, ProposalKind::AddRule { text })
                .await?;
            println!("opened rule proposal #{} in d/{}", p.id, d.slug);
        }
        Command::Rules { slug } => {
            let d = demos(services, &slug).await?;
            let rules = services.list_rules(d.id).await?;
            if rules.is_empty() {
                println!("(no rules)");
            }
            for r in rules {
                println!("#{} {}", r.id, r.text);
            }
        }
        Command::NsfwPolicy {
            proposer,
            slug,
            decision,
        } => {
            let allows_nsfw = match decision.as_str() {
                "allow" | "allowed" => true,
                "forbid" | "forbidden" | "deny" => false,
                other => return Err(anyhow!("decision must be allow|forbid (got '{other}')")),
            };
            let u = user(services, &proposer).await?;
            let d = demos(services, &slug).await?;
            let p = services
                .open_proposal(u.id, d.id, ProposalKind::SetNsfwPolicy { allows_nsfw })
                .await?;
            println!(
                "opened NSFW-policy proposal #{} in d/{} ({})",
                p.id,
                d.slug,
                if allows_nsfw { "allow" } else { "forbid" }
            );
        }

        Command::VerifyAge { user: handle } => {
            let u = user(services, &handle).await?;
            if services.verify_age(u.id).await? {
                println!("{} is now age-verified", u.handle);
            } else {
                println!("age verification failed for {}", u.handle);
            }
        }

        Command::Report {
            reporter,
            slug,
            post,
            note,
        } => {
            let u = user(services, &reporter).await?;
            let d = demos(services, &slug).await?;
            let r = services
                .file_report(
                    u.id,
                    d.id,
                    ReportTarget::Post(PostId(post)),
                    ReportReason::RuleBreak { rule: None },
                    &note,
                )
                .await?;
            println!("filed report #{} (rule-break) on post #{post}", r.id);
        }
        Command::Reports { slug } => {
            let d = demos(services, &slug).await?;
            let open = services.reports.list_open(d.id).await?;
            if open.is_empty() {
                println!("(no open reports)");
            }
            for r in open {
                let who = match r.founding().reporter {
                    Some(u) => format!("user #{u}"),
                    None => "AUTO-DETECTOR".to_string(),
                };
                let reasons: Vec<String> =
                    r.flags.iter().map(|f| format!("{:?}", f.reason)).collect();
                println!(
                    "#{} [{}] target={:?} by {} — {}",
                    r.id,
                    reasons.join(", "),
                    r.target,
                    who,
                    r.founding().note
                );
            }
        }
        Command::Trial { by, report } => {
            let caller = user(services, &by).await?;
            let t = services.open_trial(caller.id, ReportId(report)).await?;
            let jurors: Vec<u64> = t.jurors.iter().map(|j| j.0).collect();
            println!(
                "trial #{} empanelled — accused user #{}, jury (users) {:?}",
                t.id, t.accused, jurors
            );
        }
        Command::Jury {
            juror,
            trial,
            verdict,
        } => {
            let u = user(services, &juror).await?;
            let guilty = matches!(verdict.as_str(), "guilty" | "g");
            let v = writes
                .cast_jury_vote(TrialId(trial), u.id, guilty, None)
                .await?;
            println!(
                "{} voted {} on trial #{trial} — verdict now {:?}",
                u.handle,
                if guilty { "guilty" } else { "not-guilty" },
                v
            );
        }
    }
    Ok(())
}

fn print_comments(nodes: &[CommentNode], depth: usize) {
    for n in nodes {
        let indent = "  ".repeat(depth);
        let body = if n.comment.removed {
            "[removed]".to_string()
        } else {
            n.comment.body.clone()
        };
        println!(
            "{indent}└ #{} (user #{}): {}",
            n.comment.id, n.comment.author, body
        );
        print_comments(&n.children, depth + 1);
    }
}

/// A trailing `" [NSFW]"` marker for flagged posts in listings, else empty.
fn nsfw_tag(is_nsfw: bool) -> &'static str {
    if is_nsfw {
        " [NSFW]"
    } else {
        ""
    }
}

async fn user(services: &Services, handle: &str) -> Result<User> {
    services
        .users
        .by_handle(handle)
        .await?
        .ok_or_else(|| anyhow!("no such user: {handle}"))
}

async fn demos(services: &Services, slug: &str) -> Result<Demos> {
    services
        .demoi
        .by_slug(slug)
        .await?
        .ok_or_else(|| anyhow!("no such demos: {slug}"))
}

async fn show(services: &Services, slug: &str) -> Result<()> {
    let d = demos(services, slug).await?;
    let voters = services.memberships.voter_count(d.id).await?;
    let phase = Phase::from_voter_count(voters);
    println!("d/{} — {}", d.slug, d.name);
    println!("  phase: {phase:?}  voters: {voters}");
    println!(
        "  criteria: account≥{}d, member≥{}d, contribution≥{}",
        d.criteria.min_account_age_days,
        d.criteria.min_membership_days,
        d.criteria.min_contribution
    );
    let proposals = services.proposals.list(d.id).await?;
    if proposals.is_empty() {
        println!("  (no proposals)");
    }
    for p in proposals {
        let tally = services.votes.tally(p.id).await?;
        println!(
            "  #{} [{:?}] {:?}  aye {} / nay {}",
            p.id, p.kind, p.status, tally.aye, tally.nay
        );
    }
    Ok(())
}
