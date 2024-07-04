use crate::{self as pallet_dpos, ReportNewValidatorSet};
use frame_support::{
	derive_impl, parameter_types,
	traits::{ConstU128, ConstU16, ConstU32, ConstU64, FindAuthor},
};
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
type AccountId = u64;

// Define the test runtime with necessary pallets.
frame_support::construct_runtime! {
	pub struct Test {
		System: frame_system,
		Balances: pallet_balances,
		Dpos: pallet_dpos,
	}
}

// Implementing frame_system::Config for Test.
// https://paritytech.github.io/polkadot-sdk/master/frame_support/attr.derive_impl.html
#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

// Implementing pallet_balances::Config for Test.
#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ConstU32<10>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<10>;
}

parameter_types! {
	pub const MaxValidators: u32 = 10;
	pub static Author: AccountId = 7;
}

// Custom implementation to find author for DynamicAuthor.
pub struct DynamicAuthor;
impl FindAuthor<AccountId> for DynamicAuthor {
	fn find_author<'a, I>(_: I) -> Option<AccountId>
	where
		I: 'a + IntoIterator<Item = ([u8; 4], &'a [u8])>,
	{
		Some(Author::get())
	}
}

// Dummy implementation for ReportNewValidatorSet
pub struct DoNothing;
impl ReportNewValidatorSet<AccountId> for DoNothing {
	fn report_new_validator_set(_: Vec<AccountId>) {}
}

// Implementing pallet_dpos::Config for Test.
impl pallet_dpos::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type NativeBalance = Balances;
	type MaxValidators = MaxValidators;
	type FindAuthor = DynamicAuthor;
	type ReportNewValidatorSet = DoNothing;
	// Assuming blocks happen every 6 seconds, this will be 600 seconds, approximately 10 minutes. CONFIGURABLE
	type EpochDuration = ConstU64<100>;
	type RuntimeHoldReason = RuntimeHoldReason;
}

// Struct to define initial validators and their balances.
pub struct InitialValidators{
	pub initial_validators: Vec<AccountId>,
	pub initial_balances: Vec<(AccountId, Balance)>,
}

// Default implementation for InitialValidators.
impl Default for InitialValidators {
	fn default() -> Self {
		let initial_validators = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let initial_balances: Vec<(AccountId, Balance)> = initial_validators.iter().map(|v| (*v, 10_000)).collect();
        Self {
            initial_validators,
            initial_balances,
        }
	}
}

// Additional methods for InitialValidators.
impl InitialValidators {
	pub fn build(mut self, initial_validators: Vec<AccountId>) -> Self {
		self.initial_validators = initial_validators;
        self.initial_balances = self.initial_validators.iter().map(|v| (*v, 10_000)).collect();
        self
	}
}

// Function to create externalities for testing.
pub fn new_test_ext() -> sp_io::TestExternalities {
    // Learn more about improving test setup in the provided link.
	// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html
	// frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()

	let initial_validators = InitialValidators::default().build(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

    let genesis_config = pallet_dpos::GenesisConfig::<Test> {
        initial_validators: initial_validators.initial_validators.clone(),
        initial_balances: initial_validators.initial_balances.clone(),
    };

    let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into();
    genesis_config.assimilate_storage(&mut storage).unwrap();

    storage.into()
}
