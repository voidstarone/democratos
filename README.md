# Democratos

A Reddit-style forum where each community — a **demos** — governs itself.
Citizenship (the right to vote) is *earned*, and the rules of citizenship are
*themselves set by vote*. The hard problem is letting communities change while
stopping a flood of new users from seizing one; see
[`docs/governance.md`](docs/governance.md) for the model.

## Architecture — ports & adapters (hexagonal)

The crate graph enforces swappability. `domain` and `app` cannot name a database
or a web framework, so storage and delivery are chosen only in the composition
root.

```
          driving adapters                      driven adapters
        (swap API <-> CLI)                     (swap DB <-> file)
   ┌───────────────────────┐             ┌───────────────────────────┐
   │  adapter-web (Axum)   │             │  adapter-store-memory     │
   │  adapter-cli (clap)   │             │  adapter-store-textfile   │
   └───────────┬───────────┘             └─────────────┬─────────────┘
               │  call use-cases            implement ports
               ▼                                       ▲
        ┌───────────────────────────────────────────────────────┐
        │  app   — use-cases (Services) + port traits           │
        └───────────────────────────┬───────────────────────────┘
                                    ▼
        ┌───────────────────────────────────────────────────────┐
        │  domain — pure governance logic, no I/O, fully tested │
        └───────────────────────────────────────────────────────┘

        democratos — composition root: picks store + clock + delivery
```

| Crate | Role |
|---|---|
| `domain` | Pure governance rules (the four defensive layers). No I/O. |
| `app` | Use-cases (`Services`) and the **port traits** adapters implement. |
| `adapter-store-memory` | In-memory store (dev/tests) + a controllable clock. |
| `adapter-store-textfile` | Persists the whole dataset to one JSON file. |
| `adapter-media-local` | Stores uploaded image/video bytes on local disk (CDN-swappable). |
| `adapter-web` | Server-rendered, progressively-enhanced, translatable HTML. |
| `adapter-cli` | A CLI over the *same* use-cases. |
| `democratos` | The only crate that names concrete adapters. |

**Swapping storage** is one `match` arm in `crates/democratos/src/main.rs`
(`build_services`). **Swapping delivery** (web vs. CLI) is one `match` on the
subcommand. Neither touches `domain` or `app`.

## The governance engine (`domain`)

Four layers, each a pure, unit-tested function — together they make takeover
*slow, not impossible*:

1. **Earned franchise** — `evaluate_eligibility` (account age + dwell + contribution).
2. **Enfranchisement rate cap** — `enfranchisement_slots` (voter roll grows ≤10%/30d, floor 5).
3. **Tiered thresholds** — `threshold_for` (majority / 60% / supermajority; amendments disabled in Seed).
4. **Timelock + recall** — `Proposal::close` (constitutional changes wait out a recall window).

## Content & moderation

Beyond the franchise, the domain models the forum itself and how it polices
itself (all pure, all unit-tested):

- **Rules** — a demos votes its rulebook in/out (`RuleChange`, allowed even in Seed).
- **Posts** (text / image / video) and a **tree of comments**.
- **Bot detection** — `bot_score` flags likely bots and auto-files a report; it
  never auto-punishes ("the machine accuses, the demos judges").
- **Trial by jury** — `select_jury` (deterministic, seeded, auditable) + a 2/3
  conviction bar. A guilty verdict sanctions the accused (disqualifying them from
  the franchise) and removes the content.

All of this is exercised end-to-end over the CLI:

```sh
democratos --data db.json cli propose-rule founder rust "Be excellent"
democratos --data db.json cli post alice rust text "Title" "body text"
democratos --data db.json cli comment bob 1 "a reply" --parent 1
democratos --data db.json cli thread 1
democratos --data db.json cli report alice rust 1 "breaks rule 1"
democratos --data db.json cli trial founder 1  # a voter empanels a jury
democratos --data db.json cli jury juror7 1 guilty
democratos --data db.json cli reports rust   # shows auto bot reports too
```

## Frontend

The web UI covers the whole system — communities, the franchise, governance, **and**
content + moderation (a posts feed, post pages with a comment tree, a reports page,
and jury-trial pages where empanelled jurors vote). All of it is server-rendered HTML
that works with **plain forms and no JavaScript**. The
`static/app.js` layer is purely additive: it submits votes in the background and
updates tallies in place, falling back to a native form submit on any error.
Everything is **translatable** — UI strings live in a type-safe catalog
(`src/i18n.rs`), so a missing translation is a compile error. English and Spanish
ship today; locale resolves from a `lang` cookie or `Accept-Language`.

