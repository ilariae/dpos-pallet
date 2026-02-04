# Direct Delegation Proof of Stake

This pallet implements a Delegated Proof of Stake (DPoS) system for a Substrate-based blockchain. 

It allows:
- accounts to register as validators
- token holders to delegate stake to validators
- rewards to be distributed based on validator performance and stake contribution
- slashing of misbehaving validators

The implementation focuses on fair reward attribution, epoch-based accounting, and preventing delegation free-riding.

## Running Tests with Logs

To enable debug logs while running tests:

```sh
RUST_LOG=debug cargo test --package pallet-dpos test_name -- --nocapture
```

## High-Level Architecture

### Validators & Delegators
- Validators self-stake to register and participate in block production
- Delegators stake tokens to support validators and share rewards
- Validator selection is stake-weighted and recalculated each epoch

### Epochs
- The system operates in fixed-length epochs (configurable)
- At each epoch boundary:
  - block production is evaluated
  - rewards are distributed
  - validator sets may be updated

### Reward Distribution
- Rewards are proportional to blocks authored during an epoch
- Validator commission is applied before delegator distribution
- Delegators are paid based on stake and eligibility snapshot

### Slashing
- A basic slashing mechanism is implemented
- Design considers proportional slashing, escalation, and deterrence
- Advanced slashing logic is outlined but not fully implemented

## State & Transitions

The pallet maintains explicit storage for:
- current and potential validators,
- delegator relationships and stake snapshots,
- block production tracking per epoch.

State transitions are handled through:
- dispatchable calls (register, delegate, undelegate),
- hooks executed at block initialization/finalization.


## Considerations

### Validators and Delegators 

Validators are nodes that participate in block production, while delegators are accounts that support validators by staking their tokens. The value of delegating comes from ensuring your chosen validator is elected. By helping a validator get elected, delegators bring value to the blockchain and should be rewarded accordingly. If a delegator stakes with a validator in the middle of an epoch, they are taking advantage of an already elected validator and should not receive rewards for that epoch. This ensures fairness by requiring delegators to wait until the next epoch before earning rewards.

#### Validator Selection

Token holders vote to elect validators through direct delegation. Validators are selected based on the amount of stake delegated to them. Anyone can register as a validator with a minimum self-stake, and anyone can delegate to a validator. Rewards are distributed at the end of each epoch to those who helped elect validators. If a delegator starts delegating to a validator mid-epoch, they must wait until the next epoch to receive rewards.

- Deposit Requirement: Validators must stake a set amount to be able to register as block producers. This self-stake requirement helps protect the system from spam and misuse. The self-stake needs to be high enough to make it costly for attackers to fill up the validator spots.
- Validator Identity: In a more advanced stage of production, more safeguards should be used, such as holding a particular NFT to be eligible to register as a validator, or other means to guarantee that validators are trusted entities. 

#### Handling Validator and Delegator Dynamics 
**Validator set flexibility**: mechanisms for handling dynamic adjustments to the validator set. (Not implemented yet)
- Filling validator slots: If the number of elected validators falls below a required threshold, the system should fill the slots from a pool of standby validators (could be the initial trusted validators set in the genesis block)
- Fallback Mechanism: The system should have fallback validators preset (trusted entities) in case of insufficient validator registrations or inadequate validator performance to guarantee the network's security and functionality.
- Non-updating validator set:  If the current set of elected validators is considered unsatisfactory (e.g., due to poor performance or centralization concerns), the system should allow for the possibility of not updating the validator set until the next epoch, maintaining the previous set.

**Delegator payments**: Efficiently managing payments to a large number of delegators is crucial for the scalability and usability of the system. 
- Batch processing for distributing rewards to delegators to minimize the computational load and transaction costs. Instead of processing payments individually per block, payments are aggregated and batched at the end of each epoch.
- Fair Distribution: Ensuring that all delegators are paid fairly based on their contributions to the election and performance of their chosen validators. This includes proportional reward distribution based on the amount staked and (in the future) the reputation (performance metrics) of the validator.
	- reputation system for validators: scores validators based on performance, honesty, and reliability → higher rewards to high-performing, reputable validators (TODO)
- Automated distribution: Currently, payments to delegators are automated, ensuring timely and accurate payments at the end of each epoch. 
→ Claim-Based Distribution (Future Improvement): Ideally, the system would allow delegators to claim their rewards. Removing automated payouts reduces load on the blockchain and gives delegators greater control over when they receive their rewards.

### Flexible stake
In the current implementation, there is no lock on stakes. Delegators can delegate and undelegate their stakes at any time. However, to ensure fairness and prevent exploitation of the system, the rewards are distributed based on snapshots taken at the beginning of each epoch.
- Reward Eligibility: Rewards are given to delegators who helped in the election of a validator. If a delegator unstakes or changes their delegation mid-epoch, they will not be eligible for rewards until the next epoch. This is because they will be excluded from the snapshot of their previous validator for the current epoch.
This approach balances flexibility in staking with the need to prevent free-riding on already elected validators, ensuring that the value brought to the blockchain by supporting validators is appropriately rewarded.

