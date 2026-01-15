use crate::{AccountId, BalancesConfig, EVMConfig, TransactionPaymentConfig,
	RuntimeGenesisConfig, SudoConfig, ImOnlineId, ImOnlineConfig, TechnicalCommitteeConfig,
	ValidatorSetConfig, SessionConfig, AccountId32, H256, opaque::SessionKeys,
  currency::*};

#[cfg(feature = "ts-tests")]
use crate::ManualSealConfig;

use hex_literal::hex;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
#[allow(unused_imports)]
use sp_core::ecdsa;

use sp_core::{Public, Pair, H160, U256, sr25519, ed25519, crypto::Ss58Codec};
use sp_genesis_builder::PresetId;
use sp_std::prelude::*;
use sp_runtime::{MultiSignature as Signature, traits::{Verify, IdentifyAccount}};

/// Helper function to generate a crypto pair from seed
fn get_from_secret<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(seed, None).unwrap_or_else(|_| panic!("Invalid string '{}'", seed)).public()
}

/// The initial coin supply given to the endowed accounts. 
const INITIALSUPPLY: u128 = 49_999_999_900_000;
/// Mainnet node 4 has remainder of initial supply for chain development purposes only
const NODE4SUPPLY: u128 = 200_000;
//Some old docs said the nodes need a small balance to make blocks.
//I don't think so, but I'd rather not deal with it.
const VALIDATORSUPPLY: u128 = 1_000;

type AccountPublic = <Signature as Verify>::Signer;

pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>
{
	AccountPublic::from(get_from_secret::<TPublic>(seed)).into_account()
}

fn session_keys(aura: AuraId, grandpa: GrandpaId, im_online: ImOnlineId) -> SessionKeys { SessionKeys { aura, grandpa, im_online } }

//I don't know why this is required now
//PeerId 
const NONAUTHORITYNODE1AURA: &str = "5CPCGnb15jvX5919e6SJn31T4FuMs4kCUrbqJkUHQj1Wf8KU";
const NONAUTHORITYNODE1AURAHEX: &str = "0x0e1a12cfc906ba38ceec746ac1a1ef3ed3ba0fdd99f9264804101e7e01b2a574";
//const NONAUTHORITYNODE1GRANDPA: &str = "5HSJCYGTPJwmyKiPck4pw5kJ8AZckxtTAKaAD3r1doPrapsa";
const NONAUTHORITYNODE1GRANDPAHEX: &str = "0xeda4eda351aec70cced2c462c509e5f1a52afb38cd8812bb1a18e9a43f778287";
//const NONAUTHORITYNODE1H160: &str = "0xeda4eda351aec70cced2c462c509e5f1a52afb38";


///Begin Mainnet Addresses
//PeerId 12D3KooWF8V19HeCxZkqCBoeog28A2AHrgHhpuiPsveLzfsEmR7j
const MAINNETSUDONODEAURA: &str = "5HTJDxz8GCjFtaYj5HWffMYBFxhK5Sk7H9LicEATkcznYEEr";
const MAINNETSUDONODEAURAHEX: &str = "0xee6841a6e1e6b126c19df8673edbbd5967e20a22c79fbe92cf81949c11641877";
const MAINNETSUDONODEGRANDPA: &str = "5DrCUaDk4i1iLyReuoRMQ3GjPGRUDLT3vnjvpMZbheFT2tpm";
const MAINNETSUDONODEGRANDPAHEX: &str = "0x4eee9df0be0893a22410520825cc1a1a0dea0a7be08e7782c30acd70c137fbd6";
//const MAINNETSUDONODEH160: &str = "0x4eee9df0be0893a22410520825cc1a1a0dea0a7b";

