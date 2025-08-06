mod commands;
mod gargs_blockchain_provider;
mod gargs_heritage_service;
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
        params: Box<dyn Any + Send>,
    ) -> impl std::future::Future<
        Output = btc_heritage_wallet::errors::Result<Box<dyn crate::display::Displayable>>,
    > + Send;
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
    #[clap(next_help_heading = Some("Blockchain Provider Configuration"))]
    #[command(flatten)]
    pub blockchain_provider_gargs: gargs_blockchain_provider::BlockchainProviderGlobalArgs,
    #[clap(next_help_heading = Some("Heritage Service Configuration"))]
    #[command(flatten)]
    pub service_gargs: gargs_heritage_service::HeritageServiceGlobalArgs,
    #[command(subcommand)]
    /// Top level cli sub-commands.
    pub cmd: commands::Command,
}

impl CliParser {
    pub async fn execute(
        self,
    ) -> btc_heritage_wallet::errors::Result<Box<dyn crate::display::Displayable>> {
        let cmd = self.cmd;
        let params = Box::new((
            self.gargs,
            self.service_gargs,
            self.blockchain_provider_gargs,
        ));
        cmd.execute(params).await
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
