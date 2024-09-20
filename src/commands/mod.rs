mod commands;
mod gargs_blockchain_provider;
mod subcmd_heir;
mod subcmd_heirwallet;
mod subcmd_service;
mod subcmd_service_heir;
mod subcmd_service_wallet;
mod subcmd_wallet;
mod subcmd_wallet_axpubs;
mod subcmd_wallet_heritage_config;
mod subcmd_wallet_ledger_policy;

use core::any::Any;
use std::{convert::Infallible, ops::Deref, path::PathBuf, str::FromStr};

use btc_heritage_wallet::bitcoin::Network;

pub trait CommandExecutor {
    fn execute(
        self,
        params: Box<dyn Any>,
    ) -> btc_heritage_wallet::errors::Result<Box<dyn crate::display::Displayable>>;
}

#[derive(Clone, Debug, clap::Args)]
pub struct CliGlobalArgs {
    /// Set the Bitcoin network on which the CLI operates.
    #[arg(
        short, long,
        env="BITCOIN_NETWORK",
        default_value_t = Network::Bitcoin,
        global = true
    )]
    pub network: Network,
    /// Use the specified directory for database storage instead of the default one.
    #[arg(
        short, long,
        default_value_t,
        value_hint = clap::ValueHint::DirPath,
        global = true
    )]
    pub datadir: DataDir,
}

#[derive(Clone, Debug, clap::Args)]
pub struct ServiceGlobalArgs {
    /// Set the URL of the Heritage service API.
    #[arg(
        long,
        value_hint = clap::ValueHint::Url,
        env="HERITAGE_SERVICE_API_URL",
        default_value = "https://api.btcherit.com/v1",
        global = true
    )]
    pub service_api_url: String,
    /// Set the URL of the Heritage service OAUTH token endpoint for the CLI.
    #[arg(
        long,
        value_hint = clap::ValueHint::Url,
        env="HERITAGE_AUTH_URL",
        default_value = "https://device.crypto7.world/token",
        global = true
    )]
    pub auth_url: String,
    /// Set the OAUTH Client Id of the CLI for the Heritage service authentication endpoint.
    #[arg(
        long,
        env = "HERITAGE_AUTH_CLIENT_ID",
        default_value = "cda6031ca00d09d66c2b632448eb8fef",
        global = true
    )]
    pub auth_client_id: String,
}

#[derive(Clone, Debug, clap::Parser)]
/// The Heritage Wallet CLI
///
/// heritage-cli manages Heritage wallets with built-in inheritance and backup access.
/// It can work with the Heritage service or locally with a custom Bitcoin or Electrum node.
#[command(author= option_env ! ("CARGO_PKG_AUTHORS").unwrap_or(""), version = option_env ! ("CARGO_PKG_VERSION").unwrap_or("unknown"), about, long_about = None)]
pub struct CliParser {
    #[clap(next_help_heading = Some("Global options"))]
    #[command(flatten)]
    pub gargs: CliGlobalArgs,
    #[clap(next_help_heading = Some("Service options"))]
    #[command(flatten)]
    pub service_gargs: ServiceGlobalArgs,
    #[clap(next_help_heading = Some("Blockchain Provider options"))]
    #[command(flatten)]
    pub blockchain_provider_gargs: gargs_blockchain_provider::BlockchainProviderGlobalArgs,
    #[command(subcommand)]
    /// Top level cli sub-commands.
    pub cmd: commands::Command,
}

impl CliParser {
    pub fn execute(
        self,
    ) -> btc_heritage_wallet::errors::Result<Box<dyn crate::display::Displayable>> {
        let cmd = self.cmd;
        let params = Box::new((
            self.gargs,
            self.service_gargs,
            self.blockchain_provider_gargs,
        ));
        cmd.execute(params)
    }
}

#[derive(Debug, Clone)]
pub struct DataDir(PathBuf);
impl Default for DataDir {
    fn default() -> Self {
        let mut home_path: PathBuf = dirs_next::home_dir().unwrap_or_default();
        home_path.push(".heritage-wallet");
        Self(home_path)
    }
}
impl ToString for DataDir {
    fn to_string(&self) -> String {
        self.0
            .to_str()
            .expect("as it comes from a string...")
            .to_owned()
    }
}
impl FromStr for DataDir {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(PathBuf::from_str(s)?))
    }
}
impl Deref for DataDir {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