//PeerId 12D3KooWGLKhTr7ohb1mWmAPK9s7ToYnUQeBJx5AegDZNTxxYudP
const MAINNETNODE1AURA: &str = "5DNz3Ki95vHZhwrb4fy742BPgxnmFj7jRZ1CQQySe7vwvhoA";
const MAINNETNODE1AURAHEX: &str = "0x3a2d2142b7d16b1ad0c9ab32aa150041f9d8257babfc289c5b9b988b4d70de50";
const MAINNETNODE1GRANDPA: &str = "5EFf4UQZWAyVsthoXSAmrYPESNPcuJVTwKaieSPNKuvsNkg9";
const MAINNETNODE1GRANDPAHEX: &str = "0x60d2c0a998720f03bb4ef9ed8201d89473fef3e1f22d380fdf4e93d47f9b7d36";
const MAINNETNODE1ACCOUNTID: &str = "0x";
const MAINNETNODE1H160: &str = "0x";

//PeerId 12D3KooWRTkmeTHs3TvucYC6CUHiGij2Ue5HyLtBt2HiHnqe3ckL
const MAINNETNODE2AURA: &str = "5HgMQRHFm7bGS3ZBALDNtwJS5whD3iBWAuAmZeVp994UcDkv";
const MAINNETNODE2AURAHEX: &str = "0xf85d26fe68c7a627fa4c1b7b80c2d4907f45e445abb58287c0abd647b6308f69";
const MAINNETNODE2GRANDPA: &str = "5HKVESAv3hQYVgqnRYbmGYruEEXAbgZ1hrJemysgN1uUXCfy";
const MAINNETNODE2GRANDPAHEX: &str = "0xe873598775fcaae94166583bdb93c1dc2f1d83a93579cafe6eb9434b9aee819d";
const MAINNETNODE2ACCOUNTID: &str = "0x";
const MAINNETNODE2H160: &str = "0x";

//PeerId 12D3KooWPngfQHdR3TtNcjATMLr8YupwtBjcxhKvwgzec1fzXYo8
const MAINNETNODE3AURA: &str = "5DSFVPUfvxMnpFKWjHEukJcPCt4ER15RpDUN1hE5e5jAee4s";
const MAINNETNODE3AURAHEX: &str = "0x3caadf99c26f79d251096ffee2aad7fab8bb08c4c59426fcbd43c1a0ab91c441";
const MAINNETNODE3GRANDPA: &str = "5Gkt96oLmHWKeK7cAvq9gfegQpcDpDs63yX8csEw9BcFc5nr";
const MAINNETNODE3GRANDPAHEX: &str = "0xcf955ddfe3874a306fc9fa4be2983e3157e24be9da722ba3477d6413ed39ab4f";
//const MAINNETNODE3H160: &str = "0xcf955ddfe3874a306fc9fa4be2983e3157e24be9";

//PeerId 12D3KooWCreP3P122JcyiczGqt4pBZqyNX7w6YcNhtRVMD7mzB74
const MAINNETNODE4AURA: &str = "5DUNUSVj3fN9iWxaqSbM7bmdecnRUu9Jxs4Uq5BwqFQbENbo";
const MAINNETNODE4AURAHEX: &str = "0x3e48df9289ff7f5fc1d7c3b61db8a61fd77d75a5b02c0b1d68fa4a5fc9ec5002";
const MAINNETNODE4GRANDPA: &str = "5D5v4ND8soXKD241Szoswcv6aBY9XbemgxJSE3DB8jVsAdxZ";
const MAINNETNODE4GRANDPAHEX: &str = "0x2d288dab0ab0edbbca2bdb741ec430b4cc6e8ba14683051e7acf16721d44bd2f";
//const MAINNETNODE4H160: &str = "0x2d288dab0ab0edbbca2bdb741ec430b4cc6e8ba1";


