mod client;

use anyhow::{anyhow, Result};
use client::Client;
use std::fs::File;
use std::io::prelude::*;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use tokio;

#[derive(StructOpt)]
enum Commands {
    #[structopt(about = "Fetch dates")]
    Fetch { output: PathBuf },
}

enum Format {
    Json,
    Csv,
}

impl Format {
    fn from(extension: &OsStr) -> Result<Self> {
        if extension == "json" {
            return Ok(Format::Json);
        }

        if extension == "csv" {
            return Ok(Format::Csv);
        }

        Err(anyhow!("Unsupported file extension"))
    }
}

async fn fetch(output: &Path) -> Result<()> {
    let extension = output.extension().ok_or(anyhow!("No file extension"))?;
    let format = Format::from(extension)?;
    let client = Client::new()?;

    let futures = client
        .queries()
        .await?
        .into_iter()
        .map(|q| client.get_date(q));

    let mut file = File::create(output)?;

    match format {
        Format::Json => {},
        Format::Csv => {
            for future in futures {
                if let Ok(street) = future.await {
                    file.write(&format!("{};{}\n", street.name, street.date).as_bytes())?;
                }
            }
        },
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let commands = Commands::from_args();

    let result = match commands {
        Commands::Fetch { output } => fetch(&output).await,
    };

    if let Err(err) = result {
        eprintln!("\x1B[2K\r\x1B[0;31mError\x1B[0;m {}", err);
    }
}
