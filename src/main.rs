mod app;
mod archive;
mod aur;
mod config;
mod error;
mod git;

use app::App;
use config::Config;
use error::Result;
use tracing::Level;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    dotenv::dotenv().ok();

    let config = Config::new();
    let app = App::new(config)?;

    app.run().await
}
