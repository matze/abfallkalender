mod client;

use anyhow::{anyhow, Result};
use client::{Client, Street};
use futures::future::join_all;
use osmpbf::{Element, ElementReader, TagIter};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
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
}

enum Format {
    Json,
    Csv,
}

#[derive(Serialize, Clone)]
struct Point {
    lat: f64,
    lon: f64,
}

struct StreetIds {
    street: Street,
    ids: HashSet<i64>,
}

#[derive(Serialize)]
struct StreetPoints {
    street: Street,
    points: Vec<Point>,
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

fn get_value(tags: &mut TagIter, key: &str) -> Option<String> {
    for (k, v) in tags {
        if k == key {
            return Some(v.to_owned());
        }
    }

    None
}

fn ids_to_points(ids: HashSet<i64>, map: &HashMap<i64, Point>) -> Vec<Point> {
    ids.into_iter()
        .map(|id| map.get(&id).unwrap().clone())
        .collect::<Vec<Point>>()
}

fn process(input: &Path, osm: &Path, output: &Path) -> Result<()> {
    let streets: Vec<Street> = serde_json::from_reader(File::open(input)?)?;

    let mut street_ids: HashMap<String, StreetIds> = HashMap::new();
    let mut id_points: HashMap<i64, Point> = HashMap::new();

    for street in streets {
        street_ids.insert(
            street.name.clone(),
            StreetIds {
                street: street,
                ids: HashSet::new(),
            },
        );
    }

    let reader = ElementReader::from_path(osm)?;

    reader.for_each(|element| match element {
        Element::Way(way) => {
            if let Some(name) = get_value(&mut way.tags(), "name") {
                let name = name.to_uppercase();

                if let Some(value) = street_ids.get_mut(&name) {
                    for id in way.refs() {
                        value.ids.insert(id);
                    }
                }
            }
        }
        Element::DenseNode(node) => {
            id_points.insert(
                node.id,
                Point {
                    lat: node.lat(),
                    lon: node.lon(),
                },
            );
        }
        _ => {}
    })?;

    let street_points = street_ids
        .into_iter()
        .map(|(_, v)| StreetPoints {
            street: v.street,
            points: ids_to_points(v.ids, &id_points),
        })
        .collect::<Vec<_>>();

    let file = File::create(output)?;
    serde_json::to_writer(file, &street_points)?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let commands = Commands::from_args();

    let result = match commands {
        Commands::Fetch { output } => fetch(&output).await,
        Commands::Process { input, osm, output } => process(&input, &osm, &output),
    };

    if let Err(err) = result {
        eprintln!("\x1B[2K\r\x1B[0;31mError\x1B[0;m {}", err);
    }
}
