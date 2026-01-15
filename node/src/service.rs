//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.
#![allow(unused_imports)]
use std::{cell::RefCell, sync::Arc, time::Duration};
use futures::{future, channel::mpsc, prelude::*};
use sc_network_sync::SyncingService;
use prometheus_endpoint::Registry;
use oslo_network_runtime::{self, /*opaque::Block,*/ TransactionConverter, AccountId, Balance,
  Nonce};
use sc_client_api::{BlockBackend, Backend as BackendT, BlockchainEvents};
use sc_consensus::{BasicQueue, BoxBlockImport};
use sc_consensus_aura::{/*ImportQueueParams, */SlotProportion, StartAuraParams};
use sp_consensus_aura::sr25519::{AuthorityId as AuraId};
use sc_consensus_grandpa::{SharedVoterState, BlockNumberOps, block_import};
use sc_network::{config::FullNetworkConfiguration, NetworkWorker};
use sc_network_sync::strategy::warp::{WarpSyncConfig, WarpSyncProvider};
use sc_service::{error::Error as ServiceError, Configuration, TaskManager, new_wasm_executor, LocalCallExecutor};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker};
use sc_transaction_pool::TransactionPoolHandle;
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use std::{sync::Mutex, collections::BTreeMap, path::PathBuf};
//use fc_mapping_sync::{kv::MappingSyncWorker, SyncStrategy};
use fc_rpc::EthTask;
use sc_executor::WasmExecutor;
use fc_storage::{StorageOverride, StorageOverrideHandler};
use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fc_consensus::FrontierBlockImport;
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sc_executor::HostFunctions as HostFunctionsT;
use sp_runtime::AccountId32;
use sp_core::{H256, U256};
use sp_runtime::traits::{Block as BlockT, NumberFor};
use crate::{cli::Sealing, client::{BaseRuntimeApiCollection, FullBackend,
	FullClient, RuntimeApiCollection, EthCompatRuntimeApiCollection}};
use sp_api::ConstructRuntimeApi;
use pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi;
use crate::rpc::create_full;
use crate::rpc::FullDeps;
//use sp_runtime::traits::{BlakeTwo256};

#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = (
	sp_io::SubstrateHostFunctions,
	frame_benchmarking::benchmarking::HostFunctions,
	cumulus_primitives_proof_size_hostfunction::storage_proof_size::HostFunctions
);

#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = (
	sp_io::SubstrateHostFunctions,
	cumulus_primitives_proof_size_hostfunction::storage_proof_size::HostFunctions
);

type FullSelectChain<B> = sc_consensus::LongestChain<FullBackend<B>, B>;

type GrandpaBlockImport<B, C> = sc_consensus_grandpa::GrandpaBlockImport<FullBackend<B>, B, C, FullSelectChain<B>>;
type GrandpaLinkHalf<B, C> = sc_consensus_grandpa::LinkHalf<B, C, FullSelectChain<B>>;

const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

pub fn frontier_database_dir(config: &Configuration) -> std::path::PathBuf {
	config.base_path.path().join("frontier").join("db")
}

pub fn db_config_dir(config: &Configuration) -> PathBuf {
	config.base_path.config_dir(config.chain_spec.id())
}

/// The ethereum-compatibility configuration used to run a node.
#[derive(Clone, Debug, clap::Parser)]
pub struct EthConfiguration {
	/// Maximum number of logs in a query.
	#[arg(long, default_value = "10000")]
	pub max_past_logs: u32,

	/// Maximum allowed gas limit will be `block.gas_limit * execute_gas_limit_multiplier`
	/// when using eth_call/eth_estimateGas.
	#[arg(long, default_value = "10")]
	pub execute_gas_limit_multiplier: u64,

	/// Maximum fee history cache size.
	#[arg(long, default_value = "2048")]
	pub fee_history_limit: u64,

	/// Size in bytes of the LRU cache for block data.
	#[arg(long, default_value = "50")]
	pub eth_log_block_cache: usize,