//PeerId 12D3KooWDcXdEcTyqePJYCKvduX6vt3qLhjQWieg5FUkPJmkJML2
const MAINNETNODE5AURA: &str = "5GuoZBbAjUQxJyjAXHiQ4yXDHFSJY6Z9fQAYuwhrvbX5NcWi";
const MAINNETNODE5AURAHEX: &str = "0xd663243eb04c28df055ff070abebcc7c4705b5a798b032521bc4ce0a91388b68";
const MAINNETNODE5GRANDPA: &str = "5DLeyjCzSB7o1DfbhZmy5hNpG2FJPyMqd4XSz5w1A8276YKA";
const MAINNETNODE5GRANDPAHEX: &str = "0x38667884513ea7b233411b2d5b3e9fef3bdce8b052ee595492af9abae441f337";
//const MAINNETNODE5H160: &str = "0x38667884513ea7b233411b2d5b3e9fef3bdce8b0";


/// Begin Testnet Addresses
//PeerId 12D3KooWDEDmKXfnpa9h4jSqBBMER2UPGA4NrRxZgdqaXY6fpxiu
const TESTNETNODE1AURA: &str = "5EP15U8PitEa8N2qrKaCrafZzyp3RsU59owdv5mWhWtSM1Mc";
const TESTNETNODE1AURAHEX: &str = "0x666cdaab93a8fb6ffba8457fe9d6a5ff9704d3c359403cbad7c9f633a714c74f";
const TESTNETNODE1GRANDPA: &str = "5DDAPHxEebdeaKUysxudLHteRkUXdmi14koJANS2DsanZyoi";
const TESTNETNODE1GRANDPAHEX: &str = "0x32af7f7772b33a64c0382ddeae52c0f33bae7df2c7f2d91a3246ca3e49cdcbd2";
//The accountid and h160 were derived with ECDSA for EVM compatibility
const TESTNETNODE1ACCOUNTID: &str = "0x24c107ea06ea45bdb3f3c264a9a2b2d0e3263672e226ba02c38c5a9b26ce1a60";
const TESTNETNODE1H160: &str = "0x24c107ea06ea45bdb3f3c264a9a2b2d0e3263672";

//PeerId 12D3KooWCreP3P122JcyiczGqt4pBZqyNX7w6YcNhtRVMD7mzB74
const TESTNETNODE2AURA: &str = "5DUNUSVj3fN9iWxaqSbM7bmdecnRUu9Jxs4Uq5BwqFQbENbo";
const TESTNETNODE2AURAHEX: &str = "0x3e48df9289ff7f5fc1d7c3b61db8a61fd77d75a5b02c0b1d68fa4a5fc9ec5002";
const TESTNETNODE2GRANDPA: &str = "5D5v4ND8soXKD241Szoswcv6aBY9XbemgxJSE3DB8jVsAdxZ";
const TESTNETNODE2GRANDPAHEX: &str = "0x2d288dab0ab0edbbca2bdb741ec430b4cc6e8ba14683051e7acf16721d44bd2f";
const TESTNETNODE2ACCOUNTID: &str = "0x888e9160b6667e64f22ac5c54baddd56eaab62dd0a1c11fdb8ee80ae36e3c57a";
const TESTNETNODE2H160: &str = "0x888e9160b6667e64f22ac5c54baddd56eaab62dd";




/// Generate a chain spec for use with the development service.
pub fn development() -> serde_json::Value {
	development_genesis(
		// Sudo account (Alith)
		get_account_id_from_seed::<sr25519::Public>("//Alice"),
		// Pre-funded accounts
		vec![
			get_account_id_from_seed::<sr25519::Public>("//Alice"),
			get_account_id_from_seed::<sr25519::Public>("//Bob"),
			get_account_id_from_seed::<sr25519::Public>("//Charlie")
		],
    // Initial authorities
		vec![(
			get_account_id_from_seed::<sr25519::Public>("//Alice"),
			get_from_secret::<AuraId>("//Alice"),
			get_from_secret::<GrandpaId>("//Alice"),
			get_from_secret::<ImOnlineId>("//Alice")
		)],
		19980,    // chain id
		false // disable manual seal
	)
}


