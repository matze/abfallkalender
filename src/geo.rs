use crate::scrape;
use anyhow::Result;
use osmpbf::{Element, ElementReader, TagIter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone)]
pub struct Point {
    pub lat: f64,
    pub lon: f64,
}

const ORIGIN: Point = Point {
    lat: 49.0f64,
    lon: 8.40f64,
};

#[derive(Serialize, Deserialize)]
struct StreetIds {
    pickup: scrape::Pickup,
    segments: Vec<Vec<i64>>,
}

#[derive(Serialize, Deserialize)]
pub struct Pickup {
    pub street: String,
    pub date: String,
    pub segments: Vec<Vec<Point>>,
}

fn distance(a: &Point, b: &Point) -> f64 {
    let mut ma = a.clone();
    let mut mb = b.clone();
    ma.lon -= mb.lon;
    ma.lon = ma.lon.to_radians();
    ma.lat = ma.lat.to_radians();
    mb.lat = mb.lat.to_radians();
    let dz: f64 = ma.lat.sin() - mb.lat.sin();
    let dx: f64 = ma.lon.cos() * ma.lat.cos() - mb.lat.cos();
    let dy: f64 = ma.lon.sin() * ma.lat.cos();
    ((dx * dx + dy * dy + dz * dz).sqrt() / 2.0).asin() * 2.0 * 6372.8
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

fn valid_segment(segment: &[i64], map: &HashMap<i64, Point>) -> bool {
    if let Some(first) = segment.get(0) {
        return distance(map.get(first).unwrap(), &ORIGIN) < 10.0f64;
    }

    false
}

fn convert_segments(segments: Vec<Vec<i64>>, map: &HashMap<i64, Point>) -> Vec<Vec<Point>> {
    segments
        .into_iter()
        .filter(|segment| valid_segment(&segment, map))
        .map(|segment| convert_segment_ids(segment, map))
        .collect::<Vec<Vec<Point>>>()
}

pub fn convert(osm: &Path, pickups: Vec<scrape::Pickup>) -> Result<Vec<Pickup>> {
    let mut street_ids: HashMap<String, StreetIds> = HashMap::new();
    let mut id_points: HashMap<i64, Point> = HashMap::new();

    for pickup in pickups {
        street_ids.insert(
            pickup.street.clone(),
            StreetIds {
                pickup,
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

    Ok(street_ids
        .into_iter()
        .map(|(_, v)| Pickup {
            street: v.pickup.street,
            date: v.pickup.date,
            segments: convert_segments(v.segments, &id_points),
        })
        .collect::<Vec<_>>())
}
