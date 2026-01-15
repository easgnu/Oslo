#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

extern crate alloc;
mod weights;
mod precompiles;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub const WEIGHT_MILLISECS_PER_BLOCK: u64 = 10000;

use ethereum::AuthorizationList;
use pallet_grandpa::{fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use frame_system::{EnsureRoot, ChainContext};
use codec::{Encode, Decode};
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, U256, H160, H256, ConstBool};
use crate::currency::*;
use sp_runtime::{generic, impl_opaque_keys, generic::Era, ApplyExtrinsicResult, AccountId32, ExtrinsicInclusionMode,
	traits::{BlakeTwo256, Block as BlockT, NumberFor, Dispatchable, PostDispatchInfoOf, DispatchInfoOf, IdentifyAccount,
	UniqueSaturatedInto, OpaqueKeys, Verify, AccountIdLookup}, ConsensusEngineId, SaturatedConversion,
	transaction_validity::{TransactionSource, TransactionPriority, TransactionValidity, TransactionValidityError}
};

use sp_std::prelude::*;
mod genesis_config_preset;

#[cfg(feature = "std")]
use sp_version::NativeVersion;

use sp_version::{RuntimeVersion, Cow};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_evm::{Account as EVMAccount, EnsureAddressTruncated, Runner, FeeCalculator, HashedAddressMapping, GasWeightMapping};
use pallet_ethereum::{Call::transact, PostLogContent, EthereumBlockHashMapping, Transaction as EthereumTransaction, TransactionAction, TransactionData};
use fp_rpc::TransactionStatus;

// pub use this so we can import it in the chain spec.
#[cfg(feature = "std")]
pub use fp_evm::GenesisAccount;

use frame_support::{traits::OnTimestampSet, genesis_builder_helper::{build_state}};
use sp_genesis_builder::PresetId;
// A few exports that help ease life for downstream crates.
pub use frame_support::{parameter_types, pallet_prelude::PhantomData, PalletId, derive_impl, StorageValue,
	traits::{ConstU128, ConstU32, ConstU64, ConstU8, KeyOwnerProofSystem, Randomness, StorageInfo,
		FindAuthor, OnUnbalanced, Currency, Imbalance, EitherOfDiverse, EqualPrivilegeOnly, NeverEnsureOrigin,
		OnFinalize, AsEnsureOriginWithArg, tokens::pay::PayAssetFromAccount}, runtime,
	weights::{constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
		IdentityFee, Weight}
};
pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;

use pallet_transaction_payment::FungibleAdapter;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

pub use sp_runtime::{Perbill, Permill, MultiSignature, MultiAddress, MultiSigner};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = H256;

/// The hashing algorithm used by the chain.
pub type Hashing = BlakeTwo256;

/// Digest item type.
pub type DigestItem = generic::DigestItem;

pub mod currency {
	use super::Balance;
	pub const WEI: Balance = 1;
	pub const NANOOSLO: Balance = 1_000;
	pub const MICROOSLO: Balance = 1_000_000;
	pub const MILLIOSLO: Balance = 1_000_000_000;
	pub const OSLO: Balance = 1_000_000_000_000;
	pub const KILOOSLO: Balance = 1_000_000_000_000_000;

	pub const TRANSACTION_BYTE_FEE: Balance = 1 * NANOOSLO;
	pub const STORAGE_BYTE_FEE: Balance = 1 * NANOOSLO;
	pub const WEIGHT_FEE: Balance = 1 * NANOOSLO;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * NANOOSLO + (bytes as Balance) * STORAGE_BYTE_FEE
	}
}

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, AccountIndex>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = fp_self_contained::CheckedExtrinsic<AccountId, RuntimeCall, TxExtension, H256>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The `TransactionExtension` to the basic transaction logic.
pub type TxExtension = (
	frame_system::AuthorizeCall<Runtime>,
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	frame_system::WeightReclaim<Runtime>
);

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, TxExtension>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block, 
	ChainContext<Runtime>,
	Runtime, 
	AllPalletsWithSystem