/// Generate a chain spec for use with a local testnet.
pub fn testnet() -> serde_json::Value {
	testnet_genesis(
		// Sudo account
		array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(TESTNETNODE1ACCOUNTID),
		// Pre-funded accounts
		vec![
			array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(TESTNETNODE1ACCOUNTID),
			array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(TESTNETNODE2ACCOUNTID)
		],
		// Initial PoA authorities
		vec![
			(
				array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(TESTNETNODE1ACCOUNTID),
				sr25519::Public::from_h256(TESTNETNODE1AURAHEX.parse::<H256>().unwrap()).into(),
				ed25519::Public::from_h256(TESTNETNODE1GRANDPAHEX.parse::<H256>().unwrap()).into(),
				sr25519::Public::from_raw(<[u8; 32]>::try_from(AccountId32::from_ss58check_with_version(TESTNETNODE1AURA).unwrap().0.as_ref()).unwrap()).into()
			),
			(
				array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(TESTNETNODE2ACCOUNTID),
				sr25519::Public::from_h256(TESTNETNODE2AURAHEX.parse::<H256>().unwrap()).into(),
				ed25519::Public::from_h256(TESTNETNODE2GRANDPAHEX.parse::<H256>().unwrap()).into(),
				sr25519::Public::from_raw(<[u8; 32]>::try_from(AccountId32::from_ss58check_with_version(TESTNETNODE2AURA).unwrap().0.as_ref()).unwrap()).into()
			)
		],
		19980,    // chain id
		false // disable manual seal
	)
}



/// Generate a chain spec for use with the mainnet.
pub fn mainnet() -> serde_json::Value {
	mainnet_genesis(
		// Sudo account
		array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(MAINNETNODE1ACCOUNTID),
		// Pre-funded accounts
		vec![
			array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(MAINNETNODE1ACCOUNTID),
			array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(MAINNETNODE2ACCOUNTID)
		],
		// Initial PoA authorities
		vec![
			(
				array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(MAINNETNODE1ACCOUNTID),
				sr25519::Public::from_h256(MAINNETNODE1AURAHEX.parse::<H256>().unwrap()).into(),
				ed25519::Public::from_h256(MAINNETNODE1GRANDPAHEX.parse::<H256>().unwrap()).into(),
				sr25519::Public::from_raw(<[u8; 32]>::try_from(AccountId32::from_ss58check_with_version(MAINNETNODE1AURA).unwrap().0.as_ref()).unwrap()).into()
			),
			(
				array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(MAINNETNODE2ACCOUNTID),
				sr25519::Public::from_h256(MAINNETNODE2AURAHEX.parse::<H256>().unwrap()).into(),
				ed25519::Public::from_h256(MAINNETNODE2GRANDPAHEX.parse::<H256>().unwrap()).into(),
				sr25519::Public::from_raw(<[u8; 32]>::try_from(AccountId32::from_ss58check_with_version(MAINNETNODE2AURA).unwrap().0.as_ref()).unwrap()).into()
			)
		],
		19980,    // chain id
		false // disable manual seal
	)
}





