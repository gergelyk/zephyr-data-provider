use anyhow::anyhow;
use scraper::Selector;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub elevation: i64,
    pub url: String,
    pub lat: f64,
    pub long: f64,

    // Station is available only when it has a name and if wind
    // speed and direction are available. Wind gusts are optional.
    #[serde(skip_serializing)]
    pub available: bool,
}

#[derive(Debug, Default, Serialize)]
pub struct Measurement {
    pub station_id: String,
    pub wind_speed: u64,
    pub wind_direction: Option<f64>,
    pub gusts_speed: Option<u64>,
    pub temperature: Option<f64>,
    pub last_update: String,
}

pub fn parse_selector(selector: &str) -> anyhow::Result<Selector> {
    Selector::parse(selector).map_err(|e| anyhow!(e.to_string()))
}

pub fn wind_direction_to_degrees(direction: &str) -> Option<f64> {
    match direction.to_uppercase().as_str() {
        "N" => Some(0.0),
        "NNE" => Some(22.5),
        "NE" => Some(45.0),
        "ENE" => Some(67.5),
        "E" => Some(90.0),
        "ESE" => Some(112.5),
        "SE" => Some(135.0),
        "SSE" => Some(157.5),
        "S" => Some(180.0),
        "SSW" => Some(202.5),
        "SW" => Some(225.0),
        "WSW" => Some(247.5),
        "W" => Some(270.0),
        "WNW" => Some(292.5),
        "NW" => Some(315.0),
        "NNW" => Some(337.5),
        _ => None,
    }
}

pub fn get_units() -> HashMap<&'static str, &'static str> {
    let units: HashMap<&str, &str> = HashMap::from([
        ("wind_speed", "km/h"),
        ("wind_direction", "°"),
        ("gusts_speed", "km/h"),
        ("temperature", "°C"),
        ("lat", "°"),
        ("long", "°"),
        ("elevation", "m"),
        ("last_update", "ISO 8601")
    ]);
    units
}
