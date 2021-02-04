mod client;

use anyhow::{anyhow, Result};
use client::{Client, Street};
use futures::future::join_all;
use osmpbf::{Element, ElementReader, TagIter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

#[derive(Serialize, Deserialize, Clone)]
struct Point {
    lat: f64,
    lon: f64,
}

#[derive(Serialize, Deserialize)]
struct StreetIds {
    street: Street,
    segments: Vec<Vec<i64>>,
}

#[derive(Serialize, Deserialize)]
struct StreetPoints {
    name: String,
    date: String,
    segments: Vec<Vec<Point>>,
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

fn convert_segment_ids(ids: Vec<i64>, map: &HashMap<i64, Point>) -> Vec<Point> {
    ids.into_iter()
        .map(|id| map.get(&id).unwrap().clone())
        .collect::<Vec<Point>>()
}

fn convert_segments(segments: Vec<Vec<i64>>, map: &HashMap<i64, Point>) -> Vec<Vec<Point>> {
    segments
        .into_iter()
        .map(|segment| convert_segment_ids(segment, map))
        .collect::<Vec<Vec<Point>>>()
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
                segments: Vec::new(),
            },
        );
    }

    let reader = ElementReader::from_path(osm)?;

    reader.for_each(|element| match element {
        Element::Way(way) => {
            if let Some(name) = get_value(&mut way.tags(), "name") {
                let name = name.to_uppercase();

                if let Some(value) = street_ids.get_mut(&name) {
                    value
                        .segments
                        .push(way.refs().into_iter().collect::<Vec<_>>());
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
            name: v.street.name,
            date: v.street.date,
            segments: convert_segments(v.segments, &id_points),
        })
        .collect::<Vec<_>>();

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

        println!(r#"{{date: "{}", name: "{}", segments: ["#, street.date, street.name);

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
