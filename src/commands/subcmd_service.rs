use core::any::Any;

use btc_heritage_wallet::{
    errors::Result,
    heritage_service_api_client::{HeritageServiceClient, Tokens},
    Database,
};

/// Commands related purely to the Heritage service
#[derive(Debug, Clone, clap::Subcommand)]
pub enum ServiceSubcmd {
    /// Login to the Heritage service and store the resulting tokens in the internal database
    Login,
    /// Logout of the Heritage service and discard the previously stored tokens
    Logout,
    /// List the Heritage wallets already created in the Heritage service, if any
    ListWallets,
    /// Commands managing existing wallets on the service
    Wallet {
        /// The ID of the wallet to operate
        wallet_id: String,
        #[command(subcommand)]
        subcmd: super::subcmd_service_wallet::WalletSubcmd,
    },
    /// List the Heirs declared in the Heritage service, if any
    ListHeirs,
    /// Commands managing existing heirs on the service
    Heir {
        /// The ID of the heir to operate
        heir_id: String,
        #[command(subcommand)]
        subcmd: super::subcmd_service_heir::HeirSubcmd,
    },
    /// List the Heritages that you are - or will be - eligible to in Heritage service, if any
    ListHeritages,
}

impl super::CommandExecutor for ServiceSubcmd {
    fn execute(self, params: Box<dyn Any>) -> Result<Box<dyn crate::display::Displayable>> {
        let (mut db, service_gargs): (Database, super::ServiceGlobalArgs) =
            *params.downcast().unwrap();

        let service_client =
            HeritageServiceClient::new(service_gargs.service_api_url, Tokens::load(&db)?);

        let res: Box<dyn crate::display::Displayable> = match self {
            ServiceSubcmd::Login => Tokens::new(
                &service_gargs.auth_url,
                &service_gargs.auth_client_id,
                |device_auth_response| {
                    let verification_uri_complete = format!(
                        "{}?user_code={}",
                        device_auth_response.verification_uri, device_auth_response.user_code
                    );

                    let human_formated_code = format!(
                        "{}-{}",
                        &device_auth_response.user_code[..4],
                        &device_auth_response.user_code[4..]
                    );

                    println!("Go to {verification_uri_complete} to approve the connection");
                    println!();
                    println!("Verify that the code displayed is: {human_formated_code}");
                    println!();

                    _ = open::that(verification_uri_complete);

                    Ok(())
                },
            )?
            .save(&mut db)
            .map(|()| Box::new("Login successful"))?,
            ServiceSubcmd::Logout => todo!(),
            ServiceSubcmd::ListWallets => service_client.list_wallets().map(Box::new)?,
            ServiceSubcmd::Wallet { wallet_id, subcmd } => {
                let params = Box::new((wallet_id, service_client));
                subcmd.execute(params)?
            }
            ServiceSubcmd::ListHeirs => service_client.list_heirs().map(Box::new)?,
            ServiceSubcmd::Heir { heir_id, subcmd } => {
                let params = Box::new((heir_id, service_client));
                subcmd.execute(params)?
            }
            ServiceSubcmd::ListHeritages => service_client.list_heritages().map(Box::new)?,
        };
        Ok(res)
    }
}
