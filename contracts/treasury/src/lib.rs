#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, token, Address, BytesN, Env, Symbol, Vec};

mod errors;
mod storage;
mod types;

use errors::TreasuryError;
use storage::{
    get_admin, get_proposal_count, get_signers, get_threshold, get_withdrawal, has_admin,
    set_admin, set_proposal_count, set_signers, set_threshold, set_withdrawal,
    extend_instance_ttl, extend_withdrawal_ttl, extend_upgrade_proposal_ttl,
    extend_pending_admin_change_ttl, get_signer_set_version, set_signer_set_version,
    increment_signer_set_version, get_pause_state, set_pause_state, is_paused,
    get_signer_change_delay, set_signer_change_delay, get_pending_admin_change,
    set_pending_admin_change, get_upgrade_proposal, set_upgrade_proposal,
    get_upgrade_count, set_upgrade_count, clear_pending_admin_change,
    MIN_SIGNER_CHANGE_DELAY,
};
use types::{TreasuryConfig, WithdrawalRequest, WithdrawalStatus, PauseState, UpgradeProposal, UpgradeStatus, PendingAdminChange};

#[contract]
pub struct TreasuryContract;

#[contractimpl]
impl TreasuryContract {
    fn require_initialized(env: &Env) -> Result<(), TreasuryError> {
        if !has_admin(env) {
            return Err(TreasuryError::NotInitialized);
        }
        Ok(())
    }

    pub fn initialize(
        env: Env,
        admin: Address,
        signers: Vec<Address>,
        threshold: u32,
    ) -> Result<(), TreasuryError> {
        if has_admin(&env) {
            return Err(TreasuryError::AlreadyInitialized);
        }
        if threshold == 0 || threshold > signers.len() {
            return Err(TreasuryError::InvalidThreshold);
        }
        for i in 0..signers.len() {
            for j in (i + 1)..signers.len() {
                if signers.get(i).unwrap() == signers.get(j).unwrap() {
                    return Err(TreasuryError::DuplicateSigner);
                }
            }
        }
        admin.require_auth();
        set_admin(&env, &admin);
        set_signers(&env, &signers);
        set_threshold(&env, threshold);
        set_proposal_count(&env, 0);
        set_signer_set_version(&env, 0);
        set_pause_state(&env, &PauseState::Unpaused);
        set_signer_change_delay(&env, storage::DEFAULT_SIGNER_CHANGE_DELAY);
        extend_instance_ttl(&env);
        env.events()
            .publish((symbol_short!("init"),), admin.clone());
        Ok(())
    }

    pub fn deposit(
        env: Env,
        from: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        if is_paused(&env) {
            return Err(TreasuryError::Paused);
        }
        if amount <= 0 {
            return Err(TreasuryError::InvalidAmount);
        }
        from.require_auth();
        let contract_address = env.current_contract_address();
        token::Client::new(&env, &token).transfer(&from, &contract_address, &amount);
        env.events()
            .publish((symbol_short!("deposit"), from.clone()), amount);
        Ok(())
    }

