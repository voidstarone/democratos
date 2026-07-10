//! Dev-only fixture loader. `democratos seed` populates a fresh store with a
//! handful of communities, a cast of users spanning the whole popularity range,
//! multi-media posts, comments, and up/down votes — enough to exercise the feed,
//! the popularity metric, and the posting-policy gate without hand-entering data.
//!
//! It is *not* a migration or a production concern: it only ever calls the same
//! `Services` use-cases the web and CLI adapters do, so it lives here in the
//! composition root beside the other wiring. Every account it creates shares the
//! password [`seed_password::SEED_PASSWORD`], so you can sign in as any of them.

pub mod communities;
pub mod community;
pub mod generate_image;
pub mod people;
pub mod person;
pub mod post_template;
pub mod run;
pub mod seed_password;
pub mod templates;
