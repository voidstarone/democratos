//! The CLI subcommand set.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
    /// Register a new user account.
    Register { handle: String },
    /// Found a demos (the founder becomes voter #1).
    Found {
        founder: String,
        slug: String,
        name: String,
    },
    /// Join a demos as a member.
    Join { handle: String, slug: String },
    /// Record positively-received contribution for a member.
    Contribute {
        handle: String,
        slug: String,
        amount: i64,
    },
    /// Request voter status (applies Layers 1 & 2).
    Enfranchise { handle: String, slug: String },
    /// Open a "remove content" moderation proposal.
    Propose {
        proposer: String,
        slug: String,
        target: String,
    },
    /// Cast a vote on a proposal (choice: aye|nay).
    Vote {
        handle: String,
        proposal: u64,
        choice: String,
    },
    /// Close and tally a proposal.
    Close { proposal: u64 },
    /// Print a demos overview: phase, voters, criteria, proposals.
    Show { slug: String },

    // --- content ----------------------------------------------------------
    /// Create a post. kind = text | image | video (body is the text or media URL).
    Post {
        author: String,
        slug: String,
        kind: String,
        title: String,
        body: String,
        /// Comma- or space-separated tags.
        #[arg(long)]
        tags: Option<String>,
    },
    /// Search posts (and, site-wide, communities) by text and/or tag.
    Search {
        query: String,
        /// Restrict to one community by slug (omit to search the whole site).
        #[arg(long)]
        demos: Option<String>,
        /// Require this exact tag.
        #[arg(long)]
        tag: Option<String>,
    },
    /// Comment on a post (optionally as a reply to another comment via --parent).
    Comment {
        author: String,
        post: u64,
        body: String,
        #[arg(long)]
        parent: Option<u64>,
    },
    /// List the posts in a demos.
    Feed { slug: String },
    /// Upvote a post (as a member of its community).
    Upvote { user: String, post: u64 },
    /// Downvote a post (as a member of its community).
    Downvote { user: String, post: u64 },
    /// Print a user's personalized home feed (sufficiently-upvoted posts from
    /// the communities they have joined).
    Home { user: String },
    /// Print the site-wide "top" feed: most popular posts across all communities.
    Top,
    /// Recommend posts to a user from the activity of similar users (the posts
    /// people who like what they like also upvoted).
    Recommend {
        user: String,
        /// Maximum number of recommendations (0 uses the default).
        #[arg(long, default_value_t = 0)]
        limit: usize,
    },
    /// Print a post and its comment tree.
    Thread { post: u64 },

    // --- rules ------------------------------------------------------------
    /// Propose adding a community rule (a RuleChange vote).
    ProposeRule {
        proposer: String,
        slug: String,
        text: String,
    },
    /// List a demos's active rules.
    Rules { slug: String },
    /// Propose setting a community's NSFW policy (a RuleChange vote). NSFW is
    /// allowed-but-gated by default; pass `forbid` to make detected NSFW posts
    /// auto-report to a jury, or `allow` to permit it again.
    NsfwPolicy {
        proposer: String,
        slug: String,
        /// `allow` | `forbid`.
        decision: String,
    },

    // --- age verification -------------------------------------------------
    /// Run age verification for a user (uses the configured provider; the dev
    /// stub approves). Lets the user reveal NSFW content where the deployment
    /// requires verification.
    VerifyAge { user: String },

    // --- moderation: reports & trial by jury ------------------------------
    /// Report a post for breaking a rule.
    Report {
        reporter: String,
        slug: String,
        post: u64,
        note: String,
    },
    /// List open reports (including automatic bot reports).
    Reports { slug: String },
    /// Empanel a jury and put a report on trial (as a voter of its community).
    Trial { by: String, report: u64 },
    /// Cast a juror's vote (verdict = guilty | notguilty).
    Jury {
        juror: String,
        trial: u64,
        verdict: String,
    },
}
