#![no_std]
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Symbol, Vec, symbol_short, token};

mod errors;
mod storage;
mod types;

use errors::VestingError;
use storage::{
    get_admin, has_admin, set_admin, get_schedule_count, set_schedule_count,
    get_schedule, set_schedule, add_grantor_schedule, add_beneficiary_schedule,
    get_grantor_schedules, get_beneficiary_schedules,
    get_claim_history as storage_get_claim_history, add_claim_record,
    extend_instance_ttl, extend_schedule_ttl, extend_grantor_schedules_ttl,
    extend_beneficiary_schedules_ttl, extend_claim_history_ttl,
    get_pending_upgrade, set_pending_upgrade, clear_pending_upgrade, MIN_UPGRADE_DELAY,
};
use types::{VestingSchedule, VestingStatus, VestingProgress, ClaimRecord, PendingUpgrade};

#[contract]
pub struct VestingContract;

#[contractimpl]
impl VestingContract {
    /// Initialize the vesting contract with an admin.
    /// # Authorization Policy
    /// - **Caller:** Any address, provided they have the signature of the `admin` being set.
    /// - **Policy:** `admin.require_auth()` ensures the admin consents to being the administrator.
    pub fn initialize(env: Env, admin: Address) -> Result<(), VestingError> {
        if has_admin(&env) {
            return Err(VestingError::AlreadyInitialized);
        }
        admin.require_auth();
        set_admin(&env, &admin);
        set_schedule_count(&env, 0);

        env.events().publish(
            (symbol_short!("init"),),
            admin.clone(),
        );

        extend_instance_ttl(&env);

        Ok(())
    }

    /// Create a new vesting schedule with cliff + linear vesting.
    ///
    /// # Authorization Policy
    /// - **Caller:** The `grantor`.
    /// - **Policy:** `grantor.require_auth()` ensures the grantor authorizes the schedule creation and token transfer.
    ///
    /// # Arguments
    /// * `grantor` - The organization creating the schedule (must auth)
    /// * `beneficiary` - The person receiving vested tokens
    /// * `token` - The token to vest
    /// * `total_amount` - Total tokens to vest over the full period
    /// * `start_time` - When vesting begins (unix timestamp)
    /// * `cliff_duration` - Seconds before any tokens vest (cliff period)
    /// * `cliff_amount` - Amount that vests immediately at cliff time
    /// * `total_duration` - Total seconds for the full vesting period
    /// * `label` - A descriptor like "team", "advisor", "seed"
    /// * `revocable` - Whether the grantor can revoke unvested tokens
    pub fn create_schedule(
        env: Env,
        grantor: Address,
        beneficiary: Address,
        token: Address,
        total_amount: i128,
        start_time: u64,
        cliff_duration: u64,
        cliff_amount: i128,
        total_duration: u64,
        label: Symbol,
        revocable: bool,
    ) -> Result<u32, VestingError> {
        if !has_admin(&env) {
            return Err(VestingError::NotInitialized);
        }
        grantor.require_auth();

        if total_amount <= 0 {
            return Err(VestingError::InvalidAmount);
        }
        if total_duration == 0 {
            return Err(VestingError::InvalidSchedule);
        }
        if cliff_duration >= total_duration {
            return Err(VestingError::InvalidCliffDuration);
        }
        if cliff_amount < 0 || cliff_amount > total_amount {
            return Err(VestingError::InvalidAmount);
        }

        let schedule_id = get_schedule_count(&env);
        let schedule = VestingSchedule {
            id: schedule_id,
            grantor: grantor.clone(),
            beneficiary: beneficiary.clone(),
            token: token.clone(),
            total_amount,
            claimed_amount: 0,
            start_time,
            cliff_duration,
            cliff_amount,
            total_duration,
            label,
            status: VestingStatus::Active,
            revocable,
            revoked_at: None,
        };

        let token_client = token::Client::new(&env, &token);
        if token_client.balance(&grantor) < total_amount {
            return Err(VestingError::InsufficientBalance);
        }

        token_client.transfer(&grantor, &env.current_contract_address(), &total_amount);

        set_schedule(&env, schedule_id, &schedule);
        set_schedule_count(&env, schedule_id + 1);
        add_grantor_schedule(&env, &grantor, schedule_id);
        add_beneficiary_schedule(&env, &beneficiary, schedule_id);

        env.events().publish(
            (symbol_short!("v_create"), grantor.clone(), beneficiary.clone()),
            (total_amount, cliff_duration, total_duration),
        );

        extend_instance_ttl(&env);
        extend_schedule_ttl(&env, schedule_id);
        extend_grantor_schedules_ttl(&env, &grantor);
        extend_beneficiary_schedules_ttl(&env, &beneficiary);

        Ok(schedule_id)
    }

