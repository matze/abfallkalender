mod client;

use anyhow::Result;
use client::Client;
use structopt::StructOpt;
use tokio;

#[derive(StructOpt)]
enum Commands {
    #[structopt(about = "Fetch dates")]
    Fetch,
}

async fn fetch() -> Result<()> {
    let client = Client::new()?;

    for query in client.queries().await? {
        match client.get_date(query).await {
            Ok(street) => {
                println!("{};{}", street.name, street.date);
            }
            Err(_) => {}
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let commands = Commands::from_args();

    let result = match commands {
        Commands::Fetch {} => fetch().await,
    };

    if let Err(err) = result {
        eprintln!("\x1B[2K\r\x1B[0;31mError\x1B[0;m {}", err);
    }
}