>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;
	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
			pub im_online: ImOnline,
		}
	}
}

//substrate-validator-set will not allow manual removal of authorities below this point.
//The network can start with fewer nodes though and offline nodes will still be removed.
parameter_types! { pub const MinAuthorities: u32 = 5; }

impl validator_set::Config for Runtime {
	type RuntimeEvent = RuntimeEvent; 
	//A unanimous technical committee can add or remove validators
	type AddRemoveOrigin = EitherOfDiverse<EnsureRoot<AccountId>, pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>>;
	type MinAuthorities = MinAuthorities;
	type WeightInfo = validator_set::weights::SubstrateWeight<Runtime>;
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("oslo-network"),
	impl_name: Cow::Borrowed("oslo-network"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is initially set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 103,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1
};

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.

/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 30000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
// Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion { 
	NativeVersion { 
		runtime_version: VERSION, 
		can_author_with: Default::default() 
	}
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

use precompiles::SubstratePrecompiles;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	/// We allow for 10 seconds of compute with a 30 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::with_sensible_defaults(Weight::from_parts(10u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX), NORMAL_DISPATCH_RATIO);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength::max_with_normal_ratio(25 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// Block & extrinsics weights: base values and limits.
	type RuntimeEvent = RuntimeEvent;
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The aggregated RuntimeTask type.
	type RuntimeTask = RuntimeTask;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The block type.
	type Block = Block;
	/// The hashing algorithm used.
	type Hashing = Hashing;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, AccountIndex>;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	type MaxConsumers = ConstU32<16>;
	/// The set code logic, just the default since we're not a parachain.
  type OnSetCode = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
	
	type ExtensionsWeightInfo = ();
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type MaxAuthorities = ConstU32<32>;
	type DisabledValidators = ();
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Runtime>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type KeyOwnerProof = sp_core::Void;

	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = ConstU64<0>;
	type EquivocationReportSystem = ();
	type WeightInfo = ();
	type MaxAuthorities = ConstU32<32>;
}

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: u128 = 1 * currency::NANOOSLO;

impl pallet_balances::Config for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Self>;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<1>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

pub struct DealWithFees;
type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;
 
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds(mut fees_then_tips: impl Iterator<Item=NegativeImbalance>) {
		if let Some(fees) = fees_then_tips.next() {
			// for fees and tips, 20% to treasury, 80% to author
			let mut split = fees.ration(20, 80);
			if let Some(tips) = fees_then_tips.next() {
				tips.ration_merge_into(20, 80, &mut split);
			}
			Treasury::on_unbalanced(split.0);
			Author::on_unbalanced(split.1);
		}
	}
}

pub struct Author;
 
impl OnUnbalanced<NegativeImbalance> for Author {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		if let Some(author) = Authorship::author() { Balances::resolve_creating(&author, amount); }
	}
}
 
impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	//replace with dealwithfees?
	type OnChargeTransaction = FungibleAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
	type WeightInfo = pallet_transaction_payment::weights::SubstrateWeight<Runtime>;
}
 
impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
	// Retry a scheduled item every 10 blocks (5 minutes) until the preimage exists.
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

const BLOCK_GAS_LIMIT: u64 = 375_000_000;
const MAX_POV_SIZE: u64 = 25 * 1024 * 1024;

parameter_types! {
	//This had to be changed from 1998 since Kyoto testnet used it
	pub const LeetChainId: u64 = 19980;
	pub BlockGasLimit: U256 = U256::from(BLOCK_GAS_LIMIT);
	pub PrecompilesValue: SubstratePrecompiles<Runtime> = SubstratePrecompiles::<_>::new();
}

pub struct FindAuthorTruncated<F>(PhantomData<F>);

impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item=(ConsensusEngineId, &'a [u8])>
	{
		use sp_core::crypto::ByteArray;
		F::find_author(digests).and_then(|i| {
			pallet_aura::Authorities::<Runtime>::get().into_inner().get(i as usize).and_then(|id| { 
				let raw = id.to_raw_vec();
				if raw.len() >= 24 { Some(H160::from_slice(&raw[4..24])) } else {None}
			})
		})
	}
}

