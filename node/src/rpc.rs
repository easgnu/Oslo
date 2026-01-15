//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

use std::sync::Arc;
use std::collections::BTreeMap;
use jsonrpsee::RpcModule;
#[allow(unused_imports)]
use oslo_network_runtime::{AccountId, Balance, Nonce, Hash};
use sc_client_api::{backend::{Backend, StorageProvider, StateBackend}, client::BlockchainEvents, AuxStore, UsageProvider};
use sc_network_sync::SyncingService;
use sc_network::service::traits::NetworkService;
use sc_rpc::SubscriptionTaskExecutor;
use sc_transaction_pool_api::TransactionPool;
use sp_consensus_aura::{AuraApi, sr25519::AuthorityId as AuraId};
use sp_inherents::CreateInherentDataProviders;
use sp_api::{ProvideRuntimeApi, CallApiAt};
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use fc_rpc::{EthBlockDataCacheTask, pending::AuraConsensusDataProvider, EthApiServer, 
	EthSigner, EthDevSigner, Web3, Web3ApiServer, EthFilter, EthFilterApiServer, Debug, DebugApiServer,
	EthPubSub, EthPubSubApiServer, TxPool, TxPoolApiServer};
use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
use fc_mapping_sync::{EthereumBlockNotificationSinks, EthereumBlockNotification};
use fc_storage::StorageOverride;

#[cfg(feature = "ts-tests")]
use sc_consensus_manual_seal::{rpc::{ManualSeal, ManualSealApiServer}, EngineCommand};

use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fp_rpc::{ConvertTransaction, ConvertTransactionRuntimeApi, EthereumRuntimeRPCApi};
use sp_core::{H256};
use sp_runtime::{traits::{/*BlakeTwo256, */Block as BlockT}};
#[cfg(feature = "ts-tests")]
use futures::channel::mpsc;

/// EVM overrides
// pub fn overrides_handle<B, C, BE>(client: Arc<C>) -> Arc<StorageOverrideHandler<B, C, BE>>
// 	where
// 		C: ProvideRuntimeApi<B> + StorageProvider<B, BE> + AuxStore,
// 		C: HeaderBackend<B> + HeaderMetadata<B, Error=BlockChainError>,
// 		C: Send + Sync + 'static,
// 		C::Api: sp_api::ApiExt<B> + EthereumRuntimeRPCApi<B> + ConvertTransactionRuntimeApi<B>,
// 		BE: Backend<B> + 'static,
// 		BE::State: StateBackend<BlakeTwo256>,
// 		B: BlockT
// { Arc::new(StorageOverrideHandler::new(client.clone())) }


pub struct DefaultEthConfig<C, BE>(std::marker::PhantomData<(C, BE)>);

impl<B, C, BE> fc_rpc::EthConfig<B, C> for DefaultEthConfig<C, BE>
where
	B: BlockT,
	C: StorageProvider<B, BE> + Sync + Send + 'static,
	BE: Backend<B> + 'static
{
	type EstimateGasAdapter = ();
	type RuntimeStorageOverride = fc_rpc::frontier_backend_client::SystemAccountId32StorageOverride<B, C, BE>;
}


/// Full client dependencies.
/// Extra dependencies for Ethereum compatibility.
pub struct FullDeps<B: BlockT, C, P, CT, CIDP> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Manual seal command sink
	#[cfg(feature = "ts-tests")]
	pub command_sink: Option<mpsc::Sender<EngineCommand<Hash>>>,
	/// Ethereum transaction converter.
	pub converter: Option<CT>,
	/// The Node authority flag
	pub is_authority: bool,
	/// Network service
	pub network: Arc<dyn NetworkService>,
	/// Chain syncing service
	pub sync: Arc<SyncingService<B>>,
	/// Frontier Backend.
	pub frontier_backend: Arc<dyn fc_api::Backend<B>>,
	/// Ethereum data access overrides.
	pub storage_override: Arc<dyn StorageOverride<B>>,
	/// Cache for Ethereum block data.
	pub block_data_cache: Arc<EthBlockDataCacheTask<B>>,
	/// EthFilterApi pool.
	pub filter_pool: Option<FilterPool>,
	/// Maximum number of logs in a query.
	pub max_past_logs: u32,
	/// Maximum block range for eth_getLogs.
	pub max_block_range: u32,
	/// Fee history cache.
	pub fee_history_cache: FeeHistoryCache,
	/// Maximum fee history cache size.
	pub fee_history_cache_limit: FeeHistoryCacheLimit,
	/// Maximum allowed gas limit will be ` block.gas_limit * execute_gas_limit_multiplier` when
	/// using eth_call/eth_estimateGas.
	pub execute_gas_limit_multiplier: u64,
	/// Mandated parent hashes for a given block hash.
	pub forced_parent_hashes: Option<BTreeMap<H256, H256>>,
	/// Something that can create the inherent data providers for pending state
	pub pending_create_inherent_data_providers: CIDP
}



