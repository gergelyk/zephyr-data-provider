use crate::common::{parse_selector, Measurement, Station};

use anyhow::anyhow;
use encoding_rs::UTF_8;
use scraper::Html;
use serde::Deserialize;
use spin_sdk::http::{Method, Request, Response};
use std::collections::HashMap;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct MeasurementRaw {
    temperatura: Option<f64>,
    // humitat: Option<f64>,
    // precipitacio: Option<f64>,
    velocitatVent: Option<f64>,
    direccioVent: Option<f64>,
    // alturaSensorVent: Option<u64>,
    ratxaMaximaVent: Option<f64>,
    // direccioRatxaMaximaVent: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct CoordinatesRaw {
    latitud: f64,
    longitud: f64,
}

#[derive(Debug, Deserialize)]
struct StacionRaw {
    nom: String,
    coordenades: CoordinatesRaw,
    altitud: f64,
}

type MeasurementsRaw = HashMap<String, HashMap<String, MeasurementRaw>>;
type StationsRaw = HashMap<String, StacionRaw>;

pub async fn fetch_data() -> anyhow::Result<(Vec<Station>, Vec<Measurement>)> {
    println!("Fetching...");
    let url = "https://www.meteo.cat/observacions/xema";

    let request = Request::builder().method(Method::Get).uri(url).build();

    let response: Response = spin_sdk::http::send(request).await?;
    let (body, _, decoding_errors) = UTF_8.decode(response.body());
    if !decoding_errors {
        println!("Decoding errors found");
    }

    println!("Parsing...");
    let document = Html::parse_document(&body);

    println!("Analyzing...");
    let script_selector = parse_selector("script")?;

    let mut measurements_raw: Option<MeasurementsRaw> = None;
    let mut stations_raw: Option<StationsRaw> = None;

    for script in document.select(&script_selector) {
        if let Some(content) = script.text().next() {
            for line in content.lines() {
                let line = line.trim();
                if let Some(stripped) = line.strip_prefix("var dades = ") {
                    if let Some(stripped) = stripped.strip_suffix(";") {
                        let json = stripped.trim();
                        let measurements_raw_tmp: MeasurementsRaw = serde_json::from_str(json)?;
                        measurements_raw = Some(measurements_raw_tmp);
                        continue;
                    }
                }
                if let Some(stripped) = line.strip_prefix("var meta = ") {
                    if let Some(stripped) = stripped.strip_suffix(";") {
                        let json = stripped.trim();
                        let stations_raw_tmp: StationsRaw = serde_json::from_str(json)?;
                        stations_raw = Some(stations_raw_tmp);
                        continue;
                    }
                }
            }
        }
    }

    let measurements_raw = measurements_raw.ok_or_else(|| anyhow!("No measurements found"))?;
    let stations_raw = stations_raw.ok_or_else(|| anyhow!("No stations found"))?;
    let mut measurements_raw_items: Vec<(&String, &HashMap<String, MeasurementRaw>)> =
        measurements_raw.iter().collect();
    measurements_raw_items.sort_by_key(|&(date, _)| date);
    let (last_timestamp, last_measurements_raw) = measurements_raw_items
        .last()
        .ok_or_else(|| anyhow!("Empty list of measurements"))?;

    let mut available_stations: Vec<Station> = vec![];
    let mut measurements: Vec<Measurement> = vec![];

    for (vendor_id, measurement_raw) in last_measurements_raw.iter() {
        if let Some(wind_speed) = measurement_raw.velocitatVent {
            if let Some(station_raw) = stations_raw.get(vendor_id) {
                let station_url = format!(
                    "https://www.meteo.cat/observacions/xema/dades?codi={}",
                    vendor_id
                );
                let station_id = format!("{:x}", md5::compute(&station_url));
                let measurement = Measurement {
                    station_id: station_id.clone(),
                    wind_speed: wind_speed.round() as u64,
                    wind_direction: measurement_raw.direccioVent,
                    gusts_speed: measurement_raw.ratxaMaximaVent.map(|v| v.round() as u64),
                    temperature: measurement_raw.temperatura,
                    last_update_utc: last_timestamp.as_str()[last_timestamp.len()-6..last_timestamp.len()-1].to_string(), // TO BE REMOVED
                    last_update: last_timestamp.to_string(),
                };
                let station = Station {
                    id: station_id,
                    name: station_raw.nom.to_string(),
                    elevation: station_raw.altitud.round() as i64,
                    url: station_url,
                    lat: station_raw.coordenades.latitud,
                    long: station_raw.coordenades.longitud,
                    available: true,
                };
                available_stations.push(station);
                measurements.push(measurement);
            } else {
                println!("Station details unavailable: {}", vendor_id);
            }
        }
    }

    Ok((available_stations, measurements))
}