use fp_account::AccountId20;
pub struct StorageFindAuthor<Inner>(PhantomData<Inner>);

impl<Inner> FindAuthor<H160> for StorageFindAuthor<Inner>
where 
	Inner: FindAuthor<AccountId20>
{
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>
	{ Inner::find_author(digests).map(Into::into) }
}

const WEIGHT_PER_GAS: u64 = 20_000;
/// The maximum storage growth per block in bytes.
const MAX_STORAGE_GROWTH: u64 = 800 * 1024;

parameter_types! {
	pub const GasLimitPovSizeRatio: u64 = BLOCK_GAS_LIMIT.saturating_div(MAX_POV_SIZE);
	pub WeightPerGas: Weight = Weight::from_parts(WEIGHT_PER_GAS, 0);
	pub const GasLimitStorageGrowthRatio: u64 = BLOCK_GAS_LIMIT.saturating_div(MAX_STORAGE_GROWTH);
}

impl pallet_evm::Config for Runtime {
	type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
	type FeeCalculator = BaseFee;
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
	type WeightPerGas = WeightPerGas;
	type BlockHashMapping = EthereumBlockHashMapping<Self>;
	type CallOrigin = EnsureAddressTruncated;
	type WithdrawOrigin = EnsureAddressTruncated;
	type AddressMapping = HashedAddressMapping<BlakeTwo256>;
	type Currency = Balances;
	type Timestamp = Timestamp;
	type PrecompilesType = SubstratePrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = LeetChainId;
	type BlockGasLimit = BlockGasLimit;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type OnChargeTransaction = ();
	type OnCreate = ();
	type FindAuthor = FindAuthorTruncated<Aura>;
	type GasLimitStorageGrowthRatio = GasLimitStorageGrowthRatio;
	type CreateOriginFilter = ();
	type CreateInnerOriginFilter = ();
	type WeightInfo = pallet_evm::weights::SubstrateWeight<Self>;
}

parameter_types! { 
	pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Self::Version>;
	type PostLogContent = PostBlockAndTxnHashes;
	type ExtraDataLength = ConstU32<30>;
}

parameter_types! {
	pub DefaultElasticity: Permill = Permill::zero();
	pub DefaultBaseFeePerGas: U256 = U256::from(10_000);
}

pub struct BaseFeeThreshold;

impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
	fn lower() -> Permill { Permill::zero() }
	fn ideal() -> Permill { Permill::from_parts(500_000) }
	fn upper() -> Permill { Permill::from_parts(1_000_000) }
}

impl pallet_base_fee::Config for Runtime {
	type Threshold = BaseFeeThreshold;
	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
	type DefaultElasticity = DefaultElasticity;
}

// Implementing the frame system off-chain
// ////////////// // //////////////// //////////////// //////////////// //////////////// //////////////// //////////////// //////////////// //////////////// //////////////// //////////////
impl<LocalCall> frame_system::offchain::CreateBare<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>
{
	fn create_bare(call: RuntimeCall) -> UncheckedExtrinsic {
		generic::UncheckedExtrinsic::new_bare(call).into()
	}
}

impl<C> frame_system::offchain::CreateTransactionBase<C> for Runtime
where
	RuntimeCall: From<C>
{
	type Extrinsic = UncheckedExtrinsic;
	type RuntimeCall = RuntimeCall;
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>
{
	fn create_signed_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall, public: MultiSigner, account: AccountId, nonce: Nonce
	) -> Option<UncheckedExtrinsic> {
		let tip = 0;
		let period = BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
		let current_block = System::block_number().saturated_into::<u64>().saturating_sub(1);
		let era = Era::mortal(period, current_block);
		let extra = (
			frame_system::AuthorizeCall::<Runtime>::new(),
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(era),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			frame_system::WeightReclaim::<Runtime>::new()
		);
		let raw_payload = SignedPayload::new(call, extra).map_err(|e| {log::warn!("Unable to create signed payload: {:?}", e); }).ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = MultiAddress::Id(account);
		let (call, extra, _) = raw_payload.deconstruct();
		let transaction = generic::UncheckedExtrinsic::new_signed(call, address, signature, extra).into();
		Some(transaction)
	}
}
 
impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type ValidatorSet = ValidatorSet;
	type ReportUnresponsiveness = ();
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
	type RuntimeEvent = RuntimeEvent;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = ();
}

parameter_types! {
	pub const Period: u32 = 60 * MINUTES;
	pub const Offset: u32 = 0;
}


//Not sure this is needed since erc20 tokens are still supported
parameter_types! {
	pub const AssetDeposit: Balance = 1 * OSLO;
	pub const ApprovalDeposit: Balance = 1 * OSLO;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 1 * OSLO;
	pub const MetadataDepositPerByte: Balance = 1 * MICROOSLO;
}


impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<OSLO>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
	type RemoveItemsLimit = ConstU32<1000>;
	type Holder = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	pub const SpendPeriod: BlockNumber = 1 * DAYS;
	pub const Burn: Permill = Permill::from_percent(0);
	pub const MaxApprovals: u32 = 100;
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxPeerDataEncodingSize: u32 = 1_000;
	pub const SpendPayoutPeriod: BlockNumber = 30 * DAYS;
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

impl pallet_treasury::Config for Runtime {
	type SpendOrigin = NeverEnsureOrigin<u128>;
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = EitherOfDiverse<EnsureRoot<AccountId>, pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	type MaxApprovals = MaxApprovals;
	type AssetKind = u32;
	type Beneficiary = AccountId;
	type BeneficiaryLookup = AccountIdLookup<AccountId, AccountIndex>;
	type Paymaster = PayAssetFromAccount<Assets, TreasuryAccount>;
	type BalanceConverter = AssetRate;
	type PayoutPeriod = SpendPayoutPeriod;
	type BlockNumberProvider = System;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
}

impl pallet_asset_rate::Config for Runtime {
	type CreateOrigin = EnsureRoot<AccountId>;
	type RemoveOrigin = EnsureRoot<AccountId>;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type AssetKind = u32;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_asset_rate::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 7 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}


parameter_types! {
	pub const LaunchPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
	pub const VotingPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * 24 * 60 * MINUTES;
	pub const MinimumDeposit: Balance = 100 * OSLO;
	pub const EnactmentPeriod: BlockNumber = 30 * 24 * 60 * MINUTES;
	pub const CooloffPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
	pub const MaxProposals: u32 = 100;
	pub MaxProposalWeight: Weight = sp_runtime::Perbill::from_percent(50) * BlockWeights::get().max_block;
}

type CouncilCollective = pallet_collective::Instance1;

impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxProposalWeight = MaxProposalWeight;
	type DisapproveOrigin = EitherOfDiverse<EnsureRoot<AccountId>, pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>>;
	type KillOrigin = EitherOfDiverse<EnsureRoot<AccountId>, pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>>;
	type Consideration = ();
}

type TechnicalCollective = pallet_collective::Instance2;

impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type MaxMembers = TechnicalMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxProposalWeight = MaxProposalWeight;
	type DisapproveOrigin = EitherOfDiverse<EnsureRoot<AccountId>, pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 3, 4>>;
	type KillOrigin = EitherOfDiverse<EnsureRoot<AccountId>, pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 3, 4>>;
	type Consideration = ();
}

use frame_support::traits::LinearStoragePrice;
use frame_support::traits::fungible::HoldConsideration;
use frame_system::EnsureSigned;

