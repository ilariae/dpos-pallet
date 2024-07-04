#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html
// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html
// https://paritytech.github.io/polkadot-sdk/master/frame_support/attr.pallet.html#dev-mode-palletdev_mode
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*, 
		sp_runtime::traits::{Saturating, Zero}, 
		traits::{
			fungible::{self, Inspect, Mutate, MutateHold}, 
			FindAuthor, 
			BuildGenesisConfig,
			tokens::{Fortitude, Precision},
			OriginTrait,
		} 
	};
	use frame_system::pallet_prelude::*;
	use sp_std::prelude::*;

	/// trait to report new validator set to the runtime
	pub trait ReportNewValidatorSet<AccountId> { 
		fn report_new_validator_set(_new_set: Vec<AccountId>) {}
	}

	/// hold function required for Reason
	#[pallet::composite_enum]
	pub enum HoldReason { 
		ValidatorRegistration, 
		Delegation,
		Slashing,
	}

	pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<<T as frame_system::Config>::AccountId,>>::Balance;

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	
	/// delegation struct
	#[derive(TypeInfo, Encode, Decode, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Delegation<T:Config> {
		pub validator: T::AccountId,
		pub amount: BalanceOf<T>,
		pub epoch_started: BlockNumberFor<T>,
	}
	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event. https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/reference_docs/frame_runtime_types/index.html
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type to access the Balances Pallet. API
		type NativeBalance: fungible::Inspect<Self::AccountId>
			+ fungible::Mutate<Self::AccountId>
			+ fungible::MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason>
			+ fungible::InspectHold<Self::AccountId, Reason = Self::RuntimeHoldReason>
			+ fungible::hold::Inspect<Self::AccountId>
			+ fungible::hold::Mutate<Self::AccountId>
			+ fungible::freeze::Inspect<Self::AccountId>
			+ fungible::freeze::Mutate<Self::AccountId>;

		/// The maximum number of authorities that the pallet can hold.
		type MaxValidators: Get<u32>;

		/// Find the author of a block. A fake provide for this type is provided in the runtime. You can use a similar mechanism in your tests.
		type FindAuthor: FindAuthor<Self::AccountId>;

		/// Report the new validators to the runtime. This is done through a custom trait defined in this pallet.
		type ReportNewValidatorSet: ReportNewValidatorSet<Self::AccountId>;

		/// configurable constant `BlockNumber` to tell us when we should trigger the validator set change. 
		/// The runtime developer should implement this to represent the time they want validators to change, but for the pallet, we just care about the block number.
		#[pallet::constant]
		type EpochDuration: Get<BlockNumberFor<Self>>;

		type RuntimeHoldReason: From<HoldReason>; // defines the type for the hold reason that the runtime should use
		
	}

	/// The pallet's storage items.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#storage
	/// https://paritytech.github.io/polkadot-sdk/master/frame_support/pallet_macros/attr.storage.html
	// storagevalue sintax https://paritytech.github.io/polkadot-sdk/master/frame_support/storage/types/struct.StorageValue.html
	// storagemap syntax https://paritytech.github.io/polkadot-sdk/master/frame_support/storage/types/struct.StorageMap.html 
	
	// Current validators storage - bounded vec limited by max validators
	// ValueQuery will return a default value if the key does not exist
	#[pallet::storage]
	pub type CurrentValidators<T: Config> = StorageValue<
		Value = BoundedVec<T::AccountId, T::MaxValidators>, 
		QueryKind = ValueQuery
	>;

	/// OptionQuery will return None if the key does not exist
	// would like to change this to a BoundedStorageMap if i have time
	#[pallet::storage]
	pub type PotentialValidators<T: Config> = StorageMap<
		Hasher = Blake2_128Concat,
		Key = T::AccountId, 
		Value = BalanceOf<T>, 
		QueryKind = OptionQuery
	>; 

	/// represents delegators, whom they delegated their stake to and the delegated amount
	#[pallet::storage]
	pub type Delegators<T: Config> = StorageMap<
		Hasher = Blake2_128Concat,
		Key = T::AccountId, 
		Value = Delegation<T>, 
		QueryKind = OptionQuery
	>;

	/// keep track of the cumulative stake for each validator
	#[pallet::storage]
	pub type ValidatorStakes<T: Config> = StorageMap<
		Hasher = Blake2_128Concat,
		Key = T::AccountId,
		Value = BalanceOf<T>,
		QueryKind = ValueQuery,
	>;

	/// storage items for epoch tracking and rewards distribution

	/// snapshot for delegators of winning validators at the beginning of the epoch 
	#[pallet::storage]
	pub type SnapshotDelegators<T: Config> = StorageMap<
		Hasher = Blake2_128Concat,
		Key = T::AccountId, 
		Value = Delegation<T>,  
		QueryKind = OptionQuery
	>; 

	/// keep track of the number of blocks authored by each validator
	#[pallet::storage]
    pub type BlockCount<T: Config> = StorageMap<
        Hasher = Blake2_128Concat,
        Key = T::AccountId,
        Value = u32,
        QueryKind = ValueQuery,
    >;

	/// Pallets use events to inform users when important changes are made. https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> { /// Use passive tense for events.
		ValidatorRegistered {validator: T::AccountId, amount: BalanceOf<T>}, 
		Delegated {delegator: T::AccountId, validator: T::AccountId, amount: BalanceOf<T>}, // delegator delegated to a validator
		ValidatorsUpdated,
		RewardsDistributed,
		Undelegated { delegator: T::AccountId, validator: T::AccountId, amount: BalanceOf<T> },
		ValidatorDeregistered { validator: T::AccountId },
		DelegatorRemoved { delegator: T::AccountId, validator: T::AccountId, amount: BalanceOf<T> },
		ValidatorSlashed { validator: T::AccountId, amount: BalanceOf<T> },
	}

	/// Errors inform users that something went wrong. https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error
	#[pallet::error]
	pub enum Error<T> {
		TooManyValidators,
		InsufficientBalance,
		AlreadyDelegated,
		ValidatorNotFound,
		AlreadyRegistered,
		NoDelegationFound,
    	InvalidAmount,
	} 

	/// Hooks are used to execute code in pallets when certain events occur. 
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight { // runs at the beginning of every block
			log::debug!("on_initialize called at block: {:?}", n);

			if n % T::EpochDuration::get() == BlockNumberFor::<T>::zero() { // lightweight check at EVERY block, tells us when an Epoch has passed
				log::debug!("Epoch duration: {:?}", T::EpochDuration::get());
				Self::distribute_epoch_rewards();
				log::debug!("Epoch duration met, updating validators.");
				// call update_validators function at the end of each epoch // You cannot return an error here, so you have to be clever with your code...
				Self::update_validators();

				// Take a snapshot of current validators and delegators at the beginning of each epoch
				Self::snapshot_validators_delegators();
                Self::reset_block_counts();
			}	

			// We return a default weight because we do not expect you to do weights for your project... Except for extra credit...
			return Weight::default()
		}

		/// Function to increment the block count for the current block author
		fn on_finalize(n: BlockNumberFor<T>) {
            // Increment block count for the current block author
			if let Some(author) = Self::find_author() {
				BlockCount::<T>::mutate(&author, |count| {
					*count += 1;
					log::debug!("Incremented block count for validator {:?} to {:?}", author, *count);
				});
			} else {
				log::debug!("No author found for block {:?}", n);
			}
        }
	}

	/// genesis configuration to set up a set of initial validators and their balances
	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub initial_validators: Vec<T::AccountId>,
		pub initial_balances: Vec<(T::AccountId, BalanceOf<T>)>,
	}

	/// Genesis build function to initialize the pallet with the initial validators and their balances
	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_validators(self.initial_validators.clone(), self.initial_balances.clone());
		}
	}

	/// Dispatchable functions allows users to interact with the pallet and invoke state changes. These functions materialize as "extrinsics", which are often compared to transactions.
	/// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#dispatchables
	/// Origin documentation https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/reference_docs/frame_origin/index.html 
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		// ---------- register and unregister validators ----------
		/// function to allow an account to register as a potential validator by ensuring they have enough balance and reserving it
		pub fn register_validator(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
		)-> DispatchResult {
			let who = ensure_signed(origin)?; // ensure extrinsic signed by valid account and retrieves the accountID

			// ensure caller has enough balance to be a validator
			ensure!( 
				T::NativeBalance::balance(&who) >= amount,
				Error::<T>::InsufficientBalance
			);
			// ensure the caller is not already a registered validator
			ensure!(
				!PotentialValidators::<T>::contains_key(&who),
				Error::<T>::AlreadyRegistered
			);

			T::NativeBalance::hold(&HoldReason::ValidatorRegistration.into(), &who, amount)?; // hold self-stake amount
			PotentialValidators::<T>::insert(&who, amount); // add caller to list of potential validators
			ValidatorStakes::<T>::insert(&who, amount); // initialize validator's stake with self-stake

			Self::deposit_event(Event::ValidatorRegistered{validator:who, amount}); // event validator has been registered
			Ok(())
		}

		/// function to unregister a validator, function first releases the delegators and then removes the validator
		pub fn unregister_validator(
			origin: OriginFor<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
		
			// ensure caller is a registered validator
			let self_stake = PotentialValidators::<T>::get(&who).ok_or(Error::<T>::ValidatorNotFound)?;
			log::debug!("Validator {:?} found with self-stake: {:?}", who, self_stake);
		
			// create a vector to store the delegators to be undelegated
			let mut delegators_to_undelegate: Vec<T::AccountId> = Vec::new();
		
			// iterate over all delegators to find those delegating to the validator
			for (delegator, delegation) in Delegators::<T>::iter() {
				if delegation.validator == who {
					log::debug!("Found delegator {:?} with amount {:?} delegating to validator {:?}", delegator, delegation.amount, who);
					delegators_to_undelegate.push(delegator);
				}
			}
		
			// call undelegate for each delegator
			for delegator in delegators_to_undelegate {
				let delegation = Delegators::<T>::get(&delegator).ok_or(Error::<T>::NoDelegationFound)?;
				let undelegate_origin = T::RuntimeOrigin::signed(delegator.clone());
				log::debug!("Undelegating amount {:?} from delegator {:?} for validator {:?}", delegation.amount, delegator, who);
				Self::undelegate(undelegate_origin, delegation.amount)?;
			}
		
			// release self-stake for the validator
			log::debug!("Releasing self-stake of amount {:?} for validator {:?}", self_stake, who);
			T::NativeBalance::release(&HoldReason::ValidatorRegistration.into(), &who, self_stake, Precision::BestEffort)?;		
			// remove the validator from the PotentialValidators storage
			log::debug!("Removing validator {:?} from PotentialValidators storage", who);
			PotentialValidators::<T>::remove(&who);		
			// remove the validator from the ValidatorStakes storage
			log::debug!("Removing validator {:?} from ValidatorStakes storage", who);
			ValidatorStakes::<T>::remove(&who);
		
			// emit event
			Self::deposit_event(Event::ValidatorDeregistered { validator: who });
		
			Ok(())
		}			

		// ---------- delegate and undelegate ----------
		/// delegate function allows an account to delegate their stake to a validator, ensuring all necessary conditions are met
		pub fn delegate(
			origin: OriginFor<T>,
			validator: T::AccountId,
			amount: BalanceOf<T>,
		)-> DispatchResult {
			let who = ensure_signed(origin)?; 

			// Ensure the amount to delegate is greater than zero
			ensure!(amount > Zero::zero(), Error::<T>::InvalidAmount);

			log::debug!("Delegator {:?} is attempting to delegate {:?} to validator {:?}", who, amount, validator);
			
			// ensure validator exists - contains_key checks if given key exists in storagemap
			ensure!( 
				PotentialValidators::<T>::contains_key(&validator), 
				Error::<T>::ValidatorNotFound
			);

			// ensure caller has enough balance to delegate
			ensure!(
				T::NativeBalance::balance(&who) >= amount, 
				Error::<T>::InsufficientBalance
			);

			let current_epoch = <frame_system::Pallet<T>>::block_number() / T::EpochDuration::get();
			let next_epoch = current_epoch + BlockNumberFor::<T>::from(1u32);
			log::debug!("Current epoch: {:?}, Next epoch: {:?}", current_epoch, next_epoch);

			let epoch_started = if CurrentValidators::<T>::get().contains(&validator) {
				log::debug!("Validator {:?} is already a current validator. Delegation will start in the next epoch: {:?}", validator, next_epoch);
				next_epoch // If the validator is already a current validator, delegation will start in the next epoch
			} else {
				log::debug!("Validator {:?} is not a current validator. Delegation will start in the current epoch: {:?}", validator, current_epoch);
				current_epoch // If the validator is not a current validator, delegation will start in the current epoch
			};

			// check if the delegator has already delegated
			if Delegators::<T>::contains_key(&who) {
				// if delegating to a different validator, reject the call, else update the existing delegation amount
				Delegators::<T>::try_mutate_exists(&who, |delegation| {
					if let Some(delegation) = delegation {
						if delegation.validator != validator {
							return Err(Error::<T>::AlreadyDelegated);
						} else {
							delegation.amount += amount;
						}
					}
					Ok(())
				})?;
			} else { // if its a new delegatipn
				// create a new delegation entry
				let delegation = Delegation {
					validator: validator.clone(),
					amount,
					epoch_started, // Set the epoch when the delegation started
				};
				log::debug!("Inserting new delegation for delegator {:?} to validator {:?} starting at epoch {:?}", who, validator, epoch_started);
				Delegators::<T>::insert(&who, delegation);
			}

			T::NativeBalance::hold(&HoldReason::Delegation.into(), &who, amount)?; // reserve delegation amount

			// update validator's total stake
			ValidatorStakes::<T>::mutate(&validator, |stake| {
				log::debug!("Updating stake for validator {:?}: old stake = {:?}:, adding = {:?}:", validator, *stake, amount);
				*stake += amount;
			});
			// !!!!!----- Could overflow, better to use saturating or checked math here -----!!!!!

			Self::deposit_event(Event::Delegated{delegator: who, validator, amount}); // emit event
			Ok(())
		}

		/// function to undelegate stake from a validator
		pub fn undelegate(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
	
			// check if delegator has a delegation
			let delegation = Delegators::<T>::get(&who).ok_or(Error::<T>::NoDelegationFound)?;
			log::debug!("Delegation found"); 
			log::debug!("Amount delegated: {:?}", delegation.amount);
	
			// ensure amount to undelegate is valid
			ensure!(amount <= delegation.amount, Error::<T>::InsufficientBalance);
			log::debug!("Amount to undelegate is valid, trying to undelegate {:?}", amount);

			// update the delegator's delegation amount
			// !!!!!----- nit: you got the delegation already on ln 391, so can mutate and set that instead of making a new call 
			// to mutate_exists (which doesn't assume the existence of the key)
			Delegators::<T>::mutate_exists(&who, |maybe_delegation| {
				if let Some(delegation) = maybe_delegation {
					log::debug!("Undelegating: {:?} ...", amount);
					delegation.amount -= amount;
					// !!!!!----- Although you have the ensure a few lines above, still better practice to use safe math 
					// to protect against any regressions introduced in the future which may allow an underflow -----!!!!!
					log::debug!("Delegation amount after undelegation: {:?}", delegation.amount); // prints zero when trying to undelegate all 
					
					if delegation.amount == Zero::zero() {
						log::debug!("Removing delegator {:?} from storage", who);
						*maybe_delegation = None;
					}
				}
			});

			// release the held balance for the delegator
			log::debug!("Releasing stake");
			T::NativeBalance::release(&HoldReason::Delegation.into(), &who, amount, Precision::BestEffort)?;

			// update the validator's total stake
			ValidatorStakes::<T>::mutate(&delegation.validator, |stake| {
				log::debug!("Validator stake before undelegation: {:?}", *stake); 
				*stake = stake.saturating_sub(amount);
				log::debug!("Updated validator stake: {:?}", *stake); 
			});
	
			Self::deposit_event(Event::Undelegated { delegator: who.clone(), validator: delegation.validator.clone(), amount });
			Ok(())
		}


		/// call slash_validator function to slash a validator's stake
		#[cfg(test)]
		pub fn test_slash_validator(
			origin: OriginFor<T>, 
			validator: T::AccountId
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::slash_validator(validator)
		}
	
	}

	impl<T: Config> Pallet<T> {

		/// function to initialize genesis set of validators with initial balances
		pub fn initialize_validators(initial_validators: Vec<T::AccountId>, initial_balances: Vec<(T::AccountId, BalanceOf<T>)>) {
            let validators: BoundedVec<T::AccountId, T::MaxValidators> = initial_validators.clone().try_into().expect("Failed to convert validators to BoundedVec");
			let balance = <BalanceOf<T>>::from(100u32);

			// !!!!!----- Better practice for NativeBalance genesis to be set using its own genesis config -----!!!!!
            for (validator, balance) in initial_balances.iter() {
                T::NativeBalance::set_balance(validator, *balance);
            } 
            for validator in validators.iter() {
                ValidatorStakes::<T>::insert(validator, balance);
                PotentialValidators::<T>::insert(validator, balance);
            }

            CurrentValidators::<T>::put(validators);
        }
		
		// if i have time want to change the sorting mechanism to something more efficient - for now just sort by stake amount 
		/// function to update the set of validators at the end of each epoch
		pub fn update_validators() {
			log::debug!("update_validators function called");

			let mut potential_validators: Vec<(T::AccountId, BalanceOf<T>)> = ValidatorStakes::<T>::iter().collect();
			log::debug!("Potential validators before sorting: {:?}", potential_validators);

			potential_validators.sort_by(|a, b| b.1.cmp(&a.1)); // sort by stake amount in descending order
			log::debug!("Potential validators after sorting: {:?}", potential_validators);

			// get top validators
			let max_validators = T::MaxValidators::get() as usize;
			let new_validators: Result<BoundedVec<T::AccountId, T::MaxValidators>, _> = potential_validators
				.into_iter()
				.take(max_validators)
				.map(|(account_id, _)| account_id) // discards balance
				.collect::<Vec<_>>()
				.try_into(); 

			match new_validators {
				Ok(validators) => {
					log::debug!("New validators: {:?}", validators);
					CurrentValidators::<T>::put(validators.clone()); // save updated validators into current
					T::ReportNewValidatorSet::report_new_validator_set(validators.into());
					Self::deposit_event(Event::ValidatorsUpdated);
				},
				Err(_) => {
					log::error!("Failed to convert validators to BoundedVec");
				}
			}

		}

		/// function to take a snapshot of the current validators and delegators at the beginning of each epoch
		fn snapshot_validators_delegators() {
			log::debug!("snapshot_validators_delegators function called");

			// take a snapshot of the current validators
			let current_validators = CurrentValidators::<T>::get();

			// take a snapshot of the current delegators
			for validator in current_validators.iter() {
                for (delegator, delegation) in Delegators::<T>::iter() {
                    if delegation.validator == *validator {
						// log::debug!("Found delegator {:?} for validator {:?} with delegation amount {:?}", delegator, validator, delegation.amount);
                        SnapshotDelegators::<T>::insert(&delegator, delegation);
                    }
                }
            }
			log::debug!("Snapshot taken for validators and delegators");
		}

		/// function to keep track of the number of blocks authored by each validator
		fn reset_block_counts() {
            log::debug!("reset_block_counts function called");
            for (validator, _) in ValidatorStakes::<T>::iter() {
                BlockCount::<T>::insert(&validator, 0);
            }
        }

		/// function to distribute rewards to validators and delegators at the end of each epoch
		fn distribute_epoch_rewards() {
			log::debug!("Distribute epoch rewards function called");
	
			let epoch_validators = CurrentValidators::<T>::get(); // here snapshot validators
			let current_epoch = <frame_system::Pallet<T>>::block_number() / T::EpochDuration::get();
			log::debug!("Current epoch: {:?}", current_epoch);

			for validator in epoch_validators.iter() {
				let block_count = BlockCount::<T>::get(&validator);
	
				// Total reward amount for the epoch, now multiplied by the block count
				let base_reward_per_block = BalanceOf::<T>::from(1000u32);
            	let total_reward = base_reward_per_block.saturating_mul(BalanceOf::<T>::from(block_count));
				log::debug!("Validator {:?} authored {:?} blocks and has a total reward pool of {:?}", validator, block_count, total_reward);
	
				// Allocate a fixed percentage to the validator (e.g., 30%)
				let validator_percentage: BalanceOf<T> = BalanceOf::<T>::from(30u32);
				let hundred: BalanceOf<T> = BalanceOf::<T>::from(100u32);
	
				let validator_reward = total_reward.saturating_mul(validator_percentage) / hundred;
				let delegators_reward_pool = total_reward.saturating_sub(validator_reward);
	
				// Retrieve validator stake 
				let validator_stake = ValidatorStakes::<T>::get(&validator);
				log::debug!("Validator {:?} has a total stake of: {:?}", validator, validator_stake);
	
				let mut remaining_delegators_reward = delegators_reward_pool;
	
				// Iterate over all epoch delegators and distribute rewards 
				for (delegator, delegation) in SnapshotDelegators::<T>::iter() {
					log::debug!("Checking delegator {:?} who delegated to validator {:?} starting at epoch {:?}", delegator, delegation.validator, delegation.epoch_started);
					if delegation.validator == *validator && delegation.epoch_started < current_epoch{
						let delegator_reward = delegators_reward_pool.saturating_mul(delegation.amount) / validator_stake;
						remaining_delegators_reward = remaining_delegators_reward.saturating_sub(delegator_reward);
	
						// Log the delegator reward details
						log::debug!("Delegator {:?} has delegated {:?} to validator {:?} and receives a reward of {:?}", delegator, delegation.amount, validator, delegator_reward);
						Self::deposit_event(Event::RewardsDistributed);
	
						// Attempt to mint the reward to the delegator's account
						if let Err(e) = T::NativeBalance::mint_into(&delegator, delegator_reward) {
							log::error!("Failed to mint reward for delegator: {:?}", e);
						}
					}
				}
	
				log::debug!("Validator {:?} receives the reward of {:?}", validator, validator_reward);
	
				// Credit the remaining amount to the validator
				if let Err(e) = T::NativeBalance::mint_into(&validator, validator_reward) {
					log::error!("Failed to mint reward for validator: {:?}", e);
				}
	
				log::debug!("Rewards distributed for validator {:?}: {:?}", validator, validator_reward);
				Self::deposit_event(Event::RewardsDistributed);
			}
		}

		/// Internal function to slash a validator's stake.
		#[allow(dead_code)]
		fn slash_validator(validator: T::AccountId) -> DispatchResult {
			// Ensure the validator is a potential validator
			ensure!(
				PotentialValidators::<T>::contains_key(&validator), 
				Error::<T>::ValidatorNotFound
			);
	
			// Get the validator's total stake
			let validator_stake = ValidatorStakes::<T>::get(&validator);
			ensure!(
				!validator_stake.is_zero(), 
				Error::<T>::InsufficientBalance
			);
	
			// Slash the validator's entire stake
			ValidatorStakes::<T>::mutate(&validator, |stake| {
				*stake = Zero::zero();
			});
	
			// Burn the entire held balance
			T::NativeBalance::burn_held(&HoldReason::Slashing.into(), &validator, validator_stake, Precision::BestEffort, Fortitude::Force)?;
	
			// Remove the validator from the PotentialValidators storage
			PotentialValidators::<T>::remove(&validator);
	
			Self::deposit_event(Event::ValidatorSlashed { validator, amount: validator_stake });
			Ok(())
		}

	}

	impl<T: Config> Pallet<T> {
		// A function to get you an account id for the current block author.
		pub fn find_author() -> Option<T::AccountId> {
			// If you want to see a realistic example of the `FindAuthor` interface, see `pallet-authorship`.
			T::FindAuthor::find_author::<'_, Vec<_>>(Default::default())
		}
	}

}


