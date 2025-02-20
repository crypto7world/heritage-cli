use core::any::Any;
use std::{collections::HashMap, path::PathBuf};

use btc_heritage_wallet::{
    bitcoin::{address::NetworkUnchecked, psbt::Psbt, Address},
    btc_heritage::{utils::timestamp_now, HeritageWalletBackup},
    errors::{Error, Result},
    heritage_provider::{LocalWallet, ServiceBinding},
    heritage_service_api_client::{Fingerprint, HeritageServiceClient, Tokens},
    AnyHeritageProvider, AnyKeyProvider, BoundFingerprint, Database, DatabaseItem, HeirWallet,
    Heritage, HeritageProvider, KeyProvider, Language, LocalKey, Mnemonic, OnlineWallet,
};
use chrono::{DateTime, Utc};
use clap::builder::{PossibleValuesParser, TypedValueParser};

use crate::{
    commands::subcmd_heir::HeirConfigType,
    spendflow::SpendFlow,
    utils::{ask_user_confirmation, get_fingerprints, prompt_user_for_password},
};

use super::{
    gargs_blockchain_provider::BlockchainProviderConfigWithNetwork, subcmd_wallet::KeyProviderType,
};

#[derive(Debug, serde::Serialize)]
pub struct Inheritance {
    inheritance_id: String,
    #[serde(serialize_with = "crate::utils::serialize_amount")]
    value: btc_heritage_wallet::bitcoin::Amount,
    /// The timestamp after which the Heir is able to spend
    #[serde(serialize_with = "crate::utils::serialize_datetime")]
    maturity: DateTime<Utc>,
    /// The maturity of the next heir, if any
    #[serde(serialize_with = "crate::utils::serialize_opt_datetime")]
    next_heir_maturity: Option<DateTime<Utc>>,
}
impl crate::display::SerdeDisplay for Inheritance {}
impl From<Heritage> for Inheritance {
    fn from(value: Heritage) -> Self {
        let Heritage {
            heritage_id,
            value,
            maturity,
            next_heir_maturity,
        } = value;
        Inheritance {
            inheritance_id: heritage_id,
            value,
            maturity: DateTime::from_timestamp(maturity as i64, 0)
                .expect("maturiy timestamp is in range"),
            next_heir_maturity: next_heir_maturity.map(|ts| {
                DateTime::from_timestamp(ts as i64, 0).expect("maturiy timestamp is in range")
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum HeritageProviderType {
    /// No heritage provider, the resulting wallet will not be able to list heritages (it will be sign-only)
    None,
    /// Use the Heritage service as the online wallet
    Service,
    /// Use a local wallet restored from descriptors backup
    Local,
}

/// Sub-command for heir-wallets.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum HeirWalletSubcmd {
    /// Creates a new heir-wallet with the chosen heritage-provider and key-provider
    Create {
        /// The fingerprint to look for when listing or spending Heritages (required if --key-provider none)
        #[arg(long)]
        fingerprint: Option<Fingerprint>,
        /// Provide a descriptors backup to restore for the local heritage-provider
        #[arg(
            long,
            value_name = "BACKUP",
            value_parser=crate::utils::parse_heritage_wallet_backup,
            group = "restore_backup",
        )]
        backup: Option<HeritageWalletBackup>,
        /// Provide the path to a descriptors backup to restore for the local heritage-provider
        #[arg(
            long,
            value_name = "PATH",
            value_hint = clap::ValueHint::FilePath,
            group = "restore_backup",
        )]
        backup_file: Option<PathBuf>,
        /// The kind of heritage-provider to use to list and spend Heritages
        #[arg(short = 'p',long, value_name = "TYPE", aliases = ["hp"], value_enum, default_value_t=HeritageProviderType::Service, requires_if("local", "restore_backup"))]
        heritage_provider: HeritageProviderType,
        /// Specify the kind of key-provider the wallet will use to manages secrets keys and sign transactions
        #[arg(
            short = 'k',long, value_name = "TYPE", aliases = ["kp"], value_enum, default_value_t=KeyProviderType::Local,
            requires_ifs=[("local", "localgen"), ("none", "fingerprint")]
        )]
        key_provider: KeyProviderType,
        /// The mnemonic phrase to restore as a seed for the local key-provider (12, 18 or 24 words).
        #[arg(long, value_name = "WORD", num_args=2..=24, group="localgen", required_unless_present_any=["key_provider", "word_count"])]
        seed: Option<Vec<String>>,
        /// The length of the mnemonic phrase to generate as a seed for the local key-provider.
        #[arg(
            long, value_parser=PossibleValuesParser::new(["12", "18", "24"]).map(|s| s.parse::<usize>().unwrap()),
            group="localgen", required_unless_present_any=["key_provider", "seed"]
        )]
        word_count: Option<usize>,
        /// Signal that the seed of the local key-provider should be password-protected.
        #[arg(long, default_value_t = false)]
        with_password: bool,
    },
    /// Rename the heir-wallet in the database to a new name
    Rename { new_name: String },
    /// Remove the heir-wallet from the database
    /// {n}/!\ BE AWARE THAT YOU WILL NOT BE ABLE TO RETRIEVE THE SEED IF IT IS NOT BACKED-UP /!\
    #[command(visible_aliases = ["delete", "del"])]
    Remove {
        #[arg(long)]
        /// Confirm that you know what you are doing and skips verification prompts
        i_understand_what_i_am_doing: bool,
    },
    /// Display the fingerprint of the heir-wallet
    Fingerprint,
    /// Display the mnemonic of the heir-wallet for backup purposes
    #[command(visible_alias = "backup-mnemonic")]
    Mnemonic,
    /// Generate an Heir Configuration from this heir-wallet that can be used as an heir for an Heritage wallet
    HeirConfig {
        /// The kind of Heir Configuration to generate
        #[arg(short, long, value_enum, default_value_t=HeirConfigType::Xpub)]
        kind: HeirConfigType,
    },
    /// Sync the local heritage-provider from the Bitcoin network
    Sync,
    /// Display all currently spendable inheritances and their IDs
    #[command(visible_aliases = ["list-inheritance", "list-heritages", "list-heritage", "li"])]
    ListInheritances {
        /// Also take into account immature UTXO when computing the inheritance amount (the maturity date will
        /// probably be wrong, use `--details` to retrieve complete informations)
        #[arg(long, default_value_t = false)]
        immatures: bool,
        /// Do not aggregate and display each UTXO as a standalone inheritance (will duplicate IDs)
        #[arg(long, default_value_t = false)]
        details: bool,
    },
    /// Create a Partially Signed Bitcoin Transaction (PSBT), a.k.a an Unsigned TX, from the provided information
    #[command(visible_aliases = ["send-inheritance", "send-heritage", "spend-heritage", "send", "spend", "si"])]
    SpendInheritance {
        /// The inheritance ID to spend
        #[arg(short, long, value_name = "INHERITANCE_ID", required = true)]
        id: String,
        /// A recipient address to which send all Heritages with the ID (see the `list-heritages` command).
        #[arg(short, long, value_name = "ADDRESS", required = true)]
        recipient: Address<NetworkUnchecked>,
        /// Immediately sign the PSBT
        #[arg(short, long, default_value_t = false)]
        sign: bool,
        /// Immediately broadcast the PSBT after signing it
        #[arg(short, long, default_value_t = false, requires = "sign")]
        broadcast: bool,
        /// If --sign or --broadcast are requested, do it without asking for confirmation{n}
        /// /!\ BE VERY CAREFULL with that option /!\.
        #[arg(short = 'y', long, default_value_t = false)]
        skip_confirmation: bool,
    },
    /// Sign every sign-able inputs of the given Partially Signed Bitcoin Transaction (PSBT)
    #[command(visible_alias = "sign")]
    SignPsbt {
        /// The PSBT to sign
        psbt: Psbt,
        /// Immediately broadcast the PSBT after signing it
        #[arg(short, long, default_value_t = false)]
        broadcast: bool,
        /// If --broadcast is requested, do it without asking for confirmation{n}
        /// /!\ BE VERY CAREFULL with that option /!\.
        #[arg(short = 'y', long, default_value_t = false)]
        skip_confirmation: bool,
    },
    /// Extract a raw transaction from the given Partially Signed Bitcoin Transaction (PSBT) and broadcast it to the Bitcoin network
    #[command(visible_alias = "broadcast")]
    BroadcastPsbt {
        /// The PSBT to broadcast. Must have every inputs correctly signed for this to work.
        psbt: Psbt,
    },
}