	/// Size in bytes of the LRU cache for transactions statuses data.
	#[arg(long, default_value = "50")]
	pub eth_statuses_cache: usize,

	/// The dynamic-fee pallet target gas price set by block author
	#[arg(long, default_value = "1")]
	pub target_gas_price: u64
}


pub fn build_aura_grandpa_import_queue<B, RA, HF>(
	client: Arc<FullClient<B, RA, HF>>,
	config: &Configuration,
	eth_config: &EthConfiguration,
	task_manager: &TaskManager,
	telemetry: Option<TelemetryHandle>,
	grandpa_block_import: GrandpaBlockImport<B, FullClient<B, RA, HF>>
) -> Result<(BasicQueue<B>, BoxBlockImport<B>), ServiceError>
where
  B: BlockT,
	NumberFor<B>: sc_consensus_grandpa::BlockNumberOps,
	RA: sp_api::ConstructRuntimeApi<B, FullClient<B, RA, HF>>,
	RA: Send + Sync + 'static,
	RA::RuntimeApi: RuntimeApiCollection<B, AuraId, AccountId, Nonce, Balance>,
	HF: HostFunctionsT + 'static
{
	let frontier_block_import = FrontierBlockImport::new(grandpa_block_import.clone(), client.clone());

	let slot_duration = sc_consensus_aura::slot_duration(&*client)?;
	let target_gas_price = eth_config.target_gas_price;
	let create_inherent_data_providers = move |_, ()| async move {
		let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
		let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
			*timestamp,
			slot_duration
		);
		let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
		Ok((slot, timestamp, dynamic_fee))
	};

	let import_queue = sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(
		sc_consensus_aura::ImportQueueParams {
			block_import: frontier_block_import.clone(),
			justification_import: Some(Box::new(grandpa_block_import)),
			client,
			create_inherent_data_providers,
			spawner: &task_manager.spawn_essential_handle(),
			registry: config.prometheus_registry(),
			check_for_equivocation: Default::default(),
			telemetry,
			compatibility_mode: sc_consensus_aura::CompatibilityMode::None
		}
	).map_err::<ServiceError, _>(Into::into)?;

	Ok((import_queue, Box::new(frontier_block_import)))
}

/// Frontier DB backend type.
pub type FrontierBackend<B, C> = fc_db::kv::Backend<B, C>;

/// Build the import queue for the template runtime (manual seal).
#[cfg(feature = "ts-tests")]
pub fn build_manual_seal_import_queue<B, RA, HF>(
	client: Arc<FullClient<B, RA, HF>>,
	config: &Configuration,
	_eth_config: &EthConfiguration,
	task_manager: &TaskManager,
	_telemetry: Option<TelemetryHandle>,
	_grandpa_block_import: GrandpaBlockImport<B, FullClient<B, RA, HF>>
) -> Result<(BasicQueue<B>, BoxBlockImport<B>), ServiceError>
where
  B: BlockT,
	RA: sp_api::ConstructRuntimeApi<B, FullClient<B, RA, HF>>,
	RA: Send + Sync + 'static,
	RA::RuntimeApi: RuntimeApiCollection<B, AuraId, AccountId, Nonce, Balance>,
	HF: HostFunctionsT + 'static
{
	let frontier_block_import = FrontierBlockImport::new(client.clone(), client);
	Ok((
		sc_consensus_manual_seal::import_queue(
			Box::new(frontier_block_import.clone()),
			&task_manager.spawn_essential_handle(),
			config.prometheus_registry()
		),
		Box::new(frontier_block_import)
	))
}


