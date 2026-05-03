# DPoS Pallet for Substrate

A custom Substrate pallet implementing a Delegated Proof of Stake (DPoS) mechanism with validator registration, direct delegation, epoch-based reward distribution, validator set updates, and basic slashing.

This project focuses on fair reward attribution, epoch-based accounting, and preventing delegation free-riding. It was built as a blockchain systems project and combines implemented pallet logic with design decisions and future extensions that would matter in a more production-oriented version.

## Overview

The pallet allows:
- accounts to register as validators,
- token holders to delegate stake to validators,
- rewards to be distributed based on validator performance and stake contribution,
- misbehaving validators to be slashed through a basic slashing mechanism.

The main design goal is fairness: delegators should be rewarded for helping elect validators, not for joining a validator only after that validator is already active for the current epoch.

## Repository structure

```text
dpos-pallet/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── pallets/
│   └── dpos/
│       ├── Cargo.toml
│       └── src/
└── runtime/
    ├── Cargo.toml
    ├── build.rs
    └── src/
```

## Implemented features

- validator registration with self-stake
- validator deregistration
- direct delegation and undelegation
- epoch-based validator set updates
- epoch snapshots for reward eligibility
- block-author tracking per validator
- reward distribution based on authored blocks
- validator commission before delegator distribution
- basic slashing support
- runtime integration for reporting the new validator set

## Core architecture

### Validators and delegators
Validators participate in block production, while delegators support validators by staking tokens behind them. The value of delegation comes from helping a validator get elected. Because of that, a delegator who joins a validator in the middle of an epoch should not receive rewards for that epoch.

This is one of the key fairness principles behind the pallet: rewards should go to participants who contributed to validator election, not to late entrants free-riding on an already elected validator.

### Epochs
The system operates in fixed-length epochs. At each epoch boundary:
- block production is evaluated,
- rewards are distributed,
- validator sets may be updated,
- new snapshots are taken for the next reward cycle.

This allows the pallet to separate ongoing staking activity from reward eligibility and validator rotation.

### Reward distribution
Rewards are distributed based on the number of blocks authored by each validator during an epoch.

The flow is:
1. determine how many blocks each validator authored,
2. compute the reward share for each validator,
3. apply validator commission,
4. distribute the remaining rewards among eligible delegators,
5. use epoch snapshots to avoid rewarding delegators who joined too late.

### Slashing
A basic slashing function is implemented. The broader slashing design is intentionally discussed in more detail than what is currently implemented, because slashing is one of the areas that matters most for correctness, deterrence, and protocol credibility.

## Design considerations and reasoning

## Validators and delegators
Validators are nodes that participate in block production, while delegators are accounts that support validators by staking their tokens. The value of delegating comes from ensuring a chosen validator is elected. By helping a validator get elected, delegators bring value to the blockchain and should be rewarded accordingly.

If a delegator stakes with a validator in the middle of an epoch, they are taking advantage of an already elected validator and should not receive rewards for that epoch. This ensures fairness by requiring delegators to wait until the next epoch before earning rewards.

### Validator selection
Token holders vote to elect validators through direct delegation. Validators are selected based on the amount of stake delegated to them. Anyone can register as a validator with a minimum self-stake, and anyone can delegate to a validator.

Rewards are distributed at the end of each epoch to those who helped elect validators. If a delegator starts delegating to a validator mid-epoch, they must wait until the next epoch to receive rewards.

- **Deposit requirement:** Validators must stake a set amount to register as block producers. This self-stake requirement helps protect the system from spam and misuse. The self-stake should be high enough to make it costly for attackers to fill validator slots.
- **Validator identity:** In a more advanced production setting, more safeguards should be used, such as holding a specific NFT or another eligibility mechanism to ensure validators are trusted entities.

### Handling validator and delegator dynamics

#### Validator set flexibility *(not fully implemented yet)*
- **Filling validator slots:** If the number of elected validators falls below a required threshold, the system should fill the slots from a pool of standby validators, such as trusted validators configured in genesis.
- **Fallback mechanism:** The system should have preset fallback validators in case of insufficient validator registrations or inadequate validator performance, to preserve network security and functionality.
- **Non-updating validator set:** If the current set of elected validators is unsatisfactory due to poor performance or centralization concerns, the system should be able to keep the previous set until the next epoch instead of updating automatically.

#### Delegator payments
Efficiently managing payments to a large number of delegators is important for scalability and usability.

- **Batch processing:** Rewards are aggregated and processed at the end of each epoch rather than handled individually per block.
- **Fair distribution:** Delegators should be paid based on their contribution to validator election and the performance of their chosen validators. This includes proportional reward distribution based on stake and, in the future, potentially validator reputation.
- **Reputation system for validators (future idea):** Validators could be scored based on performance, honesty, and reliability, with stronger validators receiving better reward multipliers.
- **Automated distribution:** The current implementation automates delegator payouts at the end of each epoch.
- **Claim-based distribution (future improvement):** A claim-based model could reduce chain load and give delegators more control over payout timing.