/// Configure initial storage state for FRAME modules.
fn development_genesis(
	sudo_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	initial_authorities: Vec<(AccountId, AuraId, GrandpaId, ImOnlineId)>,
	_chain_id: u64,
	_enable_manual_seal: bool
) -> serde_json::Value {
		let non_authority_nodes: Vec<(AccountId, AuraId, GrandpaId, ImOnlineId)> = vec![
		(
			array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(NONAUTHORITYNODE1GRANDPAHEX),
			sr25519::Public::from_h256(NONAUTHORITYNODE1AURAHEX.parse::<H256>().unwrap()).into(),
			ed25519::Public::from_h256(NONAUTHORITYNODE1GRANDPAHEX.parse::<H256>().unwrap()).into(),
			sr25519::Public::from_raw(<[u8; 32]>::try_from(AccountId32::from_ss58check_with_version(NONAUTHORITYNODE1AURA).unwrap().0.as_ref()).unwrap()).into()
    )
  ];
	let num_endowed_accounts = endowed_accounts.len();

	let evm_accounts = {
		let mut map = sp_std::collections::btree_map::BTreeMap::new();
		map.insert(
			// H160 address of Alice dev account
			// Derived from SS58 (42 prefix) address
			// SS58: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
			// hex: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
			// Using the full hex key, truncating to the first 20 bytes (the first 40 hex chars)
			H160::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")),
			fp_evm::GenesisAccount {
				balance: U256::MAX,
				code: Default::default(),
				nonce: Default::default(),
				storage: Default::default()
			}
		);
		map.insert(
			// H160 address of CI test runner account
			H160::from(hex!("6be02d1d3665660d22ff9624b7be0551ee1ac91b")),
			fp_evm::GenesisAccount {
				balance: U256::MAX,
				code: Default::default(),
				nonce: Default::default(),
				storage: Default::default()
			}
		);
		map.insert(
			// H160 address for benchmark usage
			H160::from(hex!("1000000000000000000000000000000000000001")),
			fp_evm::GenesisAccount {
				nonce: U256::from(1),
				balance: U256::from(1_000_000_000_000_000_000_000_000u128),
				storage: Default::default(),
				code: vec![0x00]
			}
		);
		map
	};

	let config = RuntimeGenesisConfig {
		system: Default::default(),
		aura: Default::default(),
		base_fee: Default::default(),
		grandpa: Default::default(),
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 110)).collect(),
			..Default::default()
		},
		ethereum: Default::default(),
		evm: EVMConfig {
			accounts: evm_accounts.into_iter().collect(),
			..Default::default()
		},
		assets: Default::default(),
		council: Default::default(),
		democracy: Default::default(),
		im_online: ImOnlineConfig { keys: vec![] },
		#[cfg(feature = "ts-tests")]
		manual_seal: ManualSealConfig {
			enable: _enable_manual_seal,
			..Default::default()
		},
		sudo: SudoConfig {
			key: Some(sudo_key)
		},
		validator_set: ValidatorSetConfig{initial_validators: initial_authorities.iter().map(|x| (x.0.clone())).collect::<Vec<_>>()},
		session: SessionConfig{ 
			keys: initial_authorities.into_iter().map(|(acc, aura, gran, im_online)| 
				{ (acc.clone(), acc.clone(), session_keys(aura.clone(), gran.clone(), im_online.clone())) }).collect::<Vec<_>>(),
			non_authority_keys: non_authority_nodes.into_iter().map(|(acc, aura, gran, im_online)| 
				{ (acc.clone(), acc.clone(), session_keys(aura.clone(), gran.clone(), im_online.clone())) }).collect::<Vec<_>>()
		},
		transaction_payment: TransactionPaymentConfig{
			multiplier: 1000000000000.into(),
			_config: Default::default()
		},
		treasury: Default::default(),
		technical_committee: TechnicalCommitteeConfig {
			members: endowed_accounts.iter().take((num_endowed_accounts + 1) / 2).cloned().collect(),
			phantom: Default::default()
		}
	};

	serde_json::to_value(&config).expect("Could not build genesis config.")
}