impl pallet_democracy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type VoteLockingPeriod = EnactmentPeriod;
	type Scheduler = Scheduler;
	// Same as EnactmentPeriod
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
	type InstantOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
	type InstantAllowed = frame_support::traits::ConstBool<true>;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EitherOfDiverse<EnsureRoot<AccountId>, pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>>;
	type SubmitOrigin = EnsureSigned<AccountId>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type CooloffPeriod = CooloffPeriod;
	type Slash = Treasury;	
	type MaxDeposits = ConstU32<100>;
	type Preimages = Preimage;
	type MaxBlacklisted = ConstU32<100>;

	type PalletsOrigin = OriginCaller;
	type MaxVotes = frame_support::traits::ConstU32<100>;
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
	type MaxProposals = MaxProposals;
}

parameter_types! {
	pub storage EnableManualSeal: bool = false;
}
pub struct ConsensusOnTimestampSet<T>(PhantomData<T>);

#[cfg(feature = "ts-tests")]
impl<T: pallet_aura::Config> OnTimestampSet<T::Moment> for ConsensusOnTimestampSet<T> {
	fn on_timestamp_set(moment: T::Moment) {
		if EnableManualSeal::get() {
			log::info!("Manually sealing block...");
			return;
		}
		<pallet_aura::Pallet<T> as OnTimestampSet<T::Moment>>::on_timestamp_set(moment)
	}
}

#[cfg(not(feature = "ts-tests"))]
impl<T: pallet_aura::Config> OnTimestampSet<T::Moment> for ConsensusOnTimestampSet<T> {
	fn on_timestamp_set(moment: T::Moment) {
		if EnableManualSeal::get() {
			log::info!("Manual sealing requires the ts-tests feature. Ignoring...");
		}
		<pallet_aura::Pallet<T> as OnTimestampSet<T::Moment>>::on_timestamp_set(moment)
	}
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = ConsensusOnTimestampSet<Self>;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

#[cfg(feature = "ts-tests")]
#[frame_support::pallet]
pub mod pallet_manual_seal {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T> {
		pub enable: bool,
		#[serde(skip)]
		pub _config: PhantomData<T>
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			EnableManualSeal::set(&self.enable);
		}
	}
}

#[cfg(not(feature = "ts-tests"))]
#[frame_support::pallet]
pub mod pallet_manual_seal {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {}
}

impl pallet_manual_seal::Config for Runtime {}

/// Special `ValidatorIdOf` implementation that is just returning the input as result.
pub struct ValidatorIdOf;
impl sp_runtime::traits::Convert<AccountId, Option<AccountId>> for ValidatorIdOf {
	fn convert(a: AccountId) -> Option<AccountId> { Some(a) }
}


impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = validator_set::ValidatorOf<Self>;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	// to be updated
	type SessionManager = ValidatorSet; 
	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
	type WeightInfo = ();
	type DisablingStrategy = ();
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = ConstU32<50>;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = 1 * OSLO;
	pub const PreimageByteDeposit: Balance = 1 * MILLIOSLO;
	pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<AccountId, Balances, PreimageHoldReason, LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeEvent,
		RuntimeCall,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask,
		RuntimeViewFunction
	)]
	#[derive(Eq, PartialEq, Clone)]
	pub struct Runtime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(1)]
	pub type Timestamp = pallet_timestamp;

	#[runtime::pallet_index(2)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(3)]
	pub type TransactionPayment = pallet_transaction_payment;

	#[runtime::pallet_index(4)]
	pub type ValidatorSet = validator_set;

	#[runtime::pallet_index(5)]
	pub type Authorship = pallet_authorship;

	#[runtime::pallet_index(6)]
	pub type Session = pallet_session;

	#[runtime::pallet_index(7)]
	pub type Aura = pallet_aura;

	#[runtime::pallet_index(8)]
	pub type Grandpa = pallet_grandpa;

	#[runtime::pallet_index(9)]
	pub type Sudo = pallet_sudo;

	#[runtime::pallet_index(10)]
	pub type Assets = pallet_assets;

	#[runtime::pallet_index(11)]
	pub type AssetRate = pallet_asset_rate;

	#[runtime::pallet_index(12)]
	pub type Ethereum = pallet_ethereum;

	#[runtime::pallet_index(13)]
	pub type EVM = pallet_evm;

	#[runtime::pallet_index(14)]
	pub type BaseFee = pallet_base_fee;

	#[runtime::pallet_index(15)]
	pub type ImOnline = pallet_im_online;

	#[runtime::pallet_index(16)]
	pub type Treasury = pallet_treasury;

	#[runtime::pallet_index(17)]
	pub type Democracy = pallet_democracy;

	#[runtime::pallet_index(18)]
	pub type Scheduler = pallet_scheduler;

	#[runtime::pallet_index(19)]
	pub type Council = pallet_collective::Pallet<Runtime, Instance1>;

	#[runtime::pallet_index(20)]
	pub type TechnicalCommittee = pallet_collective::Pallet<Runtime, Instance2>;

	#[runtime::pallet_index(21)]
	pub type Preimage = pallet_preimage;

	#[runtime::pallet_index(22)]
	pub type ManualSeal = pallet_manual_seal;
}