pub fn new_partial<B, RA, HF>(config: &Configuration, eth_config: &EthConfiguration, sealing: Option<Sealing>) -> Result<
	sc_service::PartialComponents<
	FullClient<B, RA, HF>, FullBackend<B>, FullSelectChain<B>,
	sc_consensus::BasicQueue<B>,
	sc_transaction_pool::TransactionPoolHandle<B, FullClient<B, RA, HF>>,
	(
		Option<Telemetry>,
		BoxBlockImport<B>,
		GrandpaLinkHalf<B, FullClient<B, RA, HF>>,
		FrontierBackend<B, FullClient<B, RA, HF>>,
		Arc<dyn StorageOverride<B>>,
		(FeeHistoryCache, FeeHistoryCacheLimit)
	)>, ServiceError>
	where
		B: BlockT<Hash = H256>,
		NumberFor<B>: sc_consensus_grandpa::BlockNumberOps,
		RA: sp_api::ConstructRuntimeApi<B, FullClient<B, RA, HF>>,
		RA: Send + Sync + 'static,
		RA::RuntimeApi: BaseRuntimeApiCollection<B> + EthCompatRuntimeApiCollection<B>,
		HF: HostFunctionsT + 'static,
		<RA as ConstructRuntimeApi<B, sc_service::client::Client<sc_client_db::Backend<B>, LocalCallExecutor<B, sc_client_db::Backend<B>, WasmExecutor<HF>>, B, RA>>>::RuntimeApi: sp_consensus_aura::AuraApi<B, AuraId>,
		<RA as ConstructRuntimeApi<B, sc_service::client::Client<sc_client_db::Backend<B>, LocalCallExecutor<B, sc_client_db::Backend<B>, WasmExecutor<HF>>, B, RA>>>::RuntimeApi: sp_consensus_grandpa::GrandpaApi<B>,
		<RA as ConstructRuntimeApi<B, sc_service::client::Client<sc_client_db::Backend<B>, LocalCallExecutor<B, sc_client_db::Backend<B>, WasmExecutor<HF>>, B, RA>>>::RuntimeApi: frame_system_rpc_runtime_api::AccountNonceApi<B, AccountId32, u32>,
		<RA as ConstructRuntimeApi<B, sc_service::client::Client<sc_client_db::Backend<B>, LocalCallExecutor<B, sc_client_db::Backend<B>, WasmExecutor<HF>>, B, RA>>>::RuntimeApi: TransactionPaymentRuntimeApi<B, u128>
	{
	let telemetry = config.telemetry_endpoints.clone().filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		}).transpose()?;

	let executor = new_wasm_executor(&config.executor);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts_record_import::<B, RA, _>(config, telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()), executor, true)?;
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());
	let (grandpa_block_import, grandpa_link) = block_import(
		client.clone(),
		GRANDPA_JUSTIFICATION_PERIOD,
		&client,
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle())
	)?;

	let storage_override = Arc::new(fc_rpc::StorageOverrideHandler::<B, _, _>::new(client.clone()));
	let frontier_backend = FrontierBackend::<B, FullClient<B, RA, HF>>::open(
		Arc::clone(&client),
		&config.database,
		&db_config_dir(config)
	)?;


	let fee_history_limit: u64 = 2048;
	let fee_history_cache: FeeHistoryCache = Arc::new(Mutex::new(BTreeMap::new()));
	let fee_history_cache_limit: FeeHistoryCacheLimit = fee_history_limit;
	let fee_history = (fee_history_cache, fee_history_cache_limit);
	
	


	#[cfg(feature = "ts-tests")]
	let build_import_queue = if sealing.is_some() {
		build_manual_seal_import_queue::<B, RA, HF>
	} else {
		build_aura_grandpa_import_queue::<B, RA, HF>
	};
	
	#[cfg(not(feature = "ts-tests"))]
	let build_import_queue = build_aura_grandpa_import_queue::<B, RA, HF>;


	let (import_queue, block_import) = build_import_queue(
		client.clone(),
		&config,
		&eth_config,
		&task_manager,
		telemetry.as_ref().map(|x| x.handle()),
		grandpa_block_import
	)?;

	let transaction_pool = Arc::from(sc_transaction_pool::Builder::new(
			task_manager.spawn_essential_handle(),
			client.clone(),
			config.role.is_authority().into()
		).with_options(config.transaction_pool.clone())
		.with_prometheus(config.prometheus_registry()).build()
	);

	Ok(sc_service::PartialComponents {
		client, backend, task_manager, keystore_container, select_chain, import_queue, transaction_pool,
		other: (telemetry, block_import, grandpa_link, frontier_backend, storage_override, fee_history)
	})
}

