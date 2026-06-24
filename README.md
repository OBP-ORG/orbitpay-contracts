# OrbitPay Contracts

Soroban smart contracts powering the OrbitPay protocol — decentralized payroll, treasury, vesting, and governance on Stellar.

## Contracts

### Treasury
Multi-signature treasury contract with secure fund management and withdrawal approvals.

**Key Features:**
- Multi-signature approval system with configurable thresholds
- Unique signer validation to prevent duplicate signers
- Signer set versioning for deterministic approval checks
- Threshold 1 immediate approval for single-signer scenarios
- Comprehensive event emission for signer changes and threshold updates
- Pending withdrawal resilience during signer rotation

**Documentation:**
- See [contracts/treasury/INVARIANTS.md](contracts/treasury/INVARIANTS.md) for detailed invariants and security considerations

### Payroll Stream
Streaming payment contract for continuous payroll distributions.

### Vesting
Token vesting contract for time-based token releases.

### Governance
Governance contract for protocol decision-making.

## Build

```bash
cargo build --all
cargo test --all
```

To test individual contracts:
```bash
cargo test -p treasury
cargo test -p payroll_stream
cargo test -p vesting
cargo test -p governance
```

## License

MIT
