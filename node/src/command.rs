use crate::{chain_spec, cli::{Cli, Subcommand}, service};
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use sc_cli::SubstrateCli;
use futures::TryFutureExt;
use sc_service::{PartialComponents};
use service::{frontier_database_dir, HostFunctions};
use oslo_network_runtime::{opaque::Block, RuntimeApi};

impl SubstrateCli for Cli {
	fn impl_name() -> String { "oslo-network".into() }
	fn impl_version() -> String { env!("SUBSTRATE_CLI_IMPL_VERSION").into() }
	fn description() -> String { env!("CARGO_PKG_DESCRIPTION").into() }
	fn author() -> String { env!("CARGO_PKG_AUTHORS").into() }
	fn support_url() -> String { "oslocrypto.com".into() }
	fn copyright_start_year() -> i32 { 2017 }

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_cli::ChainSpec>, String> {
		Ok(match id {
			"dev" => {
				let enable_manual_seal = self.sealing.map(|_| true).unwrap_or_default();
				Box::new(chain_spec::development_config(enable_manual_seal)?) 
			}
			"" | "testnet" | "local" => Box::new(chain_spec::testnet_config()?),
			"live" => Box::new(chain_spec::public_config()?),
			path => Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?)
		})
	}
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		}
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, import_queue, .. } = service::new_partial::<Block, RuntimeApi, HostFunctions>(&config, &cli.eth, cli.sealing)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } = service::new_partial::<Block, RuntimeApi, HostFunctions>(&config, &cli.eth, cli.sealing)?;
				Ok((cmd.run(client, fc_db::kv::DatabaseSource::RocksDb {
					path: frontier_database_dir(&config), cache_size: 100}),
					task_manager
				))
			})
		}
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } = service::new_partial::<Block, RuntimeApi, HostFunctions>(&config, &cli.eth, cli.sealing)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, import_queue, .. } = service::new_partial::<Block, RuntimeApi, HostFunctions>(&config, &cli.eth, cli.sealing)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(fc_db::kv::DatabaseSource::RocksDb {
				path: frontier_database_dir(&config),
				cache_size: 0
			}))
		}
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, backend, .. } = service::new_partial::<Block, RuntimeApi, HostFunctions>(&config, &cli.eth, cli.sealing)?;
				let aux_revert = Box::new(|client, _, blocks| {
					sc_consensus_grandpa::revert(client, blocks)?;
					Ok(())
				});	
				Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
			})
		}
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| {
				// This switch needs to be in the client, since the client decides
				// which sub-commands it wants to support.
				match cmd {
					BenchmarkCmd::Pallet(cmd) => {
						if !cfg!(feature = "runtime-benchmarks") {
							return Err("Runtime benchmarking wasn't enabled when building the node. \
							You can enable it with `--features runtime-benchmarks`.".into())
						}
						cmd.run_with_spec::<sp_runtime::traits::BlakeTwo256, ()>(Some(config.chain_spec))
					},
					BenchmarkCmd::Block(cmd) => {
						let PartialComponents { client, .. } = service::new_partial::<Block, RuntimeApi, HostFunctions>(&config, &cli.eth, cli.sealing)?;
						cmd.run(client)
					},
					#[cfg(not(feature = "runtime-benchmarks"))]
					BenchmarkCmd::Storage(_) => Err("Storage benchmarking can be enabled with `--features runtime-benchmarks`.".into()),
					#[cfg(feature = "runtime-benchmarks")]
					BenchmarkCmd::Storage(cmd) => {
						let enable_manual_seal = self.sealing.map(|_| true).unwrap_or_default();
						let PartialComponents { client, backend, .. } = service::new_partial::<Block, RuntimeApi, HostFunctions>(&config, &cli.eth, cli.sealing)?;
						let db = backend.expose_db();
						let storage = backend.expose_storage();
						cmd.run(config, client, db, storage)
					}
					BenchmarkCmd::Overhead(_cmd) => {
						// let PartialComponents { client, .. } = service::new_partial(&config)?;
						// let ext_builder = RemarkBuilder::new(client.clone());
						//
						// cmd.run(
						// 	config,
						// 	client,
						// 	inherent_benchmark_data()?,
						// 	Vec::new(),
						// 	&ext_builder
						// )
						Ok(())
					}
					BenchmarkCmd::Extrinsic(_cmd) => {
						// let PartialComponents { client, .. } = service::new_partial(&config)?;
						// // Register the *Remark* and *TKA* builders.
						// let ext_factory = ExtrinsicFactory(vec![
						// 	Box::new(RemarkBuilder::new(client.clone())),
						// 	Box::new(TransferKeepAliveBuilder::new(
						// 		client.clone(),
						// 		Sr25519Keyring::Alice.to_account_id(),
						// 		EXISTENTIAL_DEPOSIT
						// 	))
						// ]);
						//
						// cmd.run(client, inherent_benchmark_data()?, Vec::new(), &ext_factory)
						Ok(())
					}
					BenchmarkCmd::Machine(cmd) => cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())
				}
			})
		}
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run::<Block>(&config))
		}
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| async move {
				service::new_full::<Block, RuntimeApi, HostFunctions, sc_network::NetworkWorker<_, _>>(
					config, &cli.eth, cli.sealing
				).map_err(sc_cli::Error::Service).await
			})
		}
	}
}