//! The post templates cycled through when seeding content.

use crate::seed::post_template::PostTemplate;

pub(crate) const TEMPLATES: &[PostTemplate] = &[
    PostTemplate {
        title: "First light over the ridge",
        body: "Caught the sunrise after a cold night out. Worth every shiver.",
        with_image: true,
        tags: "landscape sunrise",
    },
    PostTemplate {
        title: "A question about ownership",
        body: "Still can't decide when to reach for Rc vs an arena. What do you reach for?",
        with_image: false,
        tags: "help discussion",
    },
    PostTemplate {
        title: "Notes from the town hall",
        body: "Turnout was low but the debate on the transit levy was surprisingly sharp.",
        with_image: true,
        tags: "local policy",
    },
    PostTemplate {
        title: "Long exposure, city rain",
        body: "Thirty seconds, tripod on a bin lid. Sometimes you improvise.",
        with_image: true,
        tags: "night street",
    },
    PostTemplate {
        title: "Shipped a small crate today",
        body: "Nothing fancy — a tiny parser combinator. Feedback welcome.",
        with_image: false,
        tags: "show-and-tell",
    },
    PostTemplate {
        title: "On deliberation vs. speed",
        body: "A slower process that everyone trusts beats a fast one nobody does.",
        with_image: false,
        tags: "governance",
    },
];
