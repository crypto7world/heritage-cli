use std::{path::PathBuf, sync::Arc};

use btc_heritage_wallet::{
    bitcoin::Network,
    btc_heritage::{
        bdk_types::{Auth, ElectrumBlockchain, RpcBlockchainFactory},
        electrum_client,
    },
    online_wallet::AnyBlockchainFactory,
};
use serde::{Deserialize, Serialize};

use crate::display::SerdeDisplay;

#[derive(Clone, Debug, clap::Args)]
pub struct BlockchainProviderGlobalArgs {
    /// Set the Electrum server RPC endpoint URI to use when broadcasting a transaction or synchronizing a local wallet.
    #[arg(
        long,
        conflicts_with_all = ["auth_cookie", "username", "password"],
        group = "provider",
        global = true
    )]
    pub electrum_uri: Option<String>,
    /// Set the Bitcoin Core server RPC endpoint URL to use when broadcasting a transaction or synchronizing a local wallet.
    #[arg(
        long,
        value_hint = clap::ValueHint::Url,
        requires = "auth",
        group = "provider",
        global = true
    )]
    pub bitcoincore_url: Option<String>,
    /// Use the specified cookie-file to authenticate with Bitcoin Core.
    #[arg(
        long,
        value_hint = clap::ValueHint::FilePath,
        conflicts_with_all = ["electrum_uri", "password"],
        group = "auth",
        global = true
    )]
    pub auth_cookie: Option<PathBuf>,
    /// Use the specified username to authenticate with Bitcoin Core.
    #[arg(
        long,
        conflicts_with = "electrum_uri",
        group = "auth",
        requires = "password",
        global = true
    )]
    pub username: Option<String>,
    /// Use the specified password to authenticate with Bitcoin Core.
    #[arg(
        long,
        conflicts_with_all = ["electrum_uri", "auth_cookie"],
        requires = "username",
        global = true
    )]
    pub password: Option<String>,
}

pub struct BlockchainProviderConfigWithNetwork {
    pub bcpc: BlockchainProviderConfig,
    pub network: Network,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BlockchainProviderConfig {
    BitcoinCore { url: String, auth: Auth },
    Electrum { url: String },
}

impl SerdeDisplay for BlockchainProviderConfig {}

impl Default for BlockchainProviderConfig {
    fn default() -> Self {
        let mut file: PathBuf = dirs_next::home_dir().unwrap_or_default();
        file.push(".bitcoin/.cookie");
        Self::BitcoinCore {
            url: "http://localhost:8332".to_owned(),
            auth: Auth::Cookie { file },
        }
    }
}

impl TryFrom<BlockchainProviderGlobalArgs> for BlockchainProviderConfig {
    type Error = Self;

    fn try_from(value: BlockchainProviderGlobalArgs) -> Result<Self, Self::Error> {
        if let Some(url) = value.electrum_uri {
            Ok(Self::Electrum { url })
        } else if let Some(url) = value.bitcoincore_url {
            let auth = if let Some(file) = value.auth_cookie {
                Auth::Cookie { file }
            } else {
                Auth::UserPass {
                    username: value.username.expect("clap ensures it is present"),
                    password: value.password.expect("clap ensures it is present"),
                }
            };
            Ok(Self::BitcoinCore { url, auth })
        } else {
            Err(Self::default())
        }
    }
}

impl TryFrom<BlockchainProviderConfigWithNetwork> for AnyBlockchainFactory {
    type Error = String;

    fn try_from(value: BlockchainProviderConfigWithNetwork) -> Result<Self, Self::Error> {
        let BlockchainProviderConfigWithNetwork { bcpc, network } = value;
        Ok(match bcpc {
            BlockchainProviderConfig::BitcoinCore { url, auth } => {
                AnyBlockchainFactory::Bitcoin(RpcBlockchainFactory {
                    url,
                    auth,
                    network,
                    wallet_name_prefix: None,
                    default_skip_blocks: 0,
                    sync_params: None,
                })
            }
            BlockchainProviderConfig::Electrum { url } => {
                let config = electrum_client::ConfigBuilder::new()
                    .retry(3)
                    .timeout(Some(60))
                    .build();
                let client = electrum_client::Client::from_config(&url, config)
                    .map_err(|e| e.to_string())?;
                AnyBlockchainFactory::Electrum(Arc::new(ElectrumBlockchain::from(client)))
            }
        })
    }
}