    /// Claim vested tokens. The beneficiary can claim any tokens that have vested
    /// according to the cliff + linear schedule.
    /// # Authorization Policy
    /// - **Caller:** The `beneficiary`.
    /// - **Policy:** `beneficiary.require_auth()` ensures the beneficiary authorizes the claim.
    pub fn claim(env: Env, beneficiary: Address, schedule_id: u32) -> Result<i128, VestingError> {
        if !has_admin(&env) {
            return Err(VestingError::NotInitialized);
        }
        beneficiary.require_auth();

        let mut schedule = get_schedule(&env, schedule_id)
            .ok_or(VestingError::ScheduleNotFound)?;

        if schedule.beneficiary != beneficiary {
            return Err(VestingError::Unauthorized);
        }
        if schedule.status == VestingStatus::FullyClaimed {
            return Err(VestingError::AlreadyFullyClaimed);
        }

        let vested = Self::calculate_vested(&env, &schedule);
        let claimable = vested - schedule.claimed_amount;

        if claimable <= 0 {
            return Err(VestingError::NothingToClaim);
        }

        schedule.claimed_amount += claimable;

        let mut is_fully_claimed = false;
        if schedule.claimed_amount >= schedule.total_amount {
            schedule.status = VestingStatus::FullyClaimed;
            is_fully_claimed = true;
        }

        let token_client = token::Client::new(&env, &schedule.token);
        if token_client.balance(&env.current_contract_address()) < claimable {
            return Err(VestingError::InsufficientBalance);
        }

        token_client.transfer(&env.current_contract_address(), &beneficiary, &claimable);
 
        set_schedule(&env, schedule_id, &schedule);
        add_claim_record(&env, schedule_id, claimable, env.ledger().timestamp());

        env.events().publish(
            (symbol_short!("v_claim"), beneficiary.clone(), schedule_id),
            claimable,
        );

        if is_fully_claimed {
            env.events().publish(
                (symbol_short!("v_fully"), schedule_id),
                (),
            );
        }

        extend_instance_ttl(&env);
        extend_schedule_ttl(&env, schedule_id);
        extend_claim_history_ttl(&env, schedule_id);

        Ok(claimable)
    }

    /// Revoke a vesting schedule. Only the grantor can revoke, and only if `revocable` is true.
    /// Unvested tokens are returned to the grantor. Already-vested tokens remain claimable.
    /// # Authorization Policy
    /// - **Caller:** The `grantor`.
    /// - **Policy:** `grantor.require_auth()` ensures the grantor authorizes the revocation.
    pub fn revoke(
        env: Env,
        grantor: Address,
        schedule_id: u32,
    ) -> Result<i128, VestingError> {
        if !has_admin(&env) {
            return Err(VestingError::NotInitialized);
        }
        grantor.require_auth();

        let mut schedule = get_schedule(&env, schedule_id)
            .ok_or(VestingError::ScheduleNotFound)?;

        if schedule.grantor != grantor {
            return Err(VestingError::Unauthorized);
        }
        if schedule.status == VestingStatus::Revoked {
            return Err(VestingError::ScheduleRevoked);
        }
        if schedule.status == VestingStatus::FullyClaimed {
            return Err(VestingError::AlreadyFullyClaimed);
        }
        if !schedule.revocable {
            return Err(VestingError::Unauthorized);
        }

        let vested = Self::calculate_vested(&env, &schedule);
        let unvested = schedule.total_amount - vested;

        schedule.status = VestingStatus::Revoked;
        schedule.total_amount = vested; // Cap at vested amount
        schedule.revoked_at = Some(env.ledger().timestamp());

        if unvested > 0 {
            let token_client = token::Client::new(&env, &schedule.token);
            if token_client.balance(&env.current_contract_address()) < unvested {
                return Err(VestingError::InsufficientBalance);
            }
            token_client.transfer(&env.current_contract_address(), &grantor, &unvested);
        }

        set_schedule(&env, schedule_id, &schedule);

        env.events().publish(
            (symbol_short!("v_revoke"), grantor.clone(), schedule_id),
            unvested,
        );

        extend_instance_ttl(&env);
        extend_schedule_ttl(&env, schedule_id);

        Ok(unvested)
    }

    // ── Internal Helpers ─────────────────────────────────────────

    /// Calculate the total amount of tokens that have vested by now.
    /// Uses cliff + linear vesting model.
    fn calculate_vested(env: &Env, schedule: &VestingSchedule) -> i128 {
        if schedule.status == VestingStatus::Revoked || schedule.status == VestingStatus::FullyClaimed {
            return schedule.total_amount;
        }

        let now = env.ledger().timestamp();

        if now < schedule.start_time {
            return 0;
        }

        let elapsed = now - schedule.start_time;

        // Before cliff: nothing is vested
        if elapsed < schedule.cliff_duration {
            return 0;
        }

        // After full duration: everything is vested
        if elapsed >= schedule.total_duration {
            return schedule.total_amount;
        }

        // Cliff amount vests immediately at cliff
        // Remaining amount (total - cliff_amount) vests linearly from cliff_duration to total_duration
        let remaining_amount = schedule.total_amount - schedule.cliff_amount;
        let vesting_duration = schedule.total_duration - schedule.cliff_duration;
        let time_since_cliff = elapsed - schedule.cliff_duration;

        let vested_linear = (remaining_amount * (time_since_cliff as i128)) / (vesting_duration as i128);
        
        schedule.cliff_amount + vested_linear
    }