pub async fn spawn_frontier_tasks<B, RA, HF>(
	task_manager: &TaskManager,
	client: Arc<FullClient<B, RA, HF>>,
	backend: Arc<FullBackend<B>>,
	frontier_backend: Arc<FrontierBackend<B, FullClient<B, RA, HF>>>,
	filter_pool: Option<FilterPool>,
	storage_override: Arc<dyn StorageOverride<B>>,
	fee_history_cache: FeeHistoryCache,
	fee_history_cache_limit: FeeHistoryCacheLimit,
	sync: Arc<SyncingService<B>>,
	pubsub_notification_sinks: Arc<
		fc_mapping_sync::EthereumBlockNotificationSinks<
			fc_mapping_sync::EthereumBlockNotification<B>
		>
	>
) where
	B: BlockT<Hash = H256>,
	RA: ConstructRuntimeApi<B, FullClient<B, RA, HF>>,
	RA: Send + Sync + 'static,
	RA::RuntimeApi: EthCompatRuntimeApiCollection<B>,
	HF: HostFunctionsT + 'static
{
	// Spawn main mapping sync worker background task.

	task_manager.spawn_essential_handle().spawn(
		"frontier-mapping-sync-worker",
		Some("frontier"),
		fc_mapping_sync::kv::MappingSyncWorker::new(
			client.import_notification_stream(),
			std::time::Duration::new(30, 0),
			client.clone(),
			backend,
			storage_override.clone(),
			frontier_backend.clone(),
			3,
			0u32.into(),
			fc_mapping_sync::SyncStrategy::Normal,
			sync,
			pubsub_notification_sinks
		).for_each(|()| future::ready(()))
	);

	// Spawn Frontier EthFilterApi maintenance task.
	if let Some(filter_pool) = filter_pool {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			Some("frontier"),
			EthTask::filter_pool_task(client.clone(), filter_pool.clone(), FILTER_RETAIN_THRESHOLD)
		);
	}

	// Spawn Frontier FeeHistory cache maintenance task.
	task_manager.spawn_essential_handle().spawn(
		"frontier-fee-history",
		Some("frontier"),
		EthTask::fee_history_task(
			client,
			storage_override,
			fee_history_cache,
			fee_history_cache_limit
		)
	);
}