/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	sudo_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	initial_authorities: Vec<(AccountId, AuraId, GrandpaId, ImOnlineId)>,
	_chain_id: u64,
	_enable_manual_seal: bool
) -> serde_json::Value {
		let non_authority_nodes: Vec<(AccountId, AuraId, GrandpaId, ImOnlineId)> = vec![
		(
			array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(NONAUTHORITYNODE1GRANDPAHEX),
			sr25519::Public::from_h256(NONAUTHORITYNODE1AURAHEX.parse::<H256>().unwrap()).into(),
			ed25519::Public::from_h256(NONAUTHORITYNODE1GRANDPAHEX.parse::<H256>().unwrap()).into(),
			sr25519::Public::from_raw(<[u8; 32]>::try_from(AccountId32::from_ss58check_with_version(NONAUTHORITYNODE1AURA).unwrap().0.as_ref()).unwrap()).into()
    )
  ];
	let num_endowed_accounts = endowed_accounts.len();

	let evm_accounts = {
		let mut map = sp_std::collections::btree_map::BTreeMap::new();
		map.insert(
			// H160 address of testnetnode1's account id.
			// Derived from SS58 (42 prefix) address
			// hex: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
			// Using the full hex key, truncating to the first 20 bytes (the first 40 hex chars)
			H160::from(hex!("24c107ea06ea45bdb3f3c264a9a2b2d0e3263672")),
			fp_evm::GenesisAccount {
				balance: (OSLO * INITIALSUPPLY).into(),
				code: Default::default(),
				nonce: Default::default(),
				storage: Default::default()
			}
		);
		map.insert(
			// H160 address of testnetnode2's account id.
			H160::from(hex!("888e9160b6667e64f22ac5c54baddd56eaab62dd")),
			fp_evm::GenesisAccount {
				balance: (OSLO * INITIALSUPPLY).into(),
				code: Default::default(),
				nonce: Default::default(),
				storage: Default::default()
			}
		);
		map
	};

	let config = RuntimeGenesisConfig {
		system: Default::default(),
		aura: Default::default(),
		base_fee: Default::default(),
		grandpa: Default::default(),
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 110)).collect(),
			..Default::default()
		},
		ethereum: Default::default(),
		evm: EVMConfig {
			accounts: evm_accounts.into_iter().collect(),
			..Default::default()
		},
		assets: Default::default(),
		council: Default::default(),
		democracy: Default::default(),
		im_online: ImOnlineConfig { keys: vec![] },
		sudo: SudoConfig {
			key: Some(sudo_key)
		},
		validator_set: ValidatorSetConfig{initial_validators: initial_authorities.iter().map(|x| (x.0.clone())).collect::<Vec<_>>()},
		session: SessionConfig{ 
			keys: initial_authorities.into_iter().map(|(acc, aura, gran, im_online)| 
				{ (acc.clone(), acc.clone(), session_keys(aura.clone(), gran.clone(), im_online.clone())) }).collect::<Vec<_>>(),
			non_authority_keys: non_authority_nodes.into_iter().map(|(acc, aura, gran, im_online)| 
				{ (acc.clone(), acc.clone(), session_keys(aura.clone(), gran.clone(), im_online.clone())) }).collect::<Vec<_>>()
		},
		#[cfg(feature = "ts-tests")]
		manual_seal: ManualSealConfig {
			enable: false,
			..Default::default()
		},
		transaction_payment: TransactionPaymentConfig{
			multiplier: 1000000000000.into(),
			_config: Default::default()
		},
		treasury: Default::default(),
		technical_committee: TechnicalCommitteeConfig {
			members: endowed_accounts.iter().take((num_endowed_accounts + 1) / 2).cloned().collect(),
			phantom: Default::default()
		}
	};

	serde_json::to_value(&config).expect("Could not build genesis config.")
}




