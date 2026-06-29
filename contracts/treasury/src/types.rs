use soroban_sdk::{contracttype, Address, Symbol, Vec};

/// Represents the status of a withdrawal request in the multi-sig flow.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WithdrawalStatus {
    /// Waiting for signers to approve.
    Pending,
    /// Threshold met — ready for execution.
    Approved,
    /// Funds have been transferred to the recipient.
    Executed,
    /// The request was cancelled by the proposer or admin.
    Cancelled,
}

/// Status of the treasury pause state.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PauseState {
    /// Normal operation - withdrawals and deposits allowed.
    Unpaused,
    /// Emergency pause - all token transfers blocked.
    Paused,
}

/// A withdrawal request that tracks the multi-sig approval process.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawalRequest {
    /// Unique identifier for this withdrawal request.
    pub id: u32,
    /// The signer who created this withdrawal request.
    pub proposer: Address,
    /// The token address to withdraw.
    pub token: Address,
    /// The recipient address for the funds.
    pub recipient: Address,
    /// The amount of tokens to withdraw.
    pub amount: i128,
    /// A short description or reference for this withdrawal.
    pub memo: Symbol,
    /// List of signers who have approved this request.
    pub approvals: Vec<Address>,
    /// Current status of the withdrawal request.
    pub status: WithdrawalStatus,
    /// Ledger timestamp when the request was created.
    pub created_at: u64,
    /// The signer set version at the time of creation.
    /// Used to determine which signers can approve this request.
    pub signer_set_version: u32,
}

/// An upgrade proposal requiring multi-sig approval.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeProposal {
    /// Unique proposal identifier.
    pub id: u32,
    /// The admin who proposed the upgrade.
    pub proposer: Address,
    /// Hash of the new WASM bytecode.
    pub wasm_hash: soroban_sdk::BytesN<32>,
    /// Short description of the upgrade.
    pub description: Symbol,
    /// List of signers who have approved.
    pub approvals: Vec<Address>,
    /// Status of the upgrade proposal.
    pub status: UpgradeStatus,
    /// Signer set version at time of proposal.
    pub signer_set_version: u32,
    /// Timestamp when proposal was created.
    pub created_at: u64,
}

/// Status of an upgrade proposal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpgradeStatus {
    /// Pending approval.
    Pending,
    /// Threshold met — ready for execution.
    Approved,
    /// Upgrade has been executed.
    Executed,
    /// Pending signer change with timelock.
    PendingSignerChange,
    /// Scheduled admin change with timelock.
    PendingAdminChange,
}

/// Pending admin change with timelock.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAdminChange {
    /// The new admin address.
    pub new_admin: Address,
    /// The signer set version when change was scheduled.
    pub signer_set_version: u32,
    /// When the change becomes effective (ledger timestamp).
    pub effective_at: u64,
}

/// Treasury configuration snapshot — used for read-only queries.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryConfig {
    /// The admin address.
    pub admin: Address,
    /// Current list of authorized signers.
    pub signers: Vec<Address>,
    /// Number of approvals required for a withdrawal.
    pub threshold: u32,
    /// Total number of proposals created.
    pub proposal_count: u32,
    /// Current pause state.
    pub paused: bool,
}

/// Migration parameters for transferring state between Treasury versions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationParams {
    /// The current admin address.
    pub admin: Address,
    /// Current list of authorized signers.
    pub signers: Vec<Address>,
    /// Number of approvals required for a withdrawal.
    pub threshold: u32,
    /// Current signer set version.
    pub signer_set_version: u32,
}