### Flexible stake
In the current implementation, stakes are not locked for the full epoch. Delegators can delegate and undelegate at any time. To preserve fairness and prevent exploitation, rewards are distributed based on snapshots taken at the beginning of each epoch.

- **Reward eligibility:** Rewards are given to delegators who helped elect a validator. If a delegator unstakes or changes delegation mid-epoch, they are not eligible for rewards until the next epoch, because they are excluded from the previous validator’s reward snapshot.

This balances staking flexibility with anti-free-riding protections.

### Epochs
The system operates in epochs, each defined by a configurable number of blocks. In the current runtime configuration, the epoch duration is set to 100 blocks. New validators are reported at the end of each epoch so the active set can remain dynamic and reflect the network’s current state.

### Reward distribution
Rewards are distributed based on the number of blocks authored by each validator during an epoch. A portion of the rewards is allocated to the validator, and the remainder is distributed among delegators according to stake.

- Rewards are tied to the value of helping a validator get elected.
- Starting to delegate to an already elected validator mid-epoch does not create value for that epoch and should not earn immediate rewards.
- The reward amount per block is currently fixed.
- **Fee sharing (future idea):** Transaction fees could also be shared between validators and delegators.
- **Reward burning (future idea):** A configurable portion of rewards could be burned to introduce deflationary pressure and better align issuance with network activity.

#### Reward distribution mechanisms that incentivize decentralization *(future work)*
- **Higher reward multipliers for smaller validators:** This could encourage stake to flow toward smaller validators.
- **Diminishing returns for large stakes:** This could reduce centralization pressure by flattening rewards for already-dominant validators.

Together, these mechanisms could steer the system toward a more balanced stake distribution, where incentives favor decentralization rather than concentration.

### Slashing
Only a simple slashing function is currently implemented, but the intended broader logic is as follows.

Validators and delegators can both be slashed for behavior that jeopardizes blockchain security and integrity. Slashing acts as a deterrent and an accountability mechanism.

Validators can be slashed for:
- double signing,
- downtime,
- security breaches,
- failure to submit valid blocks,
- collusion and cartel formation,
- chain reorganization attacks.

Delegators could also be slashed for:
- supporting malicious validators,
- collusion,
- misreporting.

#### Slashing mechanism
The slashing mechanism should penalize validators and delegators proportionally to the severity of the offense.

- **Proportional slashing:** Larger or more harmful offenses should incur larger penalties.
- **Partial slashing:** Minor offenses could trigger partial penalties.
- **Complete slashing:** Severe or repeated offenses could justify removing the entire stake.

#### Defense window
To ensure fairness, validators subject to slashing should have a defense window during which they can appeal the slashing decision through some review mechanism or temporary hold process.

This balances deterrence with due process and would improve trust in the system.

## State transition design

### Storage items
- **`CurrentValidators`** — stores the current active validator set in a bounded vector.
- **`PotentialValidators`** — stores candidate validators and their self-stake. In a more mature version, this would likely be replaced with a more efficient bounded or counted structure.
- **`Delegators`** — maps delegators to their delegation details, including validator, amount, and the epoch in which delegation started.
- **`ValidatorStakes`** — tracks the cumulative stake for each validator.
- **`SnapshotDelegators`** — stores delegator snapshots used for reward distribution.
- **`BlockCount`** — tracks how many blocks each validator authored during the epoch.

### Dispatchable functions

- **`register_validator`** — registers a new validator after checking balance and holding the self-stake.
- **`unregister_validator`** — removes a validator, releases self-stake, and removes associated delegations. A more advanced version should handle active-validator exit timing more carefully.
- **`delegate`** — delegates stake to a validator, handling balance checks, validator existence, and reward-eligibility timing.
- **`undelegate`** — reduces or removes delegated stake and releases held balance.

### Hooks

- **`on_initialize`** — checks epoch boundaries, distributes rewards, updates validators, snapshots delegators, and resets block counts.
- **`on_finalize`** — identifies the block author and increments authored block count.

### Internal functions

- **`initialize_validators`** — initializes genesis validators and their balances.
- **`update_validators`** — sorts candidate validators by stake and selects the top set.
- **`snapshot_validators_delegators`** — takes reward snapshots for delegators backing active validators.

## Limitations and incomplete areas

- validator set fallback logic is discussed but not fully implemented,
- advanced slashing logic is mostly a design outline rather than a complete implementation,
- validator identity and trust gating are only proposed,
- claim-based rewards are not implemented,
- decentralization-oriented reward shaping is still conceptual,
- `PotentialValidators` could be replaced with a more efficient storage structure.

## Running tests

To run tests with logs:

```sh
RUST_LOG=debug cargo test --package pallet-dpos test_name -- --nocapture
```

To run all tests:

```sh
cargo test
```

## Notes

This repository was built from a Substrate template and then extended with a custom DPoS pallet and runtime integration. The template metadata has been updated, but parts of the runtime setup still intentionally reflect the educational/project nature of the repository.