/// Configure initial storage state for FRAME modules.
fn mainnet_genesis(
	sudo_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	initial_authorities: Vec<(AccountId, AuraId, GrandpaId, ImOnlineId)>,
	_chain_id: u64,
	_enable_manual_seal: bool
) -> serde_json::Value {
		let non_authority_nodes: Vec<(AccountId, AuraId, GrandpaId, ImOnlineId)> = vec![
		(
			array_bytes::hex_n_into_unchecked::<&str, sp_runtime::AccountId32, 32>(NONAUTHORITYNODE1GRANDPAHEX),
			sr25519::Public::from_h256(NONAUTHORITYNODE1AURAHEX.parse::<H256>().unwrap()).into(),
			ed25519::Public::from_h256(NONAUTHORITYNODE1GRANDPAHEX.parse::<H256>().unwrap()).into(),
			sr25519::Public::from_raw(<[u8; 32]>::try_from(AccountId32::from_ss58check_with_version(NONAUTHORITYNODE1AURA).unwrap().0.as_ref()).unwrap()).into()
    )
  ];
	let num_endowed_accounts = endowed_accounts.len();

	let evm_accounts = {
		let mut map = sp_std::collections::btree_map::BTreeMap::new();
		map.insert(
			// H160 address of testnetnode1's account id.
			// Derived from SS58 (42 prefix) address
			// hex: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
			// Using the full hex key, truncating to the first 20 bytes (the first 40 hex chars)
			H160::from(hex!("24c107ea06ea45bdb3f3c264a9a2b2d0e3263672")),
			fp_evm::GenesisAccount {
				balance: (OSLO * INITIALSUPPLY).into(),
				code: Default::default(),
				nonce: Default::default(),
				storage: Default::default()
			}
		);
		map.insert(
			// H160 address of testnetnode2's account id.
			H160::from(hex!("888e9160b6667e64f22ac5c54baddd56eaab62dd")),
			fp_evm::GenesisAccount {
				balance: (OSLO * INITIALSUPPLY).into(),
				code: Default::default(),
				nonce: Default::default(),
				storage: Default::default()
			}
		);
		map
	};

	let config = RuntimeGenesisConfig {
		system: Default::default(),
		aura: Default::default(),
		base_fee: Default::default(),
		grandpa: Default::default(),
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 110)).collect(),
			..Default::default()
		},
		ethereum: Default::default(),
		evm: EVMConfig {
			accounts: evm_accounts.into_iter().collect(),
			..Default::default()
		},
		assets: Default::default(),
		council: Default::default(),
		democracy: Default::default(),
		im_online: ImOnlineConfig { keys: vec![] },
		sudo: SudoConfig {
			key: Some(sudo_key)
		},
		validator_set: ValidatorSetConfig{initial_validators: initial_authorities.iter().map(|x| (x.0.clone())).collect::<Vec<_>>()},
		session: SessionConfig{ 
			keys: initial_authorities.into_iter().map(|(acc, aura, gran, im_online)| 
				{ (acc.clone(), acc.clone(), session_keys(aura.clone(), gran.clone(), im_online.clone())) }).collect::<Vec<_>>(),
			non_authority_keys: non_authority_nodes.into_iter().map(|(acc, aura, gran, im_online)| 
				{ (acc.clone(), acc.clone(), session_keys(aura.clone(), gran.clone(), im_online.clone())) }).collect::<Vec<_>>()
		},
		#[cfg(feature = "ts-tests")]
		manual_seal: ManualSealConfig {
			enable: false,
			..Default::default()
		},
		transaction_payment: TransactionPaymentConfig{
			multiplier: 1000000000000.into(),
			_config: Default::default()
		},
		treasury: Default::default(),
		technical_committee: TechnicalCommitteeConfig {
			members: endowed_accounts.iter().take((num_endowed_accounts + 1) / 2).cloned().collect(),
			phantom: Default::default()
		}
	};

	serde_json::to_value(&config).expect("Could not build genesis config.")
}




const MAINNET_PRESET: &'static str = "live"; 
/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	let patch = match id.as_str() {
		sp_genesis_builder::DEV_RUNTIME_PRESET => development(),
		sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => testnet(),
		MAINNET_PRESET => mainnet(),
		//It's probably best if this starts syncing the mainnet when a new user
		//runs the node without any arguments
		_ => mainnet()
	};
	Some(serde_json::to_string(&patch).expect("serialization to json is expected to work. qed.").into_bytes())
}