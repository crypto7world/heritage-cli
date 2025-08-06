use std::sync::Arc;

use btc_heritage_wallet::online_wallet::{AuthConfig, BlockchainProviderConfig};

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
    pub electrum_uri: Option<Arc<str>>,
    /// Set the Bitcoin Core server RPC endpoint URL to use when broadcasting a transaction or synchronizing a local wallet.
    #[arg(
        long,
        value_hint = clap::ValueHint::Url,
        requires = "auth",
        group = "provider",
        global = true
    )]
    pub bitcoincore_url: Option<Arc<str>>,
    /// Use the specified cookie-file to authenticate with Bitcoin Core.
    #[arg(
        long,
        value_hint = clap::ValueHint::FilePath,
        conflicts_with_all = ["electrum_uri", "password"],
        group = "auth",
        global = true
    )]
    pub auth_cookie: Option<Arc<str>>,
    /// Use the specified username to authenticate with Bitcoin Core.
    #[arg(
        long,
        conflicts_with = "electrum_uri",
        group = "auth",
        requires = "password",
        global = true
    )]
    pub username: Option<Arc<str>>,
    /// Use the specified password to authenticate with Bitcoin Core.
    #[arg(
        long,
        conflicts_with_all = ["electrum_uri", "auth_cookie"],
        requires = "username",
        global = true
    )]
    pub password: Option<Arc<str>>,
}

impl SerdeDisplay for BlockchainProviderConfig {}

impl TryFrom<BlockchainProviderGlobalArgs> for BlockchainProviderConfig {
    type Error = Self;

    fn try_from(value: BlockchainProviderGlobalArgs) -> Result<Self, Self::Error> {
        if let Some(url) = value.electrum_uri {
            Ok(Self::Electrum { url })
        } else if let Some(url) = value.bitcoincore_url {
            let auth = if let Some(file) = value.auth_cookie {
                AuthConfig::Cookie { file }
            } else {
                AuthConfig::UserPass {
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