/// Builds a new service for a full client.
pub async fn new_full<B, RA, HF, NB>(mut config: Configuration, eth_config: &EthConfiguration, sealing: Option<Sealing>) -> Result<TaskManager, ServiceError> 
	where
		B: BlockT<Hash = H256>,
		NumberFor<B>: BlockNumberOps,
		<B as BlockT>::Header: Unpin,
		RA: sp_api::ConstructRuntimeApi<B, FullClient<B, RA, HF>>,
		RA: Send + Sync + 'static,
		RA::RuntimeApi: RuntimeApiCollection<B, AuraId, AccountId, Nonce, Balance>,
		HF: HostFunctionsT + 'static,
		NB: sc_network::NetworkBackend<B, <B as BlockT>::Hash>
{
	let sc_service::PartialComponents {
		client, backend, mut task_manager, keystore_container, select_chain, import_queue, transaction_pool,
		other: (mut telemetry, frontier_block_import, grandpa_link, frontier_backend, storage_override, fee_history)
	} = new_partial::<B, RA, HF>(&config, &eth_config, sealing)?;

  let maybe_registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
	let mut net_config = FullNetworkConfiguration::<_, _, NB>::new(&config.network, maybe_registry.cloned());

	let peer_store_handle = net_config.peer_store_handle();
	let metrics = <NetworkWorker<B, <B as BlockT>::Hash> as sc_network::NetworkBackend::<B, <B as BlockT>::Hash>>::register_notification_metrics(maybe_registry);
	let grandpa_protocol_name = sc_consensus_grandpa::protocol_standard_name(&client.block_hash(0u32.into()).ok().flatten().expect("Genesis block exists; qed"), &config.chain_spec);
	let (grandpa_protocol_config, grandpa_notification_service) = 
		sc_consensus_grandpa::grandpa_peers_set_config::<_, NB>(grandpa_protocol_name.clone(), metrics.clone(), peer_store_handle);

	let warp_sync_config = if sealing.is_some() {
		None
	} else {
		net_config.add_notification_protocol(grandpa_protocol_config);
		let warp_sync: Arc<dyn WarpSyncProvider<B>> = Arc::new(
			sc_consensus_grandpa::warp_proof::NetworkProvider::new(
				backend.clone(),
				grandpa_link.shared_authority_set().clone(),
				Vec::new()
			)
		);
		Some(WarpSyncConfig::WithProvider(warp_sync))
	};

	let (network, system_rpc_tx, tx_handler_controller, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_config,
			block_relay: None,
			metrics
		})?;

	if config.offchain_worker.enabled || config.role.is_authority() {
		let offchain_workers = sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
			runtime_api_provider: client.clone(),
			keystore: Some(keystore_container.keystore()),
			offchain_db: backend.offchain_storage(),
			transaction_pool: Some(OffchainTransactionPoolFactory::new(transaction_pool.clone())),
			network_provider: Arc::new(network.clone()),
			is_validator: config.role.is_authority(),
			enable_http_requests: true,
			custom_extensions: |_| vec![]
		})?;
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-worker",
			offchain_workers.run(client.clone(), task_manager.spawn_handle()).boxed()
		);
	}
	
	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let name = config.network.node_name.clone();
	let frontier_backend = Arc::new(frontier_backend);
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();
	let (fee_history_cache, fee_history_cache_limit) = fee_history;

  // Channel for the rpc handler to communicate with the authorship task.
	#[cfg(feature = "ts-tests")]
	let (command_sink, commands_stream) = mpsc::channel(1000);

	// Sinks for pubsub notifications.
	// Everytime a new subscription is created, a new mpsc channel is added to the sink pool.
	// The MappingSyncWorker sends through the channel on block import and the subscription emits a notification to the subscriber on receiving a message through this channel.
	// This way we avoid race conditions when using native substrate block import notification stream.
	let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<fc_mapping_sync::EthereumBlockNotification<B>> = Default::default();
	let filter_pool: Option<FilterPool> = Some(Arc::new(Mutex::new(BTreeMap::new())));
	let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);
	
	
	// Spawn Frontier EthFilterApi maintenance task.
	if let Some(ref filter_pool) = filter_pool {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			Some("frontier"),
			EthTask::filter_pool_task(client.clone(), filter_pool.clone(), FILTER_RETAIN_THRESHOLD)
		);
	}

	// for ethereum-compatibility rpc.
	config.rpc.id_provider = Some(Box::new(fc_rpc::EthereumSubIdProvider));

	let rpc_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let network = network.clone();
		let sync_service = sync_service.clone();
		let is_authority = role.is_authority();
		let max_past_logs: u32 = 1024;
		let execute_gas_limit_multiplier: u64 = 10;
		let filter_pool = filter_pool.clone();
		let frontier_backend = frontier_backend.clone();
		let pubsub_notification_sinks = pubsub_notification_sinks.clone();
		let storage_override = storage_override.clone();
		let fee_history_cache = fee_history_cache.clone();
		let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
			task_manager.spawn_handle(), storage_override.clone(), 10, 50, prometheus_registry.clone()
		));
		let slot_duration = sc_consensus_aura::slot_duration(&*client)?;
		let target_gas_price: u64 = 1;
		let pending_create_inherent_data_providers = move |_, ()| async move {
			let current = sp_timestamp::InherentDataProvider::from_system_time();
			let next_slot = current.timestamp().as_millis() + slot_duration.as_millis();
			let timestamp = sp_timestamp::InherentDataProvider::new(next_slot.into());
			let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(*timestamp, slot_duration);
			let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
			Ok((slot, timestamp, dynamic_fee))
		};

		Box::new(move |subscription_task_executor| {
			let deps = FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				#[cfg(feature = "ts-tests")]
				command_sink: if sealing.is_some() {
					Some(command_sink.clone())
				} else {
					None
				},
				converter: Some(TransactionConverter::<B>::default()),
				is_authority,
				network: network.clone(),
				sync: sync_service.clone(),
				frontier_backend: frontier_backend.clone(),
				storage_override: storage_override.clone(),
				block_data_cache: block_data_cache.clone(),
				filter_pool: filter_pool.clone(),
				max_past_logs,
				max_block_range: 2400, // max block range, I set this equal to block hash count.
				fee_history_cache: fee_history_cache.clone(),
				fee_history_cache_limit,
				execute_gas_limit_multiplier,
				forced_parent_hashes: None,
				pending_create_inherent_data_providers
			};
			create_full(deps, subscription_task_executor, pubsub_notification_sinks.clone()).map_err(Into::into)
		})
	};

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config,
		client: client.clone(),
		backend: backend.clone(),
		task_manager: &mut task_manager,
		keystore: keystore_container.keystore(),
		transaction_pool: transaction_pool.clone(),
		rpc_builder,
		network: network.clone(),
		system_rpc_tx,
		tx_handler_controller,
		sync_service: sync_service.clone(),
		telemetry: telemetry.as_mut()
	})?;

	spawn_frontier_tasks(
		&task_manager,
		client.clone(),
		backend,
		frontier_backend.clone(),
		filter_pool,
		storage_override,
		fee_history_cache,
		fee_history_cache_limit,
		sync_service.clone(),
		pubsub_notification_sinks
	).await;

	if role.is_authority() {
		// manual-seal authorship
		#[cfg(feature = "ts-tests")]
		if let Some(sealing) = sealing {
			run_manual_seal_authorship(
				&eth_config,
				sealing,
				client,
				transaction_pool,
				select_chain,
				frontier_block_import,
				&task_manager,
				prometheus_registry.as_ref(),
				telemetry.as_ref(),
				commands_stream
			)?;

			log::info!("Manual Seal Ready");
			return Ok(task_manager);
		}

		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle())
		);
		let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

		let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
			StartAuraParams {
				slot_duration,
				client: client.clone(),
				select_chain,
				block_import: frontier_block_import,
				proposer_factory,
				sync_oracle: sync_service.clone(),
				justification_sync_link: sync_service.clone(),
				create_inherent_data_providers: move |_, ()| async move {
					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
					let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(*timestamp, slot_duration);
					Ok((slot, timestamp))
				},
				force_authoring,
				backoff_authoring_blocks: Option::<()>::None,
				keystore: keystore_container.keystore(),
				block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
				max_block_proposal_slot_portion: None,
				telemetry: telemetry.as_ref().map(|x| x.handle()),
				compatibility_mode: Default::default()
			}
		)?;

		// the AURA authoring task is considered essential, i.e. if it
		// fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking("aura", Some("block-authoring"), aura);
	}

	if enable_grandpa {
		// if the node isn't actively participating in consensus then it doesn't
		// need a keystore, regardless of which protocol we use below.
		let keystore = if role.is_authority() { Some(keystore_container.keystore()) } else { None };

		let grandpa_config = sc_consensus_grandpa::Config {
			gossip_duration: Duration::from_millis(1665),
			justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
			name: Some(name),
			observer_enabled: false,
			keystore,
			local_role: role,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			protocol_name: grandpa_protocol_name
		};

		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let full_grandpa_config = sc_consensus_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network,
			sync: sync_service,
			notification_service: grandpa_notification_service,
			voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state: SharedVoterState::empty(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool)
		};
		let grandpa_voter = sc_consensus_grandpa::run_grandpa_voter(full_grandpa_config)?;
		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking("grandpa-voter", None, grandpa_voter);
	}

	Ok(task_manager)
}