    // ── Query Functions ──────────────────────────────────────────

    /// Get a specific vesting schedule by ID.
    pub fn get_schedule(env: Env, schedule_id: u32) -> Result<VestingSchedule, VestingError> {
        get_schedule(&env, schedule_id).ok_or(VestingError::ScheduleNotFound)
    }

    /// Get the vesting progress for a schedule.
    pub fn get_progress(env: Env, schedule_id: u32) -> Result<VestingProgress, VestingError> {
        let schedule = get_schedule(&env, schedule_id)
            .ok_or(VestingError::ScheduleNotFound)?;

        let vested = Self::calculate_vested(&env, &schedule);
        let claimable = vested - schedule.claimed_amount;

        Ok(VestingProgress {
            total_amount: schedule.total_amount,
            vested_amount: vested,
            claimed_amount: schedule.claimed_amount,
            claimable_amount: if claimable > 0 { claimable } else { 0 },
            status: schedule.status,
        })
    }

    /// Get all schedule IDs for a grantor.
    pub fn get_schedules_by_grantor(env: Env, grantor: Address) -> Vec<u32> {
        get_grantor_schedules(&env, &grantor)
    }

    /// Get all schedule IDs for a beneficiary.
    pub fn get_schedules_by_beneficiary(env: Env, beneficiary: Address) -> Vec<u32> {
        get_beneficiary_schedules(&env, &beneficiary)
    }
 
    /// Get the claim history for a vesting schedule.
    pub fn get_claim_history(env: Env, schedule_id: u32) -> Vec<ClaimRecord> {
        storage_get_claim_history(&env, schedule_id)
    }

    /// Get the total number of schedules created.
    pub fn get_schedule_count(env: Env) -> u32 {
        get_schedule_count(&env)
    }

    /// Get the admin address.
    pub fn get_admin(env: Env) -> Result<Address, VestingError> {
        if !has_admin(&env) {
            return Err(VestingError::NotInitialized);
        }
        Ok(get_admin(&env))
    }

    // ── Timelocked Upgrade ─────────────────────────────────────────

    /// Propose a WASM upgrade. Admin only. The upgrade cannot be executed
    /// until MIN_UPGRADE_DELAY has elapsed.
    /// # Authorization Policy
    /// - **Caller:** The `admin`.
    /// - **Policy:** `admin.require_auth()` is enforced.
    pub fn propose_upgrade(
        env: Env,
        admin: Address,
        wasm_hash: BytesN<32>,
        description: Symbol,
    ) -> Result<(), VestingError> {
        let stored_admin = get_admin(&env);
        if admin != stored_admin {
            return Err(VestingError::Unauthorized);
        }
        admin.require_auth();

        let pending = PendingUpgrade {
            wasm_hash,
            proposed_at: env.ledger().timestamp(),
        };
        set_pending_upgrade(&env, &pending);
        extend_instance_ttl(&env);

        env.events().publish(
            (symbol_short!("upg_prop"),),
            (description, pending.proposed_at),
        );

        Ok(())
    }

    /// Execute a pending WASM upgrade after the timelock delay has expired.
    /// # Authorization Policy
    /// - **Caller:** The `executor`.
    /// - **Policy:** `executor.require_auth()` ensures the transaction is authorized.
    pub fn execute_upgrade(env: Env, executor: Address) -> Result<(), VestingError> {
        executor.require_auth();

        let pending = get_pending_upgrade(&env).ok_or(VestingError::NoPendingUpgrade)?;
        let now = env.ledger().timestamp();

        if now < pending.proposed_at + MIN_UPGRADE_DELAY {
            return Err(VestingError::TimelockNotExpired);
        }

        env.deployer().update_current_contract_wasm(pending.wasm_hash);
        clear_pending_upgrade(&env);
        extend_instance_ttl(&env);

        env.events()
            .publish((symbol_short!("upg_exec"), executor), ());

        Ok(())
    }

    /// Get the pending upgrade proposal, if any.
    pub fn get_pending_upgrade(env: Env) -> Result<Option<PendingUpgrade>, VestingError> {
        if !has_admin(&env) {
            return Err(VestingError::NotInitialized);
        }
        Ok(get_pending_upgrade(&env))
    }
}

mod test;
