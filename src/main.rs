use clap::Parser;
use ms::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    cli.run().await
}