    pub fn create_withdrawal(
        env: Env,
        proposer: Address,
        token: Address,
        recipient: Address,
        amount: i128,
        memo: Symbol,
    ) -> Result<u32, TreasuryError> {
        Self::require_initialized(&env)?;
        if is_paused(&env) {
            return Err(TreasuryError::Paused);
        }
        proposer.require_auth();
        let signers = get_signers(&env);
        let mut is_signer = false;
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == proposer {
                is_signer = true;
                break;
            }
        }
        if !is_signer {
            return Err(TreasuryError::NotASigner);
        }
        if amount <= 0 {
            return Err(TreasuryError::InvalidAmount);
        }
        let proposal_id = get_proposal_count(&env);
        let signer_set_version = get_signer_set_version(&env);
        let threshold = get_threshold(&env);
        let mut approvals = Vec::new(&env);
        approvals.push_back(proposer.clone());
        let status = if threshold == 1 { WithdrawalStatus::Approved } else { WithdrawalStatus::Pending };
        let request = WithdrawalRequest {
            id: proposal_id,
            proposer: proposer.clone(),
            token,
            recipient,
            amount,
            memo,
            approvals,
            status,
            created_at: env.ledger().timestamp(),
            signer_set_version,
        };
        set_withdrawal(&env, proposal_id, &request);
        set_proposal_count(&env, proposal_id + 1);
        extend_instance_ttl(&env);
        extend_withdrawal_ttl(&env, proposal_id);
        env.events()
            .publish((symbol_short!("w_create"), proposer.clone()), proposal_id);
        Ok(proposal_id)
    }

    pub fn approve_withdrawal(
        env: Env,
        signer: Address,
        proposal_id: u32,
    ) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        signer.require_auth();
        let signers = get_signers(&env);
        let mut is_signer = false;
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == signer {
                is_signer = true;
                break;
            }
        }
        if !is_signer {
            return Err(TreasuryError::NotASigner);
        }
        let mut request =
            get_withdrawal(&env, proposal_id).ok_or(TreasuryError::ProposalNotFound)?;
        if request.status != WithdrawalStatus::Pending {
            return Err(TreasuryError::ProposalNotPending);
        }
        for i in 0..request.approvals.len() {
            if request.approvals.get(i).unwrap() == signer {
                return Err(TreasuryError::AlreadyApproved);
            }
        }
        request.approvals.push_back(signer.clone());
        let threshold = get_threshold(&env);
        if request.approvals.len() >= threshold {
            request.status = WithdrawalStatus::Approved;
        }
        set_withdrawal(&env, proposal_id, &request);
        extend_withdrawal_ttl(&env, proposal_id);
        env.events()
            .publish((symbol_short!("approve"), signer.clone()), proposal_id);
        Ok(())
    }

    pub fn execute_withdrawal(
        env: Env,
        executor: Address,
        proposal_id: u32,
    ) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        executor.require_auth();
        let mut request =
            get_withdrawal(&env, proposal_id).ok_or(TreasuryError::ProposalNotFound)?;
        if request.status != WithdrawalStatus::Approved {
            return Err(TreasuryError::ProposalNotApproved);
        }
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &request.token);
        let contract_balance = token_client.balance(&contract_address);
        if contract_balance < request.amount {
            return Err(TreasuryError::InsufficientBalance);
        }
        token_client.transfer(&contract_address, &request.recipient, &request.amount);
        request.status = WithdrawalStatus::Executed;
        set_withdrawal(&env, proposal_id, &request);
        extend_withdrawal_ttl(&env, proposal_id);
        env.events().publish(
            (symbol_short!("w_exec"), request.recipient.clone()),
            request.amount,
        );
        Ok(())
    }

    pub fn add_signer(env: Env, admin: Address, new_signer: Address) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        let stored_admin = get_admin(&env);
        if admin != stored_admin {
            return Err(TreasuryError::Unauthorized);
        }
        admin.require_auth();
        let mut signers = get_signers(&env);
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == new_signer {
                return Err(TreasuryError::AlreadyASigner);
            }
        }
        signers.push_back(new_signer.clone());
        set_signers(&env, &signers);
        let new_version = increment_signer_set_version(&env);
        let threshold = get_threshold(&env);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("sa"), admin.clone(), new_signer.clone(), threshold, new_version),
            (signers.len(),),
        );
        Ok(())
    }

    pub fn remove_signer(env: Env, admin: Address, signer: Address) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        let stored_admin = get_admin(&env);
        if admin != stored_admin {
            return Err(TreasuryError::Unauthorized);
        }
        admin.require_auth();
        let signers = get_signers(&env);
        let threshold = get_threshold(&env);
        if signers.len() <= threshold {
            return Err(TreasuryError::InvalidThreshold);
        }
        let mut new_signers = Vec::new(&env);
        let mut found = false;
        for i in 0..signers.len() {
            let s = signers.get(i).unwrap();
            if s == signer {
                found = true;
            } else {
                new_signers.push_back(s);
            }
        }
        if !found {
            return Err(TreasuryError::NotASigner);
        }
        set_signers(&env, &new_signers);
        let new_version = increment_signer_set_version(&env);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("sr"), admin.clone(), signer.clone(), threshold, new_version),
            (new_signers.len(),),
        );
        Ok(())
    }

    pub fn update_threshold(
        env: Env,
        admin: Address,
        new_threshold: u32,
    ) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        let stored_admin = get_admin(&env);
        if admin != stored_admin {
            return Err(TreasuryError::Unauthorized);
        }
        admin.require_auth();
        let signers = get_signers(&env);
        if new_threshold == 0 || new_threshold > signers.len() {
            return Err(TreasuryError::InvalidThreshold);
        }
        set_threshold(&env, new_threshold);
        let signer_set_version = get_signer_set_version(&env);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("t_upd"), admin.clone(), new_threshold, signers.len(), signer_set_version),
            (),
        );
        Ok(())
    }

    // ── Pause Control ──────────────────────────────────────────────

    pub fn propose_pause(
        env: Env,
        proposer: Address,
        reason: Symbol,
    ) -> Result<u32, TreasuryError> {
        Self::require_initialized(&env)?;
        proposer.require_auth();
        let signers = get_signers(&env);
        let mut is_signer = false;
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == proposer {
                is_signer = true;
                break;
            }
        }
        if !is_signer {
            return Err(TreasuryError::NotASigner);
        }
        let proposal_id = get_upgrade_count(&env);
        let threshold = get_threshold(&env);
        let mut approvals = Vec::new(&env);
        approvals.push_back(proposer.clone());
        let status = if threshold == 1 { UpgradeStatus::Approved } else { UpgradeStatus::Pending };
        let proposal = UpgradeProposal {
            id: proposal_id,
            proposer: proposer.clone(),
            wasm_hash: BytesN::from_array(&env, &[0; 32]),
            description: reason.clone(),
            approvals,
            status,
            signer_set_version: get_signer_set_version(&env),
            created_at: env.ledger().timestamp(),
        };
        set_upgrade_proposal(&env, proposal_id, &proposal);
        set_upgrade_count(&env, proposal_id + 1);
        extend_upgrade_proposal_ttl(&env, proposal_id);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("pp"), proposer),
            (proposal_id, reason),
        );
        Ok(proposal_id)
    }

    pub fn approve_pause(
        env: Env,
        signer: Address,
        proposal_id: u32,
    ) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        signer.require_auth();
        let mut proposal = get_upgrade_proposal(&env, proposal_id)
            .ok_or(TreasuryError::UpgradeProposalNotFound)?;
        if proposal.status != UpgradeStatus::Pending {
            return Err(TreasuryError::UpgradeProposalNotPending);
        }
        for i in 0..proposal.approvals.len() {
            if proposal.approvals.get(i).unwrap() == signer {
                return Err(TreasuryError::AlreadyApproved);
            }
        }
        proposal.approvals.push_back(signer.clone());
        let threshold = get_threshold(&env);
        if proposal.approvals.len() >= threshold {
            proposal.status = UpgradeStatus::Approved;
            set_pause_state(&env, &PauseState::Paused);
            env.events().publish(
                (symbol_short!("pe"), signer),
                proposal.description.clone(),
            );
        }
        set_upgrade_proposal(&env, proposal_id, &proposal);
        extend_upgrade_proposal_ttl(&env, proposal_id);
        Ok(())
    }

    pub fn propose_unpause(
        env: Env,
        proposer: Address,
        reason: Symbol,
    ) -> Result<u32, TreasuryError> {
        Self::require_initialized(&env)?;
        proposer.require_auth();
        let signers = get_signers(&env);
        let mut is_signer = false;
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == proposer {
                is_signer = true;
                break;
            }
        }
        if !is_signer {
            return Err(TreasuryError::NotASigner);
        }
        let proposal_id = get_upgrade_count(&env);
        let threshold = get_threshold(&env);
        let mut approvals = Vec::new(&env);
        approvals.push_back(proposer.clone());
        let status = if threshold == 1 { UpgradeStatus::Approved } else { UpgradeStatus::Pending };
        let proposal = UpgradeProposal {
            id: proposal_id,
            proposer: proposer.clone(),
            wasm_hash: BytesN::from_array(&env, &[0; 32]),
            description: reason.clone(),
            approvals,
            status,
            signer_set_version: get_signer_set_version(&env),
            created_at: env.ledger().timestamp(),
        };
        set_upgrade_proposal(&env, proposal_id, &proposal);
        set_upgrade_count(&env, proposal_id + 1);
        extend_upgrade_proposal_ttl(&env, proposal_id);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("up"), proposer),
            (proposal_id, reason),
        );
        Ok(proposal_id)
    }

    pub fn approve_unpause(
        env: Env,
        signer: Address,
        proposal_id: u32,
    ) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        signer.require_auth();
        let mut proposal = get_upgrade_proposal(&env, proposal_id)
            .ok_or(TreasuryError::UpgradeProposalNotFound)?;
        if proposal.status != UpgradeStatus::Pending {
            return Err(TreasuryError::UpgradeProposalNotPending);
        }
        for i in 0..proposal.approvals.len() {
            if proposal.approvals.get(i).unwrap() == signer {
                return Err(TreasuryError::AlreadyApproved);
            }
        }
        proposal.approvals.push_back(signer.clone());
        let threshold = get_threshold(&env);
        if proposal.approvals.len() >= threshold {
            proposal.status = UpgradeStatus::Approved;
            set_pause_state(&env, &PauseState::Unpaused);
            env.events().publish(
                (symbol_short!("ue"), signer),
                proposal.description.clone(),
            );
        }
        set_upgrade_proposal(&env, proposal_id, &proposal);
        extend_upgrade_proposal_ttl(&env, proposal_id);
        Ok(())
    }

    // ── Signer-Controlled Upgrades ────────────────────────────────────

    pub fn propose_upgrade(
        env: Env,
        proposer: Address,
        wasm_hash: BytesN<32>,
        description: Symbol,
    ) -> Result<u32, TreasuryError> {
        Self::require_initialized(&env)?;
        proposer.require_auth();
        let signers = get_signers(&env);
        let mut is_signer = false;
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == proposer {
                is_signer = true;
                break;
            }
        }
        if !is_signer {
            return Err(TreasuryError::NotASigner);
        }
        let proposal_id = get_upgrade_count(&env);
        let threshold = get_threshold(&env);
        let mut approvals = Vec::new(&env);
        approvals.push_back(proposer.clone());
        let status = if threshold == 1 { UpgradeStatus::Approved } else { UpgradeStatus::Pending };
        let proposal = UpgradeProposal {
            id: proposal_id,
            proposer,
            wasm_hash,
            description: description.clone(),
            approvals,
            status,
            signer_set_version: get_signer_set_version(&env),
            created_at: env.ledger().timestamp(),
        };
        set_upgrade_proposal(&env, proposal_id, &proposal);
        set_upgrade_count(&env, proposal_id + 1);
        extend_upgrade_proposal_ttl(&env, proposal_id);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("ug_prop"),),
            (proposal_id, description),
        );
        Ok(proposal_id)
    }

    pub fn approve_upgrade(
        env: Env,
        signer: Address,
        proposal_id: u32,
    ) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        signer.require_auth();
        let mut proposal = get_upgrade_proposal(&env, proposal_id)
            .ok_or(TreasuryError::UpgradeProposalNotFound)?;
        if proposal.status != UpgradeStatus::Pending {
            return Err(TreasuryError::UpgradeProposalNotPending);
        }
        let signers = get_signers(&env);
        let mut is_signer = false;
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == signer {
                is_signer = true;
                break;
            }
        }
        if !is_signer {
            return Err(TreasuryError::NotASigner);
        }
        for i in 0..proposal.approvals.len() {
            if proposal.approvals.get(i).unwrap() == signer {
                return Err(TreasuryError::AlreadyApproved);
            }
        }
        proposal.approvals.push_back(signer.clone());
        let threshold = get_threshold(&env);
        if proposal.approvals.len() >= threshold {
            proposal.status = UpgradeStatus::Approved;
        }
        set_upgrade_proposal(&env, proposal_id, &proposal);
        extend_upgrade_proposal_ttl(&env, proposal_id);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("ua"), signer),
            proposal_id,
        );
        Ok(())
    }

    pub fn execute_upgrade(
        env: Env,
        executor: Address,
        proposal_id: u32,
    ) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        executor.require_auth();
        let proposal = get_upgrade_proposal(&env, proposal_id)
            .ok_or(TreasuryError::UpgradeProposalNotFound)?;
        if proposal.status != UpgradeStatus::Approved {
            return Err(TreasuryError::UpgradeProposalNotPending);
        }
        env.deployer().update_current_contract_wasm(proposal.wasm_hash);
        env.events().publish(
            (symbol_short!("ug_exec"), executor),
            (proposal_id, proposal.description),
        );
        Ok(())
    }

    // ── Timelocked Admin/Signer Changes ────────────────────────────

    pub fn propose_admin_change(
        env: Env,
        admin: Address,
        new_admin: Address,
    ) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        let stored_admin = get_admin(&env);
        if admin != stored_admin {
            return Err(TreasuryError::Unauthorized);
        }
        admin.require_auth();
        let delay = get_signer_change_delay(&env);
        let effective_at = env.ledger().timestamp() + delay;
        let pending_change = PendingAdminChange {
            new_admin,
            signer_set_version: get_signer_set_version(&env),
            effective_at,
        };
        set_pending_admin_change(&env, &pending_change);
        extend_pending_admin_change_ttl(&env);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("ac"), admin),
            (effective_at, pending_change.new_admin),
        );
        Ok(())
    }

    pub fn execute_admin_change(env: Env) -> Result<Address, TreasuryError> {
        Self::require_initialized(&env)?;
        let pending = get_pending_admin_change(&env)
            .ok_or(TreasuryError::NoPendingAdminChange)?;
        let now = env.ledger().timestamp();
        if now < pending.effective_at {
            return Err(TreasuryError::TimelockNotExpired);
        }
        let old_admin = get_admin(&env);
        set_admin(&env, &pending.new_admin.clone());
        clear_pending_admin_change(&env);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("ad"), old_admin),
            pending.new_admin.clone(),
        );
        Ok(pending.new_admin)
    }

    pub fn cancel_admin_change(env: Env, admin: Address) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        let stored_admin = get_admin(&env);
        if admin != stored_admin {
            return Err(TreasuryError::Unauthorized);
        }
        admin.require_auth();
        clear_pending_admin_change(&env);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("acc"), admin),
            (),
        );
        Ok(())
    }

    pub fn propose_signer_add(
        env: Env,
        admin: Address,
        new_signer: Address,
    ) -> Result<u32, TreasuryError> {
        Self::require_initialized(&env)?;
        let stored_admin = get_admin(&env);
        if admin != stored_admin {
            return Err(TreasuryError::Unauthorized);
        }
        admin.require_auth();
        let mut signers = get_signers(&env);
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == new_signer {
                return Err(TreasuryError::AlreadyASigner);
            }
        }
        let proposal_id = get_upgrade_count(&env);
        let threshold = get_threshold(&env);
        let mut approvals = Vec::new(&env);
        approvals.push_back(admin.clone());
        let proposal = UpgradeProposal {
            id: proposal_id,
            proposer: admin.clone(),
            wasm_hash: BytesN::from_array(&env, &[0; 32]),
            description: symbol_short!("sa"),
            approvals,
            status: UpgradeStatus::PendingSignerChange,
            signer_set_version: get_signer_set_version(&env),
            created_at: env.ledger().timestamp(),
        };
        let delay = get_signer_change_delay(&env);
        set_upgrade_proposal(&env, proposal_id, &proposal);
        set_upgrade_count(&env, proposal_id + 1);
        extend_upgrade_proposal_ttl(&env, proposal_id);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("sa_prop"), admin, new_signer),
            (proposal_id, threshold, delay),
        );
        Ok(proposal_id)
    }

    pub fn schedule_signer_remove(
        env: Env,
        admin: Address,
        signer: Address,
    ) -> Result<u32, TreasuryError> {
        Self::require_initialized(&env)?;
        let stored_admin = get_admin(&env);
        if admin != stored_admin {
            return Err(TreasuryError::Unauthorized);
        }
        admin.require_auth();
        let signers = get_signers(&env);
        let threshold = get_threshold(&env);
        let mut found = false;
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == signer {
                found = true;
                break;
            }
        }
        if !found {
            return Err(TreasuryError::NotASigner);
        }
        if signers.len() <= threshold {
            return Err(TreasuryError::InvalidThreshold);
        }
        let proposal_id = get_upgrade_count(&env);
        let mut approvals = Vec::new(&env);
        approvals.push_back(admin.clone());
        let proposal = UpgradeProposal {
            id: proposal_id,
            proposer: admin.clone(),
            wasm_hash: BytesN::from_array(&env, &[0; 32]),
            description: symbol_short!("sr"),
            approvals,
            status: UpgradeStatus::PendingSignerChange,
            signer_set_version: get_signer_set_version(&env),
            created_at: env.ledger().timestamp(),
        };
        set_upgrade_proposal(&env, proposal_id, &proposal);
        set_upgrade_count(&env, proposal_id + 1);
        extend_upgrade_proposal_ttl(&env, proposal_id);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("sr_sch"), admin, signer),
            (proposal_id, threshold),
        );
        Ok(proposal_id)
    }

    pub fn execute_signer_change(
        env: Env,
        executor: Address,
        proposal_id: u32,
    ) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        executor.require_auth();
        let proposal = get_upgrade_proposal(&env, proposal_id)
            .ok_or(TreasuryError::UpgradeProposalNotFound)?;
        if proposal.status != UpgradeStatus::PendingSignerChange {
            return Err(TreasuryError::UpgradeProposalNotPending);
        }
        let delay = get_signer_change_delay(&env);
        let now = env.ledger().timestamp();
        if now < proposal.created_at + delay {
            return Err(TreasuryError::TimelockNotExpired);
        }
        extend_instance_ttl(&env);
        Ok(())
    }

    pub fn propose_emergency_admin_change(
        env: Env,
        proposer: Address,
        _new_admin: Address,
    ) -> Result<u32, TreasuryError> {
        Self::require_initialized(&env)?;
        proposer.require_auth();
        let signers = get_signers(&env);
        let mut is_signer = false;
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == proposer {
                is_signer = true;
                break;
            }
        }
        if !is_signer {
            return Err(TreasuryError::NotASigner);
        }
        let proposal_id = get_upgrade_count(&env);
        let threshold = get_threshold(&env);
        let mut approvals = Vec::new(&env);
        approvals.push_back(proposer.clone());
        let status = if threshold == 1 { UpgradeStatus::Approved } else { UpgradeStatus::Pending };
        let proposal = UpgradeProposal {
            id: proposal_id,
            proposer,
            wasm_hash: BytesN::from_array(&env, &[0; 32]),
            description: symbol_short!("eac"),
            approvals,
            status,
            signer_set_version: get_signer_set_version(&env),
            created_at: env.ledger().timestamp(),
        };
        set_upgrade_proposal(&env, proposal_id, &proposal);
        set_upgrade_count(&env, proposal_id + 1);
        extend_upgrade_proposal_ttl(&env, proposal_id);
        extend_instance_ttl(&env);
        Ok(proposal_id)
    }

    pub fn execute_emergency_admin_change(
        env: Env,
        executor: Address,
        proposal_id: u32,
    ) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        executor.require_auth();
        let proposal = get_upgrade_proposal(&env, proposal_id)
            .ok_or(TreasuryError::UpgradeProposalNotFound)?;
        if proposal.status != UpgradeStatus::Approved {
            return Err(TreasuryError::UpgradeProposalNotPending);
        }
        let signers = get_signers(&env);
        let mut found = false;
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == proposal.proposer {
                found = true;
                break;
            }
        }
        if !found {
            return Err(TreasuryError::Unauthorized);
        }
        let old_admin = get_admin(&env);
        set_admin(&env, &executor);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("eacd"), executor.clone()),
            old_admin.clone(),
        );
        Ok(())
    }

    pub fn update_signer_change_delay(
        env: Env,
        admin: Address,
        new_delay: u64,
    ) -> Result<(), TreasuryError> {
        Self::require_initialized(&env)?;
        let stored_admin = get_admin(&env);
        if admin != stored_admin {
            return Err(TreasuryError::Unauthorized);
        }
        admin.require_auth();
        if new_delay < MIN_SIGNER_CHANGE_DELAY {
            return Err(TreasuryError::InvalidTimelock);
        }
        set_signer_change_delay(&env, new_delay);
        extend_instance_ttl(&env);
        env.events().publish(
            (symbol_short!("du"), admin),
            new_delay,
        );
        Ok(())
    }

    // ── Query Functions ──────────────────────────────────────────────

    pub fn get_admin(env: Env) -> Result<Address, TreasuryError> {
        Self::require_initialized(&env)?;
        Ok(get_admin(&env))
    }

    pub fn get_signers(env: Env) -> Result<Vec<Address>, TreasuryError> {
        Self::require_initialized(&env)?;
        Ok(get_signers(&env))
    }

    pub fn get_threshold(env: Env) -> Result<u32, TreasuryError> {
        Self::require_initialized(&env)?;
        Ok(get_threshold(&env))
    }

    pub fn get_withdrawal(env: Env, proposal_id: u32) -> Result<WithdrawalRequest, TreasuryError> {
        get_withdrawal(&env, proposal_id).ok_or(TreasuryError::ProposalNotFound)
    }

    pub fn get_proposal_count(env: Env) -> Result<u32, TreasuryError> {
        Self::require_initialized(&env)?;
        Ok(get_proposal_count(&env))
    }

    pub fn get_config(env: Env) -> Result<TreasuryConfig, TreasuryError> {
        Self::require_initialized(&env)?;
        Ok(TreasuryConfig {
            admin: get_admin(&env),
            signers: get_signers(&env),
            threshold: get_threshold(&env),
            proposal_count: get_proposal_count(&env),
            paused: is_paused(&env),
        })
    }

    pub fn get_pause_state(env: Env) -> Result<PauseState, TreasuryError> {
        Self::require_initialized(&env)?;
        Ok(get_pause_state(&env))
    }

    pub fn get_pending_admin_change(env: Env) -> Result<PendingAdminChange, TreasuryError> {
        Self::require_initialized(&env)?;
        get_pending_admin_change(&env).ok_or(TreasuryError::NoPendingAdminChange)
    }

    pub fn get_upgrade_proposal(env: Env, proposal_id: u32) -> Result<UpgradeProposal, TreasuryError> {
        Self::require_initialized(&env)?;
        get_upgrade_proposal(&env, proposal_id).ok_or(TreasuryError::UpgradeProposalNotFound)
    }

    pub fn get_signer_change_delay(env: Env) -> Result<u64, TreasuryError> {
        Self::require_initialized(&env)?;
        Ok(get_signer_change_delay(&env))
    }

    pub fn get_upgrade_count(env: Env) -> Result<u32, TreasuryError> {
        Self::require_initialized(&env)?;
        Ok(get_upgrade_count(&env))
    }
}

mod test;
