mod geo;
mod scrape;

use anyhow::{anyhow, Result};
use askama::Template;
use chrono::NaiveDate;
use futures::future::join_all;
use scrape::Client;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

struct Pickup {
    pub street: String,
    pub date: i64,
    segments: Vec<Vec<geo::Point>>,
}

impl Pickup {
    fn from(pickup: geo::Pickup) -> Result<Self> {
        let date = NaiveDate::parse_from_str(&pickup.date, "%d.%m.%Y")?
            .and_hms(0, 0, 0)
            .timestamp_millis();

        Ok(Self {
            street: pickup.street,
            date,
            segments: pickup.segments,
        })
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct RenderTemplate {
    pickups: Vec<Pickup>,
}

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

#[derive(Debug)]
enum Format {
    Json,
    Csv,
}

impl TryFrom<&OsStr> for Format {
    type Error = anyhow::Error;

    fn try_from(extension: &OsStr) -> Result<Self> {
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
    let extension = output
        .extension()
        .ok_or_else(|| anyhow!("No file extension"))?;
    let format = Format::try_from(extension)?;
    let client = Client::new()?;

    let futures = client
        .queries()
        .await?
        .into_iter()
        .map(|q| client.get_pickup(q));

    let mut file = File::create(output)?;

    match format {
        Format::Json => {
            let data: Vec<scrape::Pickup> = join_all(futures)
                .await
                .into_iter()
                .filter_map(Result::ok)
                .collect();

            serde_json::to_writer(file, &data)?;
        }
        Format::Csv => {
            for future in futures {
                if let Ok(pickup) = future.await {
                    file.write_all(&format!("{};{}\n", pickup.street, pickup.date).as_bytes())?;
                }
            }
        }
    }

    Ok(())
}

fn process(input: &Path, osm: &Path, output: &Path) -> Result<()> {
    let pickups: Vec<scrape::Pickup> = serde_json::from_reader(File::open(input)?)?;
    let pickups = geo::convert(osm, pickups)?;
    let file = File::create(output)?;
    serde_json::to_writer(file, &pickups)?;

    Ok(())
}

fn render(input: &Path) -> Result<()> {
    let pickups: Vec<geo::Pickup> = serde_json::from_reader(File::open(input)?)?;

    let pickups = pickups
        .into_iter()
        .map(Pickup::from)
        .collect::<Result<Vec<_>>>()?;

    let template = RenderTemplate { pickups };
    println!("{}", template.render()?);

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