Progressive enhancement keeps everything working with JS off, then layers on:
votes submit in place; the submit form adapts its fields to the post type and adds
drag-and-drop upload with a live preview; comment replies expand inline (native
`<details>`); search is a plain GET form.

## Media uploads, tags & search

- **Uploads.** Image/video posts can be uploaded directly (multipart), not just
  linked by URL. Bytes live behind the **`MediaStore`** port (`crates/app/src/ports.rs`).
  The domain still stores only the **URL** the store returns — never bytes — so the
  backend is a drop-in swap: `adapter-media-local` writes content-addressed files to
  disk and serves them from `/media/:key`; a future S3/CDN adapter would upload to a
  bucket and return a CDN URL instead. **That swap is one `Arc::new(...)` line in
  `build_services`** — nothing else changes. Accepted: png/jpeg/gif/webp, mp4/webm,
  up to 25 MB. The `memory` store keeps media in RAM; the `file` store uses
  `--media-dir` (default `./media`).
- **Tags.** Posts carry normalized tags (`domain::normalize_tags`); tag chips link to
  filtered search.
- **Search.** Header box (whole site) and a per-community sidebar box search post
  titles/bodies/tags plus community names/slugs, scoped to `all` or one community
  (`/search?q=…&scope=…&tag=…`). The same logic backs the `cli search` command.

## Home feed & post upvotes

Members up/down vote posts (one toggleable vote per member per post, via
`PostVoteStore`); a post's **net score** is `upvotes − downvotes`. The signed-in
home page is **"Your feed"**: across every community you've joined, the posts whose
score clears that community's bar, newest-highest first. The bar **scales with the
community** — `domain::feed_threshold` is ≈10% of its voters (min 1), so a post needs
broader support to surface in a large demos than a tiny one. Voting is progressively
enhanced (arrows update the score in place; a plain form post + redirect with JS off).
CLI: `cli upvote <user> <post>`, `cli downvote <user> <post>`, `cli home <user>`.

## Running

```sh
cargo build      # build everything
cargo test       # domain rules + end-to-end use-case flows
```

### Serve the web app

By default data is **persisted to `democratos.json`** in the current directory, so
your communities, accounts, and posts survive restarts:

```sh
cargo run -p democratos -- serve
# → http://127.0.0.1:3000
```

On startup the server prints exactly where data is going, e.g.:

```
storage: file — saving to /…/democratos/democratos.json
   loaded 1 community: d/rust
```

Then open the home page, sign in with any handle, and use **Found a community**.

> [!IMPORTANT]
> **`--store memory` does not save anything.** It is an ephemeral, in-process
> store meant for tests — everything is lost the moment the server exits, and no
> file is written. If your communities keep disappearing, this is why. The
> server prints a loud `⚠ storage: IN-MEMORY` warning when you use it. For normal
> use, just omit `--store` (the default is `file`).

### Command-line options

Global options (before the subcommand) choose where data lives:

| Option | Default | Meaning |
|---|---|---|
| `--store <file\|memory>` | `file` | `file` = persisted to disk; `memory` = ephemeral, lost on exit. |
| `--data <PATH>` | `democratos.json` | Which JSON file the `file` store reads and writes. Relative to the current directory. |
| `--media-dir <PATH>` | `media` | Directory the `file` store writes uploaded media into. |

The `serve` subcommand adds:

| Option | Default | Meaning |
|---|---|---|
| `--addr <HOST:PORT>` | `127.0.0.1:3000` | Address to bind the HTTP server to. |
| `--dev` | off | Enable the **developer account switcher**: a floating bar to create and instantly switch between test accounts so one browser can act as many users. Never enable in a real deployment. |

Examples:

```sh
# Persisted, on a custom port, with the dev account switcher
cargo run -p democratos -- --data democratos.json serve --addr 127.0.0.1:8080 --dev

# Throwaway run (nothing saved) — handy for a clean-slate demo or tests
cargo run -p democratos -- --store memory serve
```

### Same application over the CLI

The CLI is a second driving adapter over the *same* use-cases and the *same*
data file — anything you do here shows up in the web app, and vice versa:

```sh
cargo run -p democratos -- --data democratos.json cli register alice
cargo run -p democratos -- --data democratos.json cli found alice rust "Rustaceans"
cargo run -p democratos -- --data democratos.json cli post alice rust text "Hello" "first post" --tags intro,welcome
cargo run -p democratos -- --data democratos.json cli search "hello"              # site-wide
cargo run -p democratos -- --data democratos.json cli search "" --demos rust --tag intro
cargo run -p democratos -- --data democratos.json cli show rust
```

Run `cargo run -p democratos -- cli --help` for the full list of subcommands
(register, found, join, enfranchise, propose, vote, post, comment, search,
report, trial, jury, …).
