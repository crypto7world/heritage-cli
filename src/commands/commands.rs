use core::any::Any;

use btc_heritage_wallet::{
    bitcoin::psbt::Psbt, Database, DatabaseItem, Heir, HeirWallet, PsbtSummary, Wallet,
};

use crate::utils::get_fingerprints;

use super::{gargs_blockchain_provider::BlockchainProviderConfig, CommandExecutor};

/// Top level cli sub-commands.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum Command {
    /// Commands managing wallets, use this to create and manage Heritage wallets.
    #[command(visible_aliases = ["wallets", "w"])]
    Wallet {
        /// The name of the wallet to operate.
        /// Defaults to "default" or any other name you set with the "default-name" command
        wallet_name: Option<String>,
        #[command(subcommand)]
        subcmd: ListAndDefault<super::subcmd_wallet::WalletSubcmd, Wallet>,
    },
    /// Commands managing heirs, use this to create or declare heirs for your Heritage wallet
    #[command(visible_aliases = ["heirs", "h"])]
    Heir {
        /// The name of the heir to operate.
        /// Defaults to "default" or any other name you set with the "default-name" command
        heir_name: Option<String>,
        #[command(subcommand)]
        subcmd: ListAndDefault<super::subcmd_heir::HeirSubcmd, Heir>,
    },
    /// Commands managing heir-wallets, restricted wallets used only to list and claim inheritances
    /// {n}Use this if you are an heir and just want to claim an inheritance.
    #[command(visible_aliases = ["heir-wallets", "hw"])]
    HeirWallet {
        /// The name of the heir-wallet to operate.
        /// Defaults to "default" or any other name you set with the "default-name" command
        heir_wallet_name: Option<String>,
        #[command(subcommand)]
        subcmd: ListAndDefault<super::subcmd_heirwallet::HeirWalletSubcmd, HeirWallet>,
    },
    /// Commands related to the Heritage service, mainly used to authenticate the CLI with the service.
    #[command(visible_aliases = ["svc"])]
    Service {
        #[command(subcommand)]
        subcmd: super::subcmd_service::ServiceSubcmd,
    },
    /// Show or set the default blockchain provider to use when synchronizing or broadcasting from a local wallet.
    #[command(visible_aliases = ["bp", "blockchain"], aliases = ["default-blockchain", "default-blockchain-provider"])]
    BlockchainProvider {
        /// Set the default values using the current Blockchain Provider options instead of just displaying them
        #[arg(long, default_value_t = false)]
        set: bool,
    },
    /// Display infos on the given Partially Signed Bitcoin Transaction (PSBT)
    #[command(visible_alias = "display")]
    DisplayPsbt {
        /// The PSBT
        psbt: Psbt,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum ListAndDefault<
    T: Clone + core::fmt::Debug + clap::Subcommand + CommandExecutor + Send,
    I: DatabaseItem + Send,
> {
    /// List all items in the database
    List,
    /// Display or set the default name
    DefaultName {
        /// Set the default name instead of simply displaying it
        #[arg(short = 's', long = "set")]
        new_name: Option<String>,
    },
    #[command(flatten)]
    Others(T),
    #[command(skip)]
    _Impossible {
        _i: core::convert::Infallible,
        _p: core::marker::PhantomData<I>,
    },
}
impl<
        T: Clone + core::fmt::Debug + clap::Subcommand + CommandExecutor + Send,
        I: DatabaseItem + Send,
    > Clone for ListAndDefault<T, I>
{
    fn clone(&self) -> Self {
        match self {
            Self::List => Self::List,
            Self::DefaultName { new_name } => Self::DefaultName {
                new_name: new_name.clone(),
            },
            Self::Others(arg0) => Self::Others(arg0.clone()),
            Self::_Impossible { .. } => unreachable!(),
        }
    }
}

impl<
        T: Clone + core::fmt::Debug + clap::Subcommand + CommandExecutor + Send,
        I: DatabaseItem + Send,
    > super::CommandExecutor for ListAndDefault<T, I>
{
    async fn execute(
        self,
        params: Box<dyn Any + Send>,
    ) -> btc_heritage_wallet::errors::Result<Box<dyn crate::display::Displayable>> {
        let (mut db, name, gargs, service_gargs, bcpc): (
            Database,
            String,
            super::CliGlobalArgs,
            super::ServiceGlobalArgs,
            super::gargs_blockchain_provider::BlockchainProviderConfig,
        ) = *params.downcast().unwrap();

        match self {
            ListAndDefault::List => {
                let wallet_names = I::list_names(&db).await?;
                Ok(Box::new(wallet_names))
            }
            ListAndDefault::DefaultName { new_name } => {
                if let Some(new_name) = new_name {
                    I::set_default_item_name(&mut db, new_name).await?;
                }
                Ok(Box::new(I::get_default_item_name(&db).await?))
            }
            ListAndDefault::Others(sub) => {
                let params = Box::new((db, name, gargs, service_gargs, bcpc));
                sub.execute(params).await
            }
            ListAndDefault::_Impossible { .. } => unreachable!(),
        }
    }
}

impl super::CommandExecutor for Command {
    async fn execute(
        self,
        params: Box<dyn Any + Send>,
    ) -> btc_heritage_wallet::errors::Result<Box<dyn crate::display::Displayable>> {
        let (gargs, service_gargs, blockchain_provider_gargs): (
            super::CliGlobalArgs,
            super::ServiceGlobalArgs,
            super::gargs_blockchain_provider::BlockchainProviderGlobalArgs,
        ) = *params.downcast().unwrap();
        let mut db = Database::new(&gargs.datadir, gargs.network).await?;
        const DEFAULT_BCPC_KEY: &'static str = "default_bcpc";
        let bcpc = match BlockchainProviderConfig::try_from(blockchain_provider_gargs) {
            Ok(bcpc) => bcpc,
            Err(bcpc) => db.get_item(DEFAULT_BCPC_KEY).await?.unwrap_or(bcpc),
        };
        match self {
            Command::Wallet {
                wallet_name,
                subcmd,
            } => {
                let wallet_name = match wallet_name {
                    Some(wn) => wn,
                    None => Wallet::get_default_item_name(&db).await?,
                };
                let params = Box::new((db, wallet_name, gargs, service_gargs, bcpc));
                subcmd.execute(params).await
            }
            Command::Heir { heir_name, subcmd } => {
                let heir_name = match heir_name {
                    Some(wn) => wn,
                    None => Heir::get_default_item_name(&db).await?,
                };
                let params = Box::new((db, heir_name, gargs, service_gargs, bcpc));
                subcmd.execute(params).await
            }
            Command::HeirWallet {
                heir_wallet_name,
                subcmd,
            } => {
                let heir_wallet_name = match heir_wallet_name {
                    Some(wn) => wn,
                    None => HeirWallet::get_default_item_name(&db).await?,
                };
                let params = Box::new((db, heir_wallet_name, gargs, service_gargs, bcpc));
                subcmd.execute(params).await
            }
            Command::Service { subcmd } => {
                let params = Box::new((db, service_gargs));
                subcmd.execute(params).await
            }
            Command::BlockchainProvider { set } => {
                if set {
                    db.update_item(DEFAULT_BCPC_KEY, &bcpc).await?;
                }
                Ok(Box::new(bcpc))
            }
            Command::DisplayPsbt { psbt } => {
                let network = gargs.network;
                let summary =
                    PsbtSummary::try_from((&psbt, &get_fingerprints(&db).await?, network))?;
                Ok(Box::new(summary))
            }
        }
    }
}
