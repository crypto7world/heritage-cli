use core::any::Any;
use std::collections::HashSet;

use btc_heritage_wallet::{
    errors::Result, ledger::WalletPolicy, Database, DatabaseItem, LedgerPolicy, OnlineWallet,
    Wallet,
};

use crate::display::Displayable;

/// Wallet Ledger Policy management subcommand.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum WalletLedgerPolicySubcmd {
    /// List the Ledger policies (Bitcoin descriptors in a Ledger format) of the wallet
    List,
    /// List the Ledger policies of the wallet that are already registered in the Ledger
    ListRegistered,
    /// Register policies on a Ledger device
    Register {
        /// The policies to register.
        #[arg(value_name = "POLICY", num_args=1.., value_parser=parse_ledger_policies)]
        policies: Vec<LedgerPolicy>,
    },
    /// Retrieve Ledger policies using the Online component of the wallet and register them to the Offline component
    AutoRegister,
}

impl super::CommandExecutor for WalletLedgerPolicySubcmd {
    async fn execute(
        self,
        params: Box<dyn Any + Send>,
    ) -> Result<Box<dyn crate::display::Displayable>> {
        let (mut wallet, mut db): (Wallet, Database) = *params.downcast().unwrap();
        let res: Box<dyn crate::display::Displayable> = match self {
            WalletLedgerPolicySubcmd::List => Box::new(
                wallet
                    .online_wallet()
                    .backup_descriptors()
                    .await?
                    .into_iter()
                    .filter_map(|d| TryInto::<LedgerPolicy>::try_into(d).ok())
                    .collect::<Vec<_>>(),
            ),
            WalletLedgerPolicySubcmd::ListRegistered => {
                let btc_heritage_wallet::AnyKeyProvider::Ledger(ledger_wallet) =
                    wallet.key_provider()
                else {
                    return Err(btc_heritage_wallet::errors::Error::IncorrectKeyProvider(
                        "Ledger",
                    ));
                };
                Box::new(ledger_wallet.list_registered_policies())
            }
            WalletLedgerPolicySubcmd::Register { policies } => {
                let count = if let btc_heritage_wallet::AnyKeyProvider::Ledger(ledger_wallet) =
                    wallet.key_provider_mut()
                {
                    ledger_wallet
                        .register_policies(&policies, display_wallet_policy)
                        .await?
                } else {
                    return Err(btc_heritage_wallet::errors::Error::IncorrectKeyProvider(
                        "Ledger",
                    ));
                };
                wallet.save(&mut db).await?;
                Box::new(format!("{count} policies registered"))
            }
            WalletLedgerPolicySubcmd::AutoRegister => {
                let policies = if let btc_heritage_wallet::AnyKeyProvider::Ledger(ledger_wallet) =
                    wallet.key_provider()
                {
                    let registered_policy_ids = ledger_wallet
                        .list_registered_policies()
                        .into_iter()
                        .map(|(id, ..)| id)
                        .collect::<HashSet<_>>();
                    wallet
                        .online_wallet()
                        .backup_descriptors()
                        .await?
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, d)| {
                            TryInto::<LedgerPolicy>::try_into(d)
                                .map_err(|e| {
                                    log::warn!(
                                    "Cannot convert Descriptor Backup #{i} into a LedgerPolicy: {e}"
                                );
                                    e
                                })
                                .ok()
                        })
                        .filter(|p| !registered_policy_ids.contains(&p.get_account_id()))
                        .collect::<Vec<_>>()
                } else {
                    return Err(btc_heritage_wallet::errors::Error::IncorrectKeyProvider(
                        "Ledger",
                    ));
                };
                log::info!("{} new policies to register", policies.len());
                let count = if let btc_heritage_wallet::AnyKeyProvider::Ledger(ledger_wallet) =
                    wallet.key_provider_mut()
                {
                    ledger_wallet
                        .register_policies(&policies, display_wallet_policy)
                        .await?
                } else {
                    unreachable!("already confirmed it is a Ledger")
                };
                wallet.save(&mut db).await?;
                Box::new(format!("{count} new policies registered"))
            }
        };
        Ok(res)
    }
}

fn display_wallet_policy(wallet_policy: &WalletPolicy) {
    println!("\x1b[4mRegister account\x1b[0m");
    wallet_policy.display();
    println!();
}

fn parse_ledger_policies(val: &str) -> Result<LedgerPolicy> {
    Ok(val.try_into()?)
}
