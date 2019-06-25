use shared::prelude::*;
use std::collections::BTreeMap;

const AIRPORTS_CSV: &'static str = include_str!("../../data/airports.csv");

fn parse_airports(data: &'static str) -> Result<BTreeMap<String, Coords>> {
    let mut reader = csv::Reader::from_reader(data.as_bytes());

    let mut result: BTreeMap<String, Coords> = BTreeMap::new();

    for record in reader.records() {
        let record = record?;

        if record.len() != 14 {
            return Err(format!(
                "got a wrong-size csv record? expected len=14, got len={} with {:?}",
                record.len(),
                record,
            )
            .into());
        }

        let iata = &record[4]; // 3 letters
        let icao = &record[5]; // 4 letters

        let coords = Coords {
            lat: record[6].to_string(),
            lon: record[7].to_string(),
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
