/// One pending request as shown in the admin review queue.
pub struct InviteQueueItem {
    pub id: u64,
    pub email: String,
    /// The requester's note, or empty if none.
    pub note: String,
    /// Whole days the request has been waiting.
    pub waited_days: i64,
}
