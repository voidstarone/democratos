//! The seed manifest linking the seeded proposal to its eligible voters.

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Manifest {
    pub(crate) demos_id: u64,
    pub(crate) proposal_id: u64,
    pub(crate) founder_id: u64,
    pub(crate) voter_ids: Vec<u64>,
}