#[cfg(feature = "ts-tests")]
fn run_manual_seal_authorship<B, RA, HF>(
	eth_config: &EthConfiguration,
	sealing: Sealing,
	client: Arc<FullClient<B, RA, HF>>,
	transaction_pool: Arc<TransactionPoolHandle<B, FullClient<B, RA, HF>>>,
	select_chain: FullSelectChain<B>,
	block_import: BoxBlockImport<B>,
	task_manager: &TaskManager,
	prometheus_registry: Option<&Registry>,
	telemetry: Option<&Telemetry>,
	commands_stream: mpsc::Receiver<sc_consensus_manual_seal::rpc::EngineCommand<<B as BlockT>::Hash>>
) -> Result<(), ServiceError>
where
	B: BlockT,
	RA: ConstructRuntimeApi<B, FullClient<B, RA, HF>>,
	RA: Send + Sync + 'static,
	RA::RuntimeApi: RuntimeApiCollection<B, AuraId, AccountId, Nonce, Balance>,
	HF: HostFunctionsT + 'static
{
	let proposer_factory = sc_basic_authorship::ProposerFactory::new(
		task_manager.spawn_handle(),
		client.clone(),
		transaction_pool.clone(),
		prometheus_registry,
		telemetry.as_ref().map(|x| x.handle())
	);

	thread_local!(static TIMESTAMP: RefCell<u64> = const { RefCell::new(0) });

	/// Provide a mock duration starting at 0 in millisecond for timestamp inherent.
	/// Each call will increment timestamp by slot_duration making Aura think time has passed.
	struct MockTimestampInherentDataProvider;

	#[async_trait::async_trait]
	impl sp_inherents::InherentDataProvider for MockTimestampInherentDataProvider {
		async fn provide_inherent_data(
			&self,
			inherent_data: &mut sp_inherents::InherentData
		) -> Result<(), sp_inherents::Error> {
			TIMESTAMP.with(|x| {
				*x.borrow_mut() += oslo_network_runtime::SLOT_DURATION;
				inherent_data.put_data(sp_timestamp::INHERENT_IDENTIFIER, &*x.borrow())
			})
		}

		async fn try_handle_error(&self, _identifier: &sp_inherents::InherentIdentifier,_error: &[u8])
		-> Option<Result<(), sp_inherents::Error>> {
			// The pallet never reports error.
			None
		}
	}

	let target_gas_price = eth_config.target_gas_price;
	let create_inherent_data_providers = move |_, ()| async move {
		let timestamp = MockTimestampInherentDataProvider;
		let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
		Ok((timestamp, dynamic_fee))
	};

	let manual_seal = match sealing {
		Sealing::Manual => future::Either::Left(sc_consensus_manual_seal::run_manual_seal(
			sc_consensus_manual_seal::ManualSealParams {
				block_import,
				env: proposer_factory,
				client,
				pool: transaction_pool,
				commands_stream,
				select_chain,
				consensus_data_provider: None,
				create_inherent_data_providers
			}
		)),
		Sealing::Instant => future::Either::Right(sc_consensus_manual_seal::run_instant_seal(
			sc_consensus_manual_seal::InstantSealParams {
				block_import,
				env: proposer_factory,
				client,
				pool: transaction_pool,
				select_chain,
				consensus_data_provider: None,
				create_inherent_data_providers
			}
		))
	};

	// we spawn the future on a background thread managed by service.
	task_manager.spawn_essential_handle().spawn_blocking("manual-seal", None, manual_seal);
	Ok(())
}

