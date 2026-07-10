//! Assemble a flat list of comments into a reply tree.

use std::collections::HashMap;

use crate::content::comment::Comment;
use crate::content::comment_node::CommentNode;
use crate::CommentId;

/// Assemble a flat list of comments into a reply tree. Order within each level
/// follows the input order (callers typically pass comments sorted by time).
/// Comments whose parent is absent from the list are dropped from the tree.
pub fn build_comment_tree(comments: Vec<Comment>) -> Vec<CommentNode> {
    let mut by_parent: HashMap<Option<CommentId>, Vec<Comment>> = HashMap::new();
    for c in comments {
        by_parent.entry(c.parent).or_default().push(c);
    }
    build_level(None, &mut by_parent)
}

fn build_level(
    parent: Option<CommentId>,
    by_parent: &mut HashMap<Option<CommentId>, Vec<Comment>>,
) -> Vec<CommentNode> {
    let mut nodes = Vec::new();
    // `remove` both consumes the children and guards against cycles.
    if let Some(children) = by_parent.remove(&parent) {
        for c in children {
            let id = c.id;
            let kids = build_level(Some(id), by_parent);
            nodes.push(CommentNode {
                comment: c,
                children: kids,
            });
        }
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PostId, UserId};
    use crate::time::Timestamp;

    fn comment(id: u64, parent: Option<u64>) -> Comment {
        Comment::new(
            CommentId(id),
            PostId(1),
            UserId(1),
            parent.map(CommentId),
            format!("c{id}"),
            Timestamp(id as i64),
        )
    }

    #[test]
    fn builds_a_reply_tree() {
        // 1 ─ 2 ─ 4
        //   └ 3
        // 5 (separate root)
        let tree = build_comment_tree(vec![
            comment(1, None),
            comment(2, Some(1)),
            comment(3, Some(1)),
            comment(4, Some(2)),
            comment(5, None),
        ]);

        assert_eq!(tree.len(), 2); // roots 1 and 5
        let root1 = &tree[0];
        assert_eq!(root1.comment.id, CommentId(1));
        assert_eq!(root1.children.len(), 2); // 2 and 3
        assert_eq!(root1.children[0].children.len(), 1); // 4 under 2
        assert_eq!(root1.children[0].children[0].comment.id, CommentId(4));
        assert!(tree[1].children.is_empty()); // 5 has no replies
    }
}
