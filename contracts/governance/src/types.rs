use soroban_sdk::{contracttype, Address, Symbol, Vec};

/// Status of a budget proposal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    /// The proposal is open for voting.
    Active,
    /// The proposal passed (met quorum and majority voted yes).
    Approved,
    /// The proposal failed (did not meet quorum or majority voted no).
    Rejected,
    /// The approved proposal has been executed (funds disbursed).
    Executed,
    /// The proposal was cancelled by the proposer.
    Cancelled,
    /// The proposal was not finalized within the grace period.
    Expired,
}

/// The type of vote cast by a member.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}

/// A record of a single vote.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoteRecord {
    pub voter: Address,
    pub choice: VoteChoice,
    pub timestamp: u64,
}

/// A single entry in the snapshotted electorate: one member and their frozen weight.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberWeight {
    pub member: Address,
    pub weight: u128,
}

/// Immutable snapshot of governance parameters captured at proposal creation.
///
/// All vote-weight lookups and finalization checks use these frozen values so
/// that subsequent admin changes (member additions/removals, weight updates,
/// quorum tweaks) cannot alter the outcome of an in-flight proposal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalSnapshot {
    /// Quorum percentage required for the proposal to be valid (0-100).
    pub quorum_percentage: u32,
    /// Grace period in seconds after voting ends before the proposal auto-expires.
    pub grace_period: u64,
    /// Total voting weight across all eligible members at creation time.
    pub total_weight: u128,
    /// Eligible electorate with per-member weights frozen at creation time.
    pub electorate: Vec<MemberWeight>,
}

/// A budget proposal requesting funds from the treasury.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    /// Unique proposal ID.
    pub id: u32,
    /// Who submitted the proposal.
    pub proposer: Address,
    /// Short title for the proposal.
    pub title: Symbol,
    /// The token being requested.
    pub token: Address,
    /// Amount of tokens requested.
    pub amount: i128,
    /// The recipient of funds if approved.
    pub recipient: Address,
    /// Votes in favor.
    pub yes_votes: u128,
    /// Votes against.
    pub no_votes: u128,
    /// Abstaining votes.
    pub abstain_votes: u128,
    /// List of all vote records.
    pub votes: Vec<VoteRecord>,
    /// Current status.
    pub status: ProposalStatus,
    /// Timestamp when voting begins.
    pub start_time: u64,
    /// Timestamp when voting ends.
    pub end_time: u64,
    /// Governance parameters and electorate frozen at proposal creation.
    pub snapshot: ProposalSnapshot,
}

/// Configuration for the governance module.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceConfig {
    /// Minimum percentage of total voting weight that must vote for the proposal to be valid (0-100).
    pub quorum_percentage: u32,
    /// Duration of the voting window in seconds.
    pub voting_duration: u64,
    /// Buffer period after voting ends before auto-rejection (seconds).
    pub grace_period: u64,
    /// Total number of DAO members.
    pub member_count: u32,
    /// Total voting weight of all members.
    pub total_weight: u128,
}