impl super::CommandExecutor for HeirWalletSubcmd {
    async fn execute(
        mut self,
        params: Box<dyn Any + Send>,
    ) -> Result<Box<dyn crate::display::Displayable>> {
        let (mut db, heir_wallet_name, gargs, service_gargs, bcpc): (
            Database,
            String,
            super::CliGlobalArgs,
            super::ServiceGlobalArgs,
            super::gargs_blockchain_provider::BlockchainProviderConfig,
        ) = *params.downcast().unwrap();

        let service_client = HeritageServiceClient::new(
            service_gargs.service_api_url.clone(),
            Tokens::load(&mut db).await?,
        );

        let need_heritage_provider = match &self {
            HeirWalletSubcmd::Create { .. }
            | HeirWalletSubcmd::Sync
            | HeirWalletSubcmd::ListInheritances { .. }
            | HeirWalletSubcmd::SpendInheritance { .. }
            | HeirWalletSubcmd::BroadcastPsbt { .. } => true,
            HeirWalletSubcmd::SignPsbt { broadcast, .. } if *broadcast => true,
            HeirWalletSubcmd::Rename { .. }
            | HeirWalletSubcmd::Remove { .. }
            | HeirWalletSubcmd::Fingerprint
            | HeirWalletSubcmd::Mnemonic
            | HeirWalletSubcmd::SignPsbt { .. }
            | HeirWalletSubcmd::HeirConfig { .. } => false,
        };
        let need_key_provider = match &self {
            HeirWalletSubcmd::Create { .. }
            | HeirWalletSubcmd::SignPsbt { .. }
            | HeirWalletSubcmd::HeirConfig { .. }
            | HeirWalletSubcmd::Mnemonic { .. } => true,
            HeirWalletSubcmd::SpendInheritance { sign, .. } if *sign => true,
            HeirWalletSubcmd::Rename { .. }
            | HeirWalletSubcmd::Sync
            | HeirWalletSubcmd::SpendInheritance { .. }
            | HeirWalletSubcmd::Remove { .. }
            | HeirWalletSubcmd::Fingerprint
            | HeirWalletSubcmd::ListInheritances { .. }
            | HeirWalletSubcmd::BroadcastPsbt { .. } => false,
        };
        let need_blockchain_provider = match &self {
            HeirWalletSubcmd::SpendInheritance { broadcast, .. } if *broadcast => true,
            HeirWalletSubcmd::SignPsbt { broadcast, .. } if *broadcast => true,
            HeirWalletSubcmd::BroadcastPsbt { .. } | HeirWalletSubcmd::Sync => true,
            _ => false,
        };

        let heir = match &mut self {
            HeirWalletSubcmd::Create {
                fingerprint,
                backup,
                backup_file,
                heritage_provider,
                key_provider,
                seed,
                word_count,
                with_password,
            } => {
                HeirWallet::verify_name_is_free(&db, &heir_wallet_name).await?;
                let key_provider = match key_provider {
                    KeyProviderType::None => AnyKeyProvider::None,
                    KeyProviderType::Local => {
                        let password = if *with_password {
                            Some(prompt_user_for_password(true)?)
                        } else {
                            None
                        };
                        let local_key = if let Some(seed) = seed {
                            log::info!("Restoring an heir...");
                            let mnemo = Mnemonic::parse_in(Language::English, seed.join(" "))
                                .map_err(|e| {
                                    log::error!("invalid mnemonic {e}");
                                    Error::Generic(format!("invalid mnemonic {e}"))
                                })?;
                            LocalKey::restore(mnemo, password, gargs.network)
                        } else if let Some(word_count) = word_count {
                            log::info!("Generating a new heir...");
                            LocalKey::generate(*word_count, password, gargs.network)
                        } else {
                            unreachable!("Clap ensure either seed or word_count is passed");
                        };
                        AnyKeyProvider::LocalKey(local_key)
                    }
                    KeyProviderType::Ledger => todo!(),
                };
                let fingerprint = if !key_provider.is_none() {
                    key_provider.fingerprint()?
                } else if let Some(fingerprint) = fingerprint {
                    *fingerprint
                } else {
                    unreachable!("clap ensures it")
                };

                let heritage_provider = match heritage_provider {
                    HeritageProviderType::None => AnyHeritageProvider::None,
                    HeritageProviderType::Service => AnyHeritageProvider::Service(
                        ServiceBinding::new(fingerprint, service_client),
                    ),
                    HeritageProviderType::Local => {
                        let backup = if let Some(backup_file) = backup_file {
                            Some(crate::utils::parse_heritage_wallet_backup(
                                &std::fs::read_to_string(backup_file.as_path())
                                    .map_err(Error::generic)?,
                            )?)
                        } else {
                            backup.take()
                        };
                        AnyHeritageProvider::LocalWallet(
                            LocalWallet::create(
                                fingerprint,
                                &db,
                                backup.expect("clap ensure it is present"),
                            )
                            .await?,
                        )
                    }
                };
                let heir = HeirWallet::new(heir_wallet_name, key_provider, heritage_provider)?;
                // let heir = RefCell::new(heir);

                heir
            }
            _ => {
                let mut heir = HeirWallet::load(&db, &heir_wallet_name).await?;
                if need_key_provider {
                    match heir.key_provider_mut() {
                        AnyKeyProvider::None => (),
                        AnyKeyProvider::LocalKey(lk) => {
                            let password = if lk.require_password() {
                                Some(prompt_user_for_password(false)?)
                            } else {
                                None
                            };
                            lk.init_local_key(password)?;
                        }
                        AnyKeyProvider::Ledger(ledger) => ledger.init_ledger_client().await?,
                    };
                }
                if need_heritage_provider {
                    match heir.heritage_provider_mut() {
                        AnyHeritageProvider::None => (),
                        AnyHeritageProvider::Service(sb) => sb.init_service_client(service_client),
                        AnyHeritageProvider::LocalWallet(lw) => {
                            lw.local_heritage_wallet_mut()
                                .init_heritage_wallet(&db)
                                .await?;
                            if need_blockchain_provider {
                                let bcpc_with_network = BlockchainProviderConfigWithNetwork {
                                    bcpc,
                                    network: gargs.network,
                                };
                                lw.local_heritage_wallet_mut().init_blockchain_factory(
                                    bcpc_with_network.try_into().map_err(Error::generic)?,
                                )?
                            }
                        }
                    };
                }
                // RefCell::new(heir)
                heir
            }
        };

        let res: Box<dyn crate::display::Displayable> = match self {
            HeirWalletSubcmd::Create { .. } => {
                heir.create(&mut db).await?;
                Box::new("Heir wallet created")
            }
            HeirWalletSubcmd::Rename { new_name } => {
                // First verify the destination name is free
                HeirWallet::verify_name_is_free(&db, &new_name).await?;
                // Rename
                let mut heir = heir;
                heir.db_rename(&mut db, new_name).await?;
                Box::new("Heir wallet renamed")
            }
            HeirWalletSubcmd::Remove {
                i_understand_what_i_am_doing,
            } => {
                if !i_understand_what_i_am_doing {
                    if !heir.key_provider().is_none() {
                        if !ask_user_confirmation(&format!(
                            "Do you have a backup of the seed of the heir-wallet \"{}\"?",
                            heir.name()
                        ))
                        .await?
                        {
                            return Ok(Box::new("Delete heir-wallet cancelled"));
                        }
                    }
                    if !ask_user_confirmation(&format!(
                        "FINAL CONFIRMATION. Are you SURE you want to delete the heir-wallet \"{}\"?",
                        heir.name()
                    )).await?{
                        return Ok(Box::new("Delete heir-wallet cancelled"));
                    }
                }
                heir.delete(&mut db).await?;
                Box::new("Heir wallet deleted")
            }
            HeirWalletSubcmd::Fingerprint => Box::new(heir.fingerprint()?),
            HeirWalletSubcmd::Mnemonic => Box::new(heir.backup_mnemonic().await?),
            HeirWalletSubcmd::HeirConfig { kind } => Box::new(
                heir.key_provider()
                    .derive_heir_config(kind.into())
                    .await?
                    .clone(),
            ),
            HeirWalletSubcmd::Sync => {
                let mut heir = heir;
                let AnyHeritageProvider::LocalWallet(local_wallet) = heir.heritage_provider_mut()
                else {
                    return Err(
                        btc_heritage_wallet::errors::Error::IncorrectHeritageProvider("Local"),
                    );
                };
                local_wallet.local_heritage_wallet_mut().sync().await?;
                Box::new("Synchronization done")
            }
            HeirWalletSubcmd::ListInheritances { immatures, details } => {
                let heritages = heir.list_heritages().await?;

                let heritages = if immatures {
                    // If will want to take immature UTXO into account, just pass the Vec as-is
                    heritages
                } else {
                    // Else, filter out every inheritances that have a maturity greater than now
                    heritages
                        .into_iter()
                        .filter(|h| h.maturity <= timestamp_now())
                        .collect()
                };

                let heritages = if details {
                    // If we want to have the details, do not aggregate and pass the Vec as-is
                    heritages
                } else {
                    let mut aggregator: HashMap<String, Heritage> = HashMap::new();
                    for heritage in heritages {
                        aggregator
                            .entry(heritage.heritage_id.clone())
                            .and_modify(|agg| {
                                agg.value += heritage.value;
                                agg.maturity = agg.maturity.max(heritage.maturity);
                                agg.next_heir_maturity =
                                    match (agg.next_heir_maturity, heritage.next_heir_maturity) {
                                        (None, None) => None,
                                        (None, Some(v)) | (Some(v), None) => Some(v),
                                        (Some(a), Some(b)) => Some(a.min(b)),
                                    };
                            })
                            .or_insert(heritage);
                    }
                    aggregator.into_values().collect()
                };
                Box::new(
                    heritages
                        .into_iter()
                        .map(Inheritance::from)
                        .collect::<Vec<_>>(),
                )
            }
            HeirWalletSubcmd::SpendInheritance {
                id,
                recipient,
                sign,
                broadcast,
                skip_confirmation,
            } => {
                let recipient = recipient
                    .require_network(gargs.network)
                    .map_err(|e| Error::InvalidAddressNetwork(e.to_string()))?;

                // Get the PSBT
                let (psbt, summary) = heir.create_psbt(&id, recipient).await?;
                SpendFlow::new(psbt, gargs.network)
                    .fingerprints(&get_fingerprints(&db).await?)
                    .transaction_summary(&summary)
                    .display()
                    .set_skip_confirmations(skip_confirmation)
                    .set_sign(if sign {
                        Some(heir.key_provider())
                    } else {
                        None
                    })
                    .set_broadcast(if broadcast {
                        Some(heir.heritage_provider())
                    } else {
                        None
                    })
                    .run()
                    .await?
            }
            HeirWalletSubcmd::SignPsbt {
                psbt,
                broadcast,
                skip_confirmation,
            } => {
                SpendFlow::new(psbt, gargs.network)
                    .fingerprints(&get_fingerprints(&db).await?)
                    .sign(heir.key_provider())
                    .set_skip_confirmations(skip_confirmation)
                    .set_broadcast(if broadcast {
                        Some(heir.heritage_provider())
                    } else {
                        None
                    })
                    .run()
                    .await?
            }
            HeirWalletSubcmd::BroadcastPsbt { psbt } => {
                SpendFlow::<AnyKeyProvider, _>::new(psbt, gargs.network)
                    .broadcast(heir.heritage_provider())
                    .run()
                    .await?
            }
        };
        Ok(res)
    }
}
