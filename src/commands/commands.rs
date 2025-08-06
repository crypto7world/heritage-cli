use core::any::Any;

use btc_heritage_wallet::{
    bitcoin::psbt::Psbt, btc_heritage::utils::bitcoin_network,
    heritage_service_api_client::HeritageServiceConfig, online_wallet::BlockchainProviderConfig,
    Database, DatabaseItem, DatabaseSingleItem, Heir, HeirWallet, PsbtSummary, Wallet,
};

use crate::utils::get_fingerprints;

use super::CommandExecutor;

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
        /// Set the default values using the current Blockchain Provider configuration options instead of just displaying them
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
        let (mut db, name, hsc, bcpc): (
            Database,
            String,
            HeritageServiceConfig,
            BlockchainProviderConfig,
        ) = *params.downcast().unwrap();

        match self {
            ListAndDefault::List => {
                let wallet_names = I::list_names(&db)?;
                Ok(Box::new(wallet_names))
            }
            ListAndDefault::DefaultName { new_name } => {
                if let Some(new_name) = new_name {
                    I::set_default_item_name(&mut db, new_name)?;
                }
                Ok(Box::new(I::get_default_item_name(&db)?))
            }
            ListAndDefault::Others(sub) => {
                let params = Box::new((db, name, hsc, bcpc));
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
            super::gargs_heritage_service::HeritageServiceGlobalArgs,
            super::gargs_blockchain_provider::BlockchainProviderGlobalArgs,
        ) = *params.downcast().unwrap();

        bitcoin_network::set(gargs.network);
        let mut db = Database::new(&gargs.datadir, gargs.network)?;

        let bcpc = match BlockchainProviderConfig::try_from(blockchain_provider_gargs) {
            Ok(bcpc) => bcpc,
            Err(bcpc) => BlockchainProviderConfig::load(&db).unwrap_or(bcpc),
        };

        let mut hsc = HeritageServiceConfig::load(&db).unwrap_or_default();
        if let Some(service_api_url) = service_gargs.service_api_url {
            hsc.service_api_url = service_api_url;
        }
        if let Some(auth_url) = service_gargs.auth_url {
            hsc.auth_url = auth_url;
        }
        if let Some(auth_client_id) = service_gargs.auth_client_id {
            hsc.auth_client_id = auth_client_id;
        }

        match self {
            Command::Wallet {
                wallet_name,
                subcmd,
            } => {
                let wallet_name = match wallet_name {
                    Some(wn) => wn,
                    None => Wallet::get_default_item_name(&db)?,
                };
                let params = Box::new((db, wallet_name, hsc, bcpc));
                subcmd.execute(params).await
            }
            Command::Heir { heir_name, subcmd } => {
                let heir_name = match heir_name {
                    Some(wn) => wn,
                    None => Heir::get_default_item_name(&db)?,
                };
                let params = Box::new((db, heir_name, hsc, bcpc));
                subcmd.execute(params).await
            }
            Command::HeirWallet {
                heir_wallet_name,
                subcmd,
            } => {
                let heir_wallet_name = match heir_wallet_name {
                    Some(wn) => wn,
                    None => HeirWallet::get_default_item_name(&db)?,
                };
                let params = Box::new((db, heir_wallet_name, hsc, bcpc));
                subcmd.execute(params).await
            }
            Command::Service { subcmd } => {
                let params = Box::new((db, hsc));
                subcmd.execute(params).await
            }
            Command::BlockchainProvider { set } => {
                if set {
                    bcpc.save(&mut db)?;
                }
                Ok(Box::new(bcpc))
            }
            Command::DisplayPsbt { psbt } => {
                let summary = PsbtSummary::try_from((
                    &psbt,
                    &get_fingerprints(&db).await?,
                    bitcoin_network::get(),
                ))?;
                Ok(Box::new(summary))
            }
        }
    }
}
