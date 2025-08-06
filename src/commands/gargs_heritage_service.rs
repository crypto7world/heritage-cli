use std::sync::Arc;

use btc_heritage_wallet::heritage_service_api_client::HeritageServiceConfig;

use crate::display::SerdeDisplay;

#[derive(Clone, Debug, clap::Args)]
pub struct HeritageServiceGlobalArgs {
    /// Set the URL of the Heritage service API.
    #[arg(
        long,
        value_hint = clap::ValueHint::Url,
        env="HERITAGE_SERVICE_API_URL",
        global = true,
    )]
    pub service_api_url: Option<Arc<str>>,
    /// Set the URL of the Heritage service OAUTH token endpoint for the CLI.
    #[arg(
        long,
        value_hint = clap::ValueHint::Url,
        env="HERITAGE_AUTH_URL",
        global = true,
    )]
    pub auth_url: Option<Arc<str>>,
    /// Set the OAUTH Client Id of the CLI for the Heritage service authentication endpoint.
    #[arg(long, env = "HERITAGE_AUTH_CLIENT_ID", global = true)]
    pub auth_client_id: Option<Arc<str>>,
}

impl SerdeDisplay for HeritageServiceConfig {}
