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

macro_rules! str_display {
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

str_display!(&str);
str_display!(String);
str_display!(Fingerprint);
str_display!(Vec<String>);
str_display!(Vec<AccountXPub>);
str_display!(Vec<WalletAddress>);

pub trait SerdeDisplay: serde::Serialize {}

impl<T: SerdeDisplay> Displayable for T {
    fn display(&self) {
        println!(
            "{}",
            serde_json::to_string_pretty(self)
                .expect("Caller responsability to ensure Json serialization works")
        )
    }
}
impl<T: SerdeDisplay> SerdeDisplay for Vec<T> {}

macro_rules! serde_display {
    ($name:ty) => {
        impl SerdeDisplay for $name {}
    };
}

serde_display!(HeirConfig);
serde_display!(MnemonicBackup);
serde_display!(Heir);
serde_display!(HeritageWalletMeta);
serde_display!(Heritage);
serde_display!(HeritageConfig);
serde_display!(AccountXPubWithStatus);
serde_display!(WalletStatus);
serde_display!(TransactionSummary);
serde_display!(BlockInclusionObjective);
serde_display!(HeritageWalletBackup);
serde_display!(PsbtSummary);

// TODO: Display it like the Ledger device is displaying it
serde_display!(LedgerPolicy);

impl<A, B, C, D> SerdeDisplay for (A, B, C, D)
where
    A: serde::Serialize,
    B: serde::Serialize,
    C: serde::Serialize,
    D: serde::Serialize,
{
}
