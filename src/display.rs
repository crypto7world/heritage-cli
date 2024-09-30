use btc_heritage_wallet::{
    btc_heritage::{
        heritage_wallet::WalletAddress, AccountXPub, BlockInclusionObjective, HeirConfig,
        HeritageConfig, HeritageWalletBackup,
    },
    heritage_service_api_client::{
        AccountXPubWithStatus, Fingerprint, Heir, Heritage, HeritageWalletMeta, TransactionSummary,
    },
    key_provider::MnemonicBackup,
    online_wallet::WalletStatus,
    LedgerPolicy, PsbtSummary,
};

pub trait Displayable {
    fn display(&self);
}

impl Displayable for () {
    fn display(&self) {}
}

macro_rules! displayable {
    (Vec<$name:ty>) => {
        impl Displayable for Vec<$name> {
            fn display(&self) {
                for e in self {
                    println!("{e}")
                }
            }
        }
    };
    ($name:ty) => {
        impl Displayable for $name {
            fn display(&self) {
                println!("{self}")
            }
        }
    };
}

displayable!(&str);
displayable!(String);
displayable!(Fingerprint);
displayable!(Vec<String>);
displayable!(Vec<AccountXPub>);
displayable!(Vec<WalletAddress>);

pub trait AutoDisplayable {}

impl<T: serde::Serialize + AutoDisplayable> Displayable for T {
    fn display(&self) {
        println!(
            "{}",
            serde_json::to_string_pretty(self)
                .expect("Caller responsability to ensure Json serialization works")
        )
    }
}

impl AutoDisplayable for HeirConfig {}
impl AutoDisplayable for MnemonicBackup {}
impl AutoDisplayable for Heir {}
impl AutoDisplayable for HeritageWalletMeta {}
impl AutoDisplayable for Heritage {}
impl AutoDisplayable for HeritageConfig {}
impl AutoDisplayable for AccountXPubWithStatus {}
impl AutoDisplayable for LedgerPolicy {}
impl AutoDisplayable for WalletStatus {}
impl AutoDisplayable for TransactionSummary {}
impl AutoDisplayable for BlockInclusionObjective {}
impl AutoDisplayable for HeritageWalletBackup {}
impl AutoDisplayable for PsbtSummary {}
impl<A, B, C, D> AutoDisplayable for (A, B, C, D)
where
    A: serde::Serialize,
    B: serde::Serialize,
    C: serde::Serialize,
    D: serde::Serialize,
{
}
impl<T: AutoDisplayable> AutoDisplayable for Vec<T> {}
