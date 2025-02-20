use btc_heritage_wallet::{
    btc_heritage::AccountXPub, errors::Result, heritage_service_api_client::AccountXPubWithStatus,
    KeyProvider, OnlineWallet, Wallet,
};
use core::any::Any;

/// Wallet Account XPubs management subcommand.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum WalletAXpubSubcmd {
    /// List Account eXtended Public Keys generated using the Offline component of the wallet
    Generate {
        /// The index (inclusive) at which we start generation of Account eXtended Public Keys
        #[arg(short, long, default_value_t = 0)]
        start: u32,
        /// The index (exclusive) at which we stop generation of Account eXtended Public Keys
        #[arg(short, long, default_value_t = 20)]
        end: u32,
    },
    /// List the Account eXtended Public Keys already added by the Online component of the wallet and their status
    ListAdded {
        /// List the used Account eXtended Public Keys of the Online wallet
        #[arg(long, default_value_t = true)]
        used: bool,
        /// List the unused Account eXtended Public Keys of the Online wallet
        #[arg(long, default_value_t = true)]
        unused: bool,
    },
    /// Add Account eXtended Public Keys to the Online component of the wallet
    Add {
        /// The Account eXtended Public Key(s) to feed
        #[arg(value_name = "ACCOUNT_XPUB", num_args=1.., required = true, value_parser=parse_account_xpubs)]
        account_xpubs: Vec<AccountXPub>,
    },
    /// Generate Account eXtended Public Keys using the Offline component of the wallet and add them to the Online component
    AutoAdd {
        /// The number of unused Account eXtended Public Keys to ensure
        #[arg(short, long, default_value_t = 20)]
        count: usize,
    },
}

impl super::CommandExecutor for WalletAXpubSubcmd {
    async fn execute(
        self,
        params: Box<dyn Any + Send>,
    ) -> Result<Box<dyn crate::display::Displayable>> {
        let mut wallet: Wallet = *params.downcast().unwrap();
        let res: Box<dyn crate::display::Displayable> = match self {
            WalletAXpubSubcmd::ListAdded { used, unused } => {
                let mut res = wallet.list_account_xpubs().await?;
                if !used {
                    res.retain(|e| {
                        match e {
                            AccountXPubWithStatus::Used(_) => false,
                            _ => true,
                        };
                        true
                    })
                }
                if !unused {
                    res.retain(|e| {
                        match e {
                            AccountXPubWithStatus::Unused(_) => false,
                            _ => true,
                        };
                        true
                    })
                }
                Box::new(res)
            }
            WalletAXpubSubcmd::Generate { start, end } => {
                Box::new(wallet.derive_accounts_xpubs(start..end).await?)
            }
            WalletAXpubSubcmd::Add { account_xpubs } => {
                wallet.feed_account_xpubs(account_xpubs).await?;
                Box::new(())
            }
            WalletAXpubSubcmd::AutoAdd { count } => {
                let axpubs = wallet.list_account_xpubs().await?;
                let (unused_count, last_seen_index) =
                    axpubs
                        .iter()
                        .fold((0usize, None), |(uc, lsi), axpub| match axpub {
                            AccountXPubWithStatus::Used(axpub) => {
                                (uc, core::cmp::max(lsi, Some(axpub.descriptor_id())))
                            }
                            AccountXPubWithStatus::Unused(axpub) => {
                                (uc + 1, core::cmp::max(lsi, Some(axpub.descriptor_id())))
                            }
                        });
                let start = last_seen_index.map(|lsi| lsi + 1).unwrap_or(0);
                let end = start + (count.checked_sub(unused_count).unwrap_or(0)) as u32;
                let account_xpubs = wallet.derive_accounts_xpubs(start..end).await?;
                wallet.feed_account_xpubs(account_xpubs).await?;
                Box::new(())
            }
        };
        Ok(res)
    }
}

fn parse_account_xpubs(val: &str) -> Result<AccountXPub> {
    Ok(AccountXPub::try_from(val)?)
}