### Epochs
The system operates in epochs, each defined by a configurable number of blocks (currently 100). New validators are reported at the end of each epoch, ensuring the validator set remains dynamic and reflects the network's current state.

### Reward Distribution
Rewards are distributed based on the number of blocks authored by each validator during an epoch. A portion of the rewards is allocated to the validator (currently 30%), and the rest is distributed among their delegators based on the amount staked.
- Rewards are tied to the value of helping a validator get elected. Starting to delegate to an already elected validator mid-epoch does not bring value to the blockchain and thus doesn't earn rewards until the next epoch.
- Currently, the reward amount per block is fixed. 
- Fee Sharing: In addition to block rewards, transaction fees could be shared between validators and delegators to better align incentives with network usage and throughput.
- Reward Burning: A configurable portion of rewards could be burned to introduce deflationary pressure, discourage excessive block production purely for rewards, and better align issuance with network activity.

#### Reward Distribution Mechanisms that incentivize decentralization (TODO)
- Higher Reward Multiplier for Smaller Validators: Incentivizes staking to smaller validators and discourages centralization around a few.
- Diminishing Returns for Large Stakes: Apply diminishing returns for validators with excessive stakes to promote a more even stake distribution across the network.

Together, these mechanisms aim to steer the system toward a more balanced stake distribution. The reward function is intended to follow a curve that increases rapidly for low stake values and gradually flattens as the stake grows, converging toward a midrange equilibrium that favors decentralization.

### Slashing 
Only a simple slashing function was implemented, but the logic should be the following (TODO)
Validators and delegators can both be slashed for behaviors that jeopardize the blockchain's security and integrity. Slashing serves as a deterrent to malicious activities and enforces accountability.

Validators can be slashed for misbehavior:
- Double signing: Signing multiple blocks for the same height.
- Downtime: Failing to participate in block production for an extended period.
- Security breaches
- Failure to submit valid blocks: Consistently failing to produce valid blocks.
- Collusion and cartel formation: Engaging in activities that centralize power and undermine network decentralization. → The reward mechanism not implemented, as previously talked about, would help prevent centralization around certain validators and discourage collusion  
- Chain reorganization attacks: Participating in attempts to reorganize the blockchain for malicious purposes.

Delegators can be slashed for misbehavior as well as:
- Supporting malicious validators
- Collusion
- Misreporting

#### Slashing Mechanism
The slashing mechanism should be designed to penalize both validators and delegators based on the severity of their offense, with penalties proportional to the gravity of the misconduct. 
- Proportional Slashing: The amount slashed is proportional to the stake involved, meaning larger stakes face higher penalties.
	- Partial Slashing: For less severe offenses, only a portion of the stake is slashed. This can be used as a warning or for minor protocol breaches.
	- Complete Slashing: For severe or repeated offenses, the entire stake may be slashed to remove malicious actors from the network permanently.

#### Defense Window
To ensure fairness, validators subject to slashing will have a defense window during which they can appeal the slashing decision. (appeal process → review mechanism → temporary hold)
By implementing a defense window, the system ensures that validators have a fair opportunity to defend themselves against slashing, thereby maintaining the integrity and fairness of the slashing mechanism. This approach balances deterrence with due process, promoting a more trustworthy and reliable network.

## State Transition Function

## Storage Items
- **`CurrentValidators`**: StorageValue - Stores the current set of validators.
- **`PotentialValidators`**: StorageMap - Stores potential validators and their self-stake. → Now it is an unlimited storage map. I would make it into a CountedStorageMap, BagList or BTreeeMap or some more efficient storage item
- **`Delegators`**: StorageMap - Maps delegators to their delegation details. Value is a struct that stores the validator they are delegating to, the amount delegated, and the epoch they started delegating. The epoch is used for the reward distribution.
- **`ValidatorStakes`**: Tracks the cumulative stake for each validator.
- **`SnapshotDelegators`**: Used for reward distribution to the delegators that backed the validator in the election. 
- **`BlockCount`**: StorageMap to keep track of the block count increment for each block author during the epoch

#### Dispatchable Functions

**`register_validator`**: Allows an account to register as a potential validator/block author.
- Balance Check: Verifies that the caller has enough balance to cover the minimum stake required to become a validator.
- Hold Mechanism: Holds the minimum stake amount using the `NativeBalance` trait to prevent it from being used elsewhere.
- Storage Update: Inserts the caller into the `PotentialValidators` storage map with their self-stake amount and initializes their stake in the `ValidatorStakes` storage map.

**`unregister_validator`**: Allows a validator to unregister, releasing their self-stake and removing delegators.
- Validator Check: Confirms the caller is a registered validator.
- Delegators Handling: Iterates through the `Delegators` storage map to find delegators who have delegated to the validator and undelegates their stake.
- Release Self-Stake: Releases the self-stake held for the validator.
- Storage Cleanup: Removes the validator from the `PotentialValidators` and `ValidatorStakes` storage maps.
**Consideration**: If the validator is currently active, they should ideally wait until the end of the epoch to unregister to maintain system stability. Currently, there's nothing that restricts or manages this. <!-- kinda important but didnt have time to think about it-->

