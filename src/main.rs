mod geo;
mod scrape;

use anyhow::{anyhow, Result};
use futures::future::join_all;
use geo::{StreetPoints, to_points};
use scrape::{Client, Street};
use std::ffi::OsStr;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use tokio;

#[derive(StructOpt)]
enum Commands {
    #[structopt(about = "Fetch dates")]
    Fetch { output: PathBuf },

    #[structopt(about = "Compute coordinates for all streets")]
    Process {
        input: PathBuf,
        osm: PathBuf,
        output: PathBuf,
    },

    #[structopt(about = "Render HTML map")]
    Render { input: PathBuf },
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
        Format::Json => {
            let data: Vec<Street> = join_all(futures)
                .await
                .into_iter()
                .filter_map(Result::ok)
                .collect();

            serde_json::to_writer(file, &data)?;
        }
        Format::Csv => {
            for future in futures {
                if let Ok(street) = future.await {
                    file.write(&format!("{};{}\n", street.name, street.date).as_bytes())?;
                }
            }
        }
    }

    Ok(())
}

fn process(input: &Path, osm: &Path, output: &Path) -> Result<()> {
    let streets: Vec<Street> = serde_json::from_reader(File::open(input)?)?;
    let street_points = to_points(osm, streets)?;
    let file = File::create(output)?;
    serde_json::to_writer(file, &street_points)?;

    Ok(())
}

fn render(input: &Path) -> Result<()> {
    let streets: Vec<StreetPoints> = serde_json::from_reader(File::open(input)?)?;

    println!("var streets = [");

    for street in streets {
        if street.segments.len() == 0 {
            continue;
        }

        println!(
            r#"{{date: "{}", name: "{}", segments: ["#,
            street.date, street.name
        );

        for segment in street.segments {
            print!("[");
            for point in segment {
                print!("[{}, {}],", point.lat, point.lon);
            }
            print!("],");
        }

        println!(r#"]}},"#);
    }

    println!("];");

    Ok(())
}

#[tokio::main]
async fn main() {
    let commands = Commands::from_args();

    let result = match commands {
        Commands::Fetch { output } => fetch(&output).await,
        Commands::Process { input, osm, output } => process(&input, &osm, &output),
        Commands::Render { input } => render(&input),
    };

    if let Err(err) = result {
        eprintln!("\x1B[2K\r\x1B[0;31mError\x1B[0;m {}", err);
    }
}