/// Instantiate all full RPC extensions.
pub fn create_full<B, C, BE, P, CT, CIDP>(
	deps: FullDeps<B, C, P, CT, CIDP>,
	subscription_task_executor: SubscriptionTaskExecutor,
	pubsub_notification_sinks: Arc<EthereumBlockNotificationSinks<EthereumBlockNotification<B>>>
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
	where
		B: BlockT<Hash = H256>,
		C: CallApiAt<B> + ProvideRuntimeApi<B> + AuxStore + UsageProvider<B>,
		C: BlockchainEvents<B> + StorageProvider<B, BE>,
		C: HeaderBackend<B> + HeaderMetadata<B, Error=BlockChainError> + 'static,
		C: Send + Sync + 'static,
		C::Api: substrate_frame_rpc_system::AccountNonceApi<B, AccountId, Nonce>,
		C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<B, Balance>,
		C::Api: AuraApi<B, AuraId> + BlockBuilder<B> + ConvertTransactionRuntimeApi<B> + EthereumRuntimeRPCApi<B>,
		BE: Backend<B> + 'static,
		BE::State: StateBackend<sp_runtime::traits::HashingFor<B>>,
		P: TransactionPool<Block=B, Hash=B::Hash> + 'static,
		CT: ConvertTransaction<<B as BlockT>::Extrinsic> + Send + Sync + 'static,
		CIDP: CreateInherentDataProviders<B, ()> + Send + 'static
{
	use fc_rpc::{Eth, Net, NetApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};
	let mut module = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		#[cfg(feature = "ts-tests")]
		command_sink,
		converter,
		is_authority,
		network,
		sync,
		frontier_backend,
		storage_override,
		block_data_cache,
		filter_pool,
		max_past_logs,
		max_block_range,
		fee_history_cache,
		fee_history_cache_limit,
		execute_gas_limit_multiplier,
		forced_parent_hashes,
		pending_create_inherent_data_providers
	} = deps;

	let mut signers = Vec::new();
	signers.push(Box::new(EthDevSigner::new()) as Box<dyn EthSigner>);
	//signers.push(Box::new(oslo_network_runtime::MultiSigner::Ecdsa) as Box<dyn EthSigner>);

	module.merge(System::new(client.clone(), pool.clone()).into_rpc())?;
	module.merge(Web3::new(client.clone()).into_rpc())?;

	module.merge(Net::new(client.clone(), network, true).into_rpc())?;

	#[cfg(feature = "ts-tests")]
	if let Some(command_sink) = command_sink {
		module.merge(
			// We provide the rpc handler with the sending end of the channel to allow the rpc
			// send EngineCommands to the background block authorship task.
			ManualSeal::new(command_sink).into_rpc()
		)?;
	}

	module.merge(Eth::<B, C, P, CT, BE, CIDP, DefaultEthConfig<C, BE>>::new(
		client.clone(),
		pool.clone(),
		converter,
		sync.clone(),
		signers,
		storage_override.clone(),
		frontier_backend.clone(),
		is_authority,
		block_data_cache.clone(),
		fee_history_cache,
		fee_history_cache_limit,
		execute_gas_limit_multiplier,
		forced_parent_hashes,
		pending_create_inherent_data_providers,
		Some(Box::new(AuraConsensusDataProvider::new(client.clone())))
	).replace_config::<DefaultEthConfig<C, BE>>().into_rpc())?;
	module.merge(Debug::new(client.clone(), frontier_backend.clone(), storage_override.clone(), block_data_cache.clone()).into_rpc())?;
	module.merge(EthPubSub::new(
		pool.clone(), client.clone(), sync, subscription_task_executor, storage_override.clone(), pubsub_notification_sinks
	).into_rpc())?;

	if let Some(filter_pool) = filter_pool {
		module.merge(EthFilter::new(
			client.clone(),
			frontier_backend.clone(),
			pool.clone(),
			filter_pool,
			500_usize, // max stored filters
			max_past_logs,
			max_block_range,
			block_data_cache.clone()
		).into_rpc())?;
	}

	module.merge(TxPool::new(client.clone(), pool).into_rpc())?;
	module.merge(TransactionPayment::new(client).into_rpc())?;
	Ok(module)
}