**`delegate`**: Allows an account to delegate its stake to a validator.
- Validator Existence Check: Verifies that the specified validator is in the `PotentialValidators` storage map.
- Balance Check: Ensures the caller has enough balance to cover the delegation amount.
- Delegation Check: Checks if the caller is already delegating to a different validator.
- Epoch Handling: Sets the delegation start epoch based on whether the validator is a current validator or not. If the validator is currently elected, it sets the start epoch to the next one; otherwise, it sets it to the current epoch.
- Hold Mechanism: Holds the delegation amount using the `NativeBalance` trait.
- Storage Update: Updates the `Delegators` storage map with the new delegation details and increments the `ValidatorStakes` storage map for the specified validator.

**`undelegate`**: Allows a delegator to undelegate their stake from a validator.
- Delegation Check: Confirms the caller has an existing delegation.
- Amount Check: Ensures the amount to undelegate is valid and not more than the delegated amount.
- Storage Update: Updates the `Delegators` storage map to reflect the reduced delegation amount or removes the delegator if they undelegate the entire amount. Adjusts the `ValidatorStakes` storage map accordingly.
- Release Hold: Releases the held delegation amount.

## Hooks
**`on_initialize`**: Checks if an epoch has ended and triggers validator set updates and reward distribution.
- Reward Distribution: If an epoch has ended, it calls the `distribute_epoch_rewards` function to distribute rewards to validators and delegators based on their performance and stake.
- Validator Update: Calls `update_validators` to update the set of active validators.
- Snapshot: Takes a snapshot of the current validators and delegators for reward distribution in the next epoch.
- Block Count Reset: Resets the block count for each validator.

**`on_finalize`**: Increments the block count for the current block author.
- Identifies the author of the current block and increments the block count for the identified validator.

### Internal functions
**`initialize_validators`**: Sets the initial set of validators for genesis and the first epoch.
- Initial Setup: Initializes the `CurrentValidators` and `ValidatorStakes` storage with predefined validators and their balances.
- Fallback Mechanism: These initial validators can act as fallback validators if the updated validators are not satisfactory.

**`update_validators`**: Orders the potential validators by stake and updates the set of active validators.
- Sort and Select: Sorts the potential validators by their total stake in descending order and selects the top validators up to the maximum allowed.
- Update Storage: Updates the `CurrentValidators` storage with the selected validators.

**`snapshot_validators_delegators`**: Takes snapshots of the current validators and delegators for reward distribution.
- Delegator Snapshot: Iterates over all delegators and takes a snapshot of those delegating to the current validators, storing them in `SnapshotDelegators`.

**`reset_block_counts`**: Resets the block count for each validator at the beginning of each epoch.
- Reset Logic: Iterates over the validators and resets their block counts to zero.

**`distribute_epoch_rewards`**: Distributes rewards to validators and delegators at the end of each epoch based on the snapshot.
- Block Count and Reward Calculation: Calculates the total reward based on the number of blocks authored by each validator.
- Validator Reward: Allocates a fixed percentage of the total reward to the validator.
- Delegator Reward Pool: Distributes the remaining reward among the validator's delegators proportionally based on their staked amount.

**`slash_validator`**: Slashes the entire stake of a misbehaving validator. (internal function that gets called when certain events happen - did not have time to finish implementing this - it's only half implemented)
- Origin: Ensures the call is from the root or an authorized entity.
- Validator Existence Check: Confirms the validator exists in the `PotentialValidators` storage map.
- Stake Check: Verifies that the validator has a non-zero stake.
- Slashing: Sets the validator's stake to zero in the `ValidatorStakes` storage map and burns the held balance.
- Storage Cleanup: Removes the validator from the `PotentialValidators` storage map.


## Genesis Configuration
- **Genesis Struct**: The `GenesisConfig` struct allows specifying initial parameters during the genesis block creation. These parameters include the initial set of validators and their corresponding balances.
- **Genesis Build**: The `BuildGenesisConfig` trait is implemented for the `GenesisConfig` struct. This implementation defines how the genesis configuration is applied during blockchain initialization.
- **Initialization Function**: The `initialize_validators` function sets the initial state of the validators and their stakes based on the genesis configuration.


# Improvements
- Implementing a more efficient sorting algorithm and storage mechanism for potential validators to optimize performance. 
- Make reward distribution dynamic, adjusting based on network conditions, validator performance, and block size.
	- Implementing a reputation system for validators will score them based on performance, honesty, and reliability, offering higher rewards to high-performing, reputable validators. 	
- Transitioning to a claim-based reward distribution system, which would reduce blockchain load and provide more control to delegators. 
- Enforcing that active validators must wait until the end of an epoch to unregister will maintain system stability. 
- Implementing advanced validator identity verification and mechanisms for handling dynamic adjustments in the validator set will enhance the security and functionality of the network.
- Last but not least, developing **reward distribution mechanisms that incentivize decentralization**, with higher reward multipliers for smaller validators and diminishing returns for stakes that are too large, this would promote a more evenly distributed stake across the network and discourage centralization around a few large validators.







 