#[derive(Clone)]
pub struct TransactionConverter<B>(PhantomData<B>);

impl<B> Default for TransactionConverter<B> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<B: BlockT> fp_rpc::ConvertTransaction<<B as BlockT>::Extrinsic> for TransactionConverter<B> {
	fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> <B as BlockT>::Extrinsic {
		let extrinsic = UncheckedExtrinsic::new_bare(
			pallet_ethereum::Call::<Runtime>::transact { transaction }.into()
		);
		let encoded = extrinsic.encode();
		<B as BlockT>::Extrinsic::decode(&mut &encoded[..]).expect("Encoded extrinsic is always valid")
	}
}

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_democracy, Democracy]
		[pallet_collective, Council]
	);
}

impl fp_self_contained::SelfContainedCall for RuntimeCall {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			RuntimeCall::Ethereum(call) => call.is_self_contained(),
			_ => false
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => call.check_self_contained(),
			_ => None
		}
	}

	fn validate_self_contained(&self, info: &Self::SignedInfo, dispatch_info: &DispatchInfoOf<RuntimeCall>, len: usize) 
	-> Option<TransactionValidity> {
		match self {
			RuntimeCall::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
			_ => None
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		len: usize
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => {
				call.pre_dispatch_self_contained(info, dispatch_info, len)
			}
			_ => None
		}
	}

	fn apply_self_contained(self, info: Self::SignedInfo) 
	-> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => Some(call.dispatch(
				RuntimeOrigin::from(pallet_ethereum::RawOrigin::EthereumTransaction(info))
			)),
			_ => None
		}
	}
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion { VERSION }
		fn execute_block(block: Block) { Executive::execute_block(block) }
		fn initialize_block(header: &<Block as BlockT>::Header) -> ExtrinsicInclusionMode { Executive::initialize_block(header) }
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata { OpaqueMetadata::new(Runtime::metadata().into()) }
		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> { Runtime::metadata_at_version(version) }
		fn metadata_versions() -> sp_std::vec::Vec<u32> { Runtime::metadata_versions() }
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult { Executive::apply_extrinsic(extrinsic) }

		fn finalize_block() -> <Block as BlockT>::Header { Executive::finalize_block() }

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> { data.create_extrinsics() }

		fn check_inherents(block: Block, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult { data.check_extrinsics(&block) }
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(source: TransactionSource, tx: <Block as BlockT>::Extrinsic, block_hash: <Block as BlockT>::Hash)
		-> TransactionValidity { Executive::validate_transaction(source, tx, block_hash) }
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) { 
			Executive::offchain_worker(header) 
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<PresetId>) -> Option<Vec<u8>> {
			frame_support::genesis_builder_helper::get_preset::<RuntimeGenesisConfig>(id, genesis_config_preset::get_preset)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			vec![
				PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
				PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
			]
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> { opaque::SessionKeys::generate(seed) }

		fn decode_session_keys(encoded: Vec<u8> ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration { sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration()) }
		fn authorities() -> Vec<AuraId> { pallet_aura::Authorities::<Runtime>::get().into_inner() }
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList { Grandpa::grandpa_authorities() }
		fn current_set_id() -> fg_primitives::SetId { Grandpa::current_set_id() }

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<<Block as BlockT>::Hash, NumberFor<Block>>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof
		) -> Option<()> { None }

		fn generate_key_ownership_proof(_set_id: fg_primitives::SetId,_authority_id: GrandpaId) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've defined
			// our key owner proof type as a bottom type (i.e. a type with no values).
			None
		}
	}


	impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
		fn chain_id() -> u64 {<Runtime as pallet_evm::Config>::ChainId::get()}

		fn account_basic(address: H160) -> EVMAccount {
			let (account, _accountIndex) = pallet_evm::Pallet::<Runtime>::account_basic(&address);
			account
		}

		fn gas_price() -> U256 {
			let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
			gas_price
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			pallet_evm::AccountCodes::<Runtime>::get(address)
		}

		fn author() -> H160 { <pallet_evm::Pallet<Runtime>>::find_author() }

		fn storage_at(address: H160, index: U256) -> H256 {
			index.to_big_endian();
			pallet_evm::AccountStorages::<Runtime>::get(address, H256::from(index.to_big_endian()))
		}

		fn call(from: H160, to: H160, data: Vec<u8>, value: U256, gas_limit: U256, 
			max_fee_per_gas: Option<U256>, max_priority_fee_per_gas: Option<U256>, nonce: Option<U256>, 
			estimate: bool, access_list: Option<Vec<(H160, Vec<H256>)>>, authorization_list: Option<AuthorizationList>
		) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			// Estimated encoded transaction size must be based on the heaviest transaction
			// type (EIP7702Transaction) to be compatible with all transaction types.
			let mut estimated_transaction_len = data.len() +
				// pallet ethereum index: 1
				// transact call index: 1
				// Transaction enum variant: 1
				// chain_id 8 bytes
				// nonce: 32
				// max_priority_fee_per_gas: 32
				// max_fee_per_gas: 32
				// gas_limit: 32
				// action: 21 (enum varianrt + call address)
				// value: 32
				// access_list: 1 (empty vec size)
				// authorization_list: 1 (empty vec size)
				// 65 bytes signature
				259;

			if access_list.is_some() {
				estimated_transaction_len += access_list.encoded_size();
			}

			if authorization_list.is_some() {
				estimated_transaction_len += authorization_list.encoded_size();
			}

			let gas_limit = if gas_limit > U256::from(u64::MAX) {
				u64::MAX
			} else {
				gas_limit.low_u64()
			};

			let without_base_extrinsic_weight = true;

			let (weight_limit, proof_size_base_cost) =
				match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(gas_limit, without_base_extrinsic_weight) {
					weight_limit if weight_limit.proof_size() > 0 => {
						(Some(weight_limit), Some(estimated_transaction_len as u64))
					}
					_ => (None, None)
				};

			<Runtime as pallet_evm::Config>::Runner::call(
				from, to, data, value, gas_limit.unique_saturated_into(), max_fee_per_gas,
				max_priority_fee_per_gas, nonce, access_list.unwrap_or_default(),
				authorization_list.unwrap_or_default(), false, true, weight_limit, proof_size_base_cost,
				config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config())
			).map_err(|err| err.error.into())
		}

		fn create(from: H160, data: Vec<u8>, value: U256, gas_limit: U256, max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>, nonce: Option<U256>, estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>, authorization_list: Option<AuthorizationList>
		) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			let gas_limit = if gas_limit > U256::from(u64::MAX) {
				u64::MAX
			} else {
				gas_limit.low_u64()
			};

			let transaction_data = TransactionData::new(TransactionAction::Create,
				data.clone(), nonce.unwrap_or_default(), gas_limit.into(),
				None, max_fee_per_gas, max_priority_fee_per_gas, value,
				Some(<Runtime as pallet_evm::Config>::ChainId::get()),
				access_list.clone().unwrap_or_default(), authorization_list.clone().unwrap_or_default()
			);
			let (weight_limit, proof_size_base_cost) = pallet_ethereum::Pallet::<Runtime>::transaction_weight(&transaction_data);

			<Runtime as pallet_evm::Config>::Runner::create(
				from,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				authorization_list.unwrap_or_default(),
				false,
				true,
				weight_limit,
				proof_size_base_cost,
				config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config())
			).map_err(|err| err.error.into())
		}

		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
			pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
		}

		fn current_block() -> Option<pallet_ethereum::Block> { 
			pallet_ethereum::CurrentBlock::<Runtime>::get() 
		}

		fn elasticity() -> Option<Permill> { Some(pallet_base_fee::Elasticity::<Runtime>::get()) }

		fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> { 
			pallet_ethereum::CurrentReceipts::<Runtime>::get() 
		}

		fn current_all() -> (Option<pallet_ethereum::Block>, Option<Vec<pallet_ethereum::Receipt>>, Option<Vec<TransactionStatus>>) {(
			pallet_ethereum::CurrentBlock::<Runtime>::get(),
			pallet_ethereum::CurrentReceipts::<Runtime>::get(),
			pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
		)}

		fn gas_limit_multiplier_support() {}

		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>
		) -> Vec<EthereumTransaction> {
			xts.into_iter().filter_map(|xt| match xt.0.function {
				RuntimeCall::Ethereum(transact { transaction }) => Some(transaction),
				_ => None
			}).collect::<Vec<EthereumTransaction>>()
		}

		fn pending_block(xts: Vec<<Block as BlockT>::Extrinsic>) 
		-> (Option<pallet_ethereum::Block>, Option<Vec<TransactionStatus>>) {
			for ext in xts.into_iter() { let _ = Executive::apply_extrinsic(ext); }

			Ethereum::on_finalize(System::block_number() + 1);
			(
				pallet_ethereum::CurrentBlock::<Runtime>::get(),
				pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
			)
		}

		fn initialize_pending_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header);
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
		fn convert_transaction(transaction: EthereumTransaction) -> <Block as BlockT>::Extrinsic {
			UncheckedExtrinsic::new_bare(pallet_ethereum::Call::<Runtime>::transact { transaction }.into())
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) 
		-> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len) 
		}
		
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32)
		-> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		
		fn query_weight_to_fee(weight: Weight) -> Balance { TransactionPayment::weight_to_fee(weight) }
		fn query_length_to_fee(length: u32) -> Balance { TransactionPayment::length_to_fee(length) }
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall> for Runtime {
		fn query_call_info(call: RuntimeCall, len: u32) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> 
		{ TransactionPayment::query_call_info(call, len) }

		fn query_call_fee_details(call: RuntimeCall, len: u32) -> pallet_transaction_payment::FeeDetails<Balance> 
		{ TransactionPayment::query_call_fee_details(call, len) }

		fn query_weight_to_fee(weight: Weight) -> Balance { TransactionPayment::weight_to_fee(weight) }
		fn query_length_to_fee(length: u32) -> Balance { TransactionPayment::length_to_fee(length) }
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) 
		-> (Vec<frame_benchmarking::BenchmarkList>, Vec<frame_support::traits::StorageInfo>) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;
			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(config: frame_benchmarking::BenchmarkConfig) 
		-> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch};
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;
			use sp_storage::TrackedStorageKey;
			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{Runtime, WeightPerGas};
	#[test]
	fn configured_base_extrinsic_weight_is_evm_compatible() {
		let min_ethereum_transaction_weight = WeightPerGas::get() * 21_000;
		let base_extrinsic = <Runtime as frame_system::Config>::BlockWeights::get()
			.get(frame_support::dispatch::DispatchClass::Normal).base_extrinsic;
			
		assert!(base_extrinsic.ref_time() <= min_ethereum_transaction_weight.ref_time());
	}
}