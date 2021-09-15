use rustbot::prelude::*;
use std::collections::BTreeMap;

const AIRPORTS_CSV: &str = include_str!("../../data/airports.csv");

fn parse_airports(data: &'static str) -> Result<BTreeMap<String, Coords>> {
    let mut result: BTreeMap<String, Coords> = BTreeMap::new();

    for line in data.split('\n') {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split(',').collect();

        if parts.len() != 4 {
            bail!(
                "got a wrong-size csv record? expected len=4, got len={} with {:?}",
                parts.len(),
                line,
            );
        }

        let iata = parts[0]; // 3 letters
        let icao = parts[1]; // 4 letters

        let coords = Coords {
            lat: parts[2].to_string(),
            lon: parts[3].to_string(),
        };

        if iata != "\\N" {
            result.insert(iata.to_string(), coords.clone());
        }
        result.insert(icao.to_string(), coords);
    }

    Ok(result)
}

#[derive(Debug, Clone)]
pub struct Coords {
    pub lat: String,
    pub lon: String,
}

lazy_static! {
    static ref AIRPORTS: BTreeMap<String, Coords> = parse_airports(AIRPORTS_CSV).unwrap();
}

pub fn locate(name: &str) -> Option<Coords> {
    if name.len() == 3 || name.len() == 4 {
        AIRPORTS.get(&name.to_ascii_uppercase()).cloned()
    } else {
        None
    }
}
