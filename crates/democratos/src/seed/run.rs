//! Seed the store with the dev fixtures.

use anyhow::{bail, Result};

use app::Services;
use domain::{DemosId, PostingPolicy, UserId};

use crate::seed::communities::COMMUNITIES;
use crate::seed::community::Community;
use crate::seed::generate_image::generate_image;
use crate::seed::people::PEOPLE;
use crate::seed::seed_password::SEED_PASSWORD;
use crate::seed::templates::TEMPLATES;

/// Seed the store. Refuses to run if any of the target communities already
/// exists, so it never doubles up on a store that has already been seeded.
pub async fn run(services: &Services) -> Result<()> {
    for c in COMMUNITIES {
        if services.demoi.by_slug(c.slug).await?.is_some() {
            bail!(
                "d/{} already exists — seed onto a fresh store \
                 (e.g. `--data seed.json` or `--store memory`)",
                c.slug
            );
        }
    }

    // 1. Accounts. Every person gets the shared dev password so you can log in.
    let mut ids: Vec<(&'static str, UserId)> = Vec::new();
    for person in PEOPLE {
        let email = format!("{}@example.test", person.handle);
        let u = services
            .register_account(person.handle, &email, SEED_PASSWORD)
            .await?;
        ids.push((person.handle, u.id));
    }
    let id_of = |handle: &str| {
        ids.iter()
            .find(|(h, _)| *h == handle)
            .map(|(_, i)| *i)
            .unwrap()
    };
    let fame_of = |handle: &str| {
        PEOPLE
            .iter()
            .find(|p| p.handle == handle)
            .map(|p| p.fame)
            .unwrap()
    };

    // 2. Communities, founded by their founder (who becomes voter #1). They are
    //    left at the default "members" policy for now so seeding can post freely;
    //    the real policy is applied at the end.
    let mut demoi: Vec<(&'static Community, DemosId)> = Vec::new();
    for c in COMMUNITIES {
        let d = services
            .found_demos(id_of(c.founder), c.slug, c.name)
            .await?;
        demoi.push((c, d.id));
    }

    // 3. Everyone joins every community (a member may vote; the founder already
    //    is one of their own). Membership is the precondition for voting, which
    //    is how popularity accrues.
    for (_, demos_id) in &demoi {
        for person in PEOPLE {
            let uid = id_of(person.handle);
            // Skip the founder of this community — already a voter-member.
            if services.memberships.get(uid, *demos_id).await?.is_none() {
                services.join(uid, *demos_id).await?;
            }
        }
    }

    // 4. Posts + comments + votes, per community. Popularity is scored per
    //    community, so we do a full pass in each.
    for (community, demos_id) in &demoi {
        seed_community_content(services, community, *demos_id, &id_of, &fame_of).await?;
    }

    // 5. Lock in each community's real posting policy now that content exists.
    for (community, demos_id) in &demoi {
        services
            .demoi
            .set_posting_policy(*demos_id, community.final_policy)
            .await?;
    }

    print_summary(services, &demoi, &id_of).await?;
    Ok(())
}

/// Fill one community: each member authors `1 + fame` posts, high-fame authors
/// draw more upvotes, and a couple of comments per post get their own votes.
async fn seed_community_content(
    services: &Services,
    _community: &Community,
    demos_id: DemosId,
    id_of: &impl Fn(&str) -> UserId,
    fame_of: &impl Fn(&str) -> u8,
) -> Result<()> {
    let members: Vec<&'static str> = PEOPLE.iter().map(|p| p.handle).collect();

    // Track (post_id, author) so we can cast votes after everything exists.
    let mut posts: Vec<(domain::PostId, &'static str)> = Vec::new();
    let mut template_cursor = 0usize;

    for author in &members {
        let fame = fame_of(author);
        let post_count = 1 + fame as usize;
        for _ in 0..post_count {
            let tpl = &TEMPLATES[template_cursor % TEMPLATES.len()];
            template_cursor += 1;
            let media = if tpl.with_image {
                vec![generate_image(tpl.title, template_cursor as u32)]
            } else {
                Vec::new()
            };
            let tags = domain::normalize_tags(tpl.tags);
            let post = services
                .create_post(id_of(author), demos_id, tpl.title, tpl.body, media, tags)
                .await?;
            posts.push((post.id, author));
        }
    }

    // Votes: for a post by an author of fame f, the first `f*3 + 1` *other*
    // members upvote it; fame-0 authors also collect one downvote, netting ~0.
    for (post_id, author) in &posts {
        let fame = fame_of(author);
        let upvotes = (fame as usize) * 3 + 1;
        let voters: Vec<&str> = members.iter().copied().filter(|h| h != author).collect();
        for voter in voters.iter().take(upvotes) {
            services
                .vote_post(*post_id, id_of(voter), Some(true), None)
                .await?;
        }
        if fame == 0 {
            if let Some(downvoter) = voters.last() {
                services
                    .vote_post(*post_id, id_of(downvoter), Some(false), None)
                    .await?;
            }
        }
    }

    // A light comment layer: every author leaves one comment on the first post
    // that isn't their own, and high-fame commenters attract a few upvotes.
    for author in &members {
        let target = posts.iter().find(|(_, a)| a != author);
        if let Some((post_id, _)) = target {
            let comment = services
                .comment(
                    id_of(author),
                    *post_id,
                    None,
                    "Genuinely great — thanks for sharing this.",
                )
                .await?;
            let fame = fame_of(author);
            let voters: Vec<&str> = members.iter().copied().filter(|h| h != author).collect();
            for voter in voters.iter().take((fame as usize) * 2) {
                services
                    .vote_comment(comment.id, id_of(voter), Some(true))
                    .await?;
            }
        }
    }

    Ok(())
}

/// Print a per-community popularity table so it's obvious the spread landed.
async fn print_summary(
    services: &Services,
    demoi: &[(&'static Community, DemosId)],
    id_of: &impl Fn(&str) -> UserId,
) -> Result<()> {
    println!("seeded {} communities, {} users", demoi.len(), PEOPLE.len());
    println!("every account's password is `{SEED_PASSWORD}` (dev only)\n");
    for (community, demos_id) in demoi {
        println!(
            "d/{} — {} (posting: {:?})",
            community.slug, community.name, community.final_policy
        );
        let mut rows: Vec<(&str, i64)> = Vec::new();
        for person in PEOPLE {
            let m = services
                .member_metrics(id_of(person.handle), *demos_id)
                .await?;
            rows.push((person.handle, m.popularity()));
        }
        rows.sort_by(|a, b| b.1.cmp(&a.1));
        for (handle, pop) in rows {
            let gate = match community.final_policy {
                PostingPolicy::MinContribution(n) if pop < n => "  (below posting threshold)",
                _ => "",
            };
            println!("   {pop:>4}  {handle}{gate}");
        }
        println!();
    }
    Ok(())
}
