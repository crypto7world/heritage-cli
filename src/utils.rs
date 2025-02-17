use std::{
    collections::HashMap,
    io::{stdin, stdout, Write},
};

use btc_heritage_wallet::{
    bitcoin::{Amount, Denomination},
    btc_heritage::HeritageWalletBackup,
    errors::{Error, Result},
    heritage_service_api_client::Fingerprint,
    BoundFingerprint, Database, DatabaseItem, Heir, HeirWallet, Wallet,
};
use chrono::{DateTime, Utc};
use serde::Serializer;

pub async fn ask_user_confirmation(prompt: &str) -> Result<bool> {
    print!("{prompt} Answer \"yes\" or \"no\" (default \"no\"): ");
    stdout().flush().map_err(|e| {
        log::error!("Could not display the confirmation prompt: {e}");
        Error::generic(e)
    })?;

    let mut s = tokio::task::spawn_blocking(|| {
        let mut s = String::new();
        stdin().read_line(&mut s).map_err(|e| {
            log::error!("Not a correct string: {e}");
            Error::generic(e)
        })?;
        Ok::<_, Error>(s)
    })
    .await
    .unwrap()?;

    // Remove the final \r\n, if present
    if let Some('\n') = s.chars().next_back() {
        s.pop();
    }
    if let Some('\r') = s.chars().next_back() {
        s.pop();
    }
    Ok(s == "yes".to_owned())
}

pub fn prompt_user_for_password(double_check: bool) -> Result<String> {
    let passphrase1 =
        rpassword::prompt_password("Please enter your password: ").map_err(Error::generic)?;
    if double_check {
        let passphrase2 = rpassword::prompt_password("Please re-enter your password: ")
            .map_err(Error::generic)?;
        if passphrase1 != passphrase2 {
            return Err(Error::Generic("Passwords did not match".to_owned()));
        }
    }
    Ok(passphrase1)
}

pub async fn get_fingerprints(db: &Database) -> Result<HashMap<Fingerprint, Vec<String>>> {
    let mut map = HashMap::new();
    // TODO: this is sequentially executed.
    // That is not a problem considering the Databse is in fact quite quick
    // But for the "beauty of the gesture" I should consider how to do that truly concurrently
    // if it is at all possible (but it may well not be as there is a lock on the DB)
    for heir in Heir::all_in_db(&db).await?.into_iter() {
        if let Some(fingerprint) = heir.fingerprint().ok() {
            map.entry(fingerprint)
                .or_insert(vec![])
                .push(format!("heir:{}", heir.name()))
        }
    }
    for heir_wallet in HeirWallet::all_in_db(&db).await?.into_iter() {
        if let Some(fingerprint) = heir_wallet.fingerprint().ok() {
            map.entry(fingerprint)
                .or_insert(vec![])
                .push(format!("heir-wallet:{}", heir_wallet.name()))
        }
    }
    for wallet in Wallet::all_in_db(&db).await?.into_iter() {
        if let Some(fingerprint) = wallet.fingerprint().ok() {
            map.entry(fingerprint)
                .or_insert(vec![])
                .push(format!("wallet:{}", wallet.name()))
        }
    }
    Ok(map)
}

pub(crate) fn parse_heritage_wallet_backup(
    val: &str,
) -> core::result::Result<HeritageWalletBackup, serde_json::Error> {
    serde_json::from_str(val)
}

pub(crate) fn serialize_amount<S: Serializer>(
    amount: &Amount,
    serializer: S,
) -> core::result::Result<S::Ok, S::Error> {
    if *amount >= Amount::from_btc(0.1).unwrap() {
        serializer.serialize_str(&format!("{} BTC", amount.display_in(Denomination::Bitcoin)))
    } else if *amount >= Amount::from_sat(10000) {
        serializer.serialize_str(&format!(
            "{} mBTC",
            amount.display_in(Denomination::MilliBitcoin)
        ))
    } else {
        serializer.serialize_str(&format!("{} sat", amount.display_in(Denomination::Satoshi)))
    }
}

pub(crate) fn serialize_datetime<S: Serializer>(
    dt: &DateTime<Utc>,
    serializer: S,
) -> core::result::Result<S::Ok, S::Error> {
    serializer.serialize_str(&dt.to_string())
}

pub(crate) fn serialize_opt_datetime<S: Serializer>(
    dt: &Option<DateTime<Utc>>,
    serializer: S,
) -> core::result::Result<S::Ok, S::Error> {
    if let Some(dt) = dt {
        serialize_datetime(dt, serializer)
    } else {
        serializer.serialize_str("None")
    }
}
