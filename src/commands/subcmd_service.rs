use core::any::Any;

use btc_heritage_wallet::{
    errors::Result,
    heritage_service_api_client::{HeritageServiceClient, HeritageServiceConfig, TokenCache},
    Database, DatabaseSingleItem,
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
    /// Display the current Heritage Service configuration options
    Config {
        /// Set the default values using the current Heritage Service configuration options instead of just displaying them
        #[arg(long, default_value_t = false)]
        set: bool,
    },
}

impl super::CommandExecutor for ServiceSubcmd {
    async fn execute(
        self,
        params: Box<dyn Any + Send>,
    ) -> Result<Box<dyn crate::display::Displayable>> {
        let (mut db, hsc): (Database, HeritageServiceConfig) = *params.downcast().unwrap();

        match &self {
            ServiceSubcmd::Config { set } => {
                if *set {
                    hsc.save(&mut db)?
                }
                return Ok(Box::new(hsc));
            }
            _ => (),
        }

        let service_client = HeritageServiceClient::from(hsc);
        service_client.load_tokens_from_cache(&db).await?;

        let res: Box<dyn crate::display::Displayable> = match self {
            ServiceSubcmd::Login => {
                service_client
                    .login(|device_auth_response| async move {
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
                    })
                    .await?;
                service_client.persist_tokens_in_cache(&mut db).await?;
                Box::new("Login successful")
            }
            ServiceSubcmd::Logout => {
                TokenCache::clear(&mut db).await?;
                service_client.logout().await?;
                Box::new("Logout successful")
            }
            ServiceSubcmd::ListWallets => service_client.list_wallets().await.map(Box::new)?,
            ServiceSubcmd::Wallet { wallet_id, subcmd } => {
                let params = Box::new((wallet_id, service_client));
                subcmd.execute(params).await?
            }
            ServiceSubcmd::ListHeirs => service_client.list_heirs().await.map(Box::new)?,
            ServiceSubcmd::Heir { heir_id, subcmd } => {
                let params = Box::new((heir_id, service_client));
                subcmd.execute(params).await?
            }
            ServiceSubcmd::ListHeritages => service_client.list_heritages().await.map(Box::new)?,
            ServiceSubcmd::Config { .. } => unreachable!("Already processed"),
        };
        Ok(res)
    }
}
