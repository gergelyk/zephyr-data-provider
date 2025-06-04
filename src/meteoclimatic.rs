use encoding_rs::ISO_8859_15;
use spin_sdk::http::{Method, Request, Response};
use std::collections::HashMap;
use crate::common::{parse_selector, wind_direction_to_degrees, Measurement, Station};
use anyhow::anyhow;
use html_escape::decode_html_entities;
use scraper::{ElementRef, Html, Node};
use chrono::{NaiveTime, Utc, TimeZone, Duration};

// Two reference points has been selected to convert from
// pixel location (x, y) to geolocation (long, lat).

// ESCAT0800000008572A: Bellmunt
const X1: f64 = 394.0;
const Y1: f64 = 223.0;
const LAT1: f64 = 42.10178106107319;
const LONG1: f64 = 2.294541510472325;

//ESCAT0800000008870D: Sitges
const X2: f64 = 309.0;
const Y2: f64 = 416.0;
const LAT2: f64 = 41.235099892573196;
const LONG2: f64 = 1.8118575503754906;

const XM: f64 = (LONG1 - LONG2) / (X1 - X2);
const XC: f64 = (X1 * LONG2 - X2 * LONG1) / (X1 - X2);
const YM: f64 = (LAT1 - LAT2) / (Y1 - Y2);
const YC: f64 = (Y1 * LAT2 - Y2 * LAT1) / (Y1 - Y2);

fn xy_to_long_lat(x: f64, y: f64) -> (f64, f64) {
    let long = XM * x + XC;
    let lat = YM * y + YC;
    (long, lat)
}

pub async fn fetch_data() -> anyhow::Result<(Vec<Station>, Vec<Measurement>)> {
    println!("Fetching...");
    let url = "https://www.meteoclimatic.net/mapinfo/ESCAT";

    let request = Request::builder().method(Method::Get).uri(url).build();

    let response: Response = spin_sdk::http::send(request).await?;
    let (body, _, decoding_errors) = ISO_8859_15.decode(response.body());
    if !decoding_errors {
        println!("Decoding errors found");
    }

    println!("Parsing...");
    let document = Html::parse_document(&body);

    println!("Analyzing...");
    let mut stations: HashMap<String, Station> = HashMap::new();
    collect_stations(&document, &mut stations)?;

    let measurements = collect_measurements(document, &mut stations)?;

    let stations_count = stations.len();

    let available_stations: Vec<Station> = stations
        .into_iter()
        .flat_map(|(_, v)| v.available.then_some(v))
        .collect();

    println!(
        "Found {} stations where {} are available",
        stations_count,
        available_stations.len()
    );

    Ok((available_stations, measurements))
}

fn collect_stations(
    document: &Html,
    stations: &mut HashMap<String, Station>,
) -> Result<(), anyhow::Error> {
    let map_selector = parse_selector("map#estacions")?;
    let point_selector = parse_selector("area")?;
    let stations_map = document
        .select(&map_selector)
        .next()
        .ok_or(anyhow!("Stations map not found"))?;

    stations_map.select(&point_selector).for_each(|area| {
        if let Err(e) = consume_area(area, stations) {
            println!("{}", e);
        }
    });

    Ok(())
}

fn consume_area(
    area: ElementRef<'_>,
    stations: &mut HashMap<String, Station>,
) -> anyhow::Result<()> {
    let shape = area.attr("shape").unwrap_or("");
    if shape == "circle" {
        let href = area
            .attr("href")
            .ok_or_else(|| anyhow::anyhow!("No href found for area element"))?;

        let coords = area.attr("coords").ok_or_else(|| {
            anyhow::anyhow!("No coords found for area element with href: {}", href)
        })?;

        let xy: Vec<&str> = coords.split(',').collect();

        let x = xy
            .first()
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid x value in coords: {} for station: {}",
                    coords,
                    href
                )
            })?;

        let y = xy
            .get(1)
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid y value in coords: {} for station: {}",
                    coords,
                    href
                )
            })?;

        let url = format!("https://www.meteoclimatic.net{}", href);
        let id = format!("{:x}", md5::compute(&url));
        let (long, lat) = xy_to_long_lat(x, y);

        let entry = Station {
            id: id.to_owned(),
            name: "".to_owned(),
            elevation: 0,
            available: false,
            url,
            lat,
            long,
        };
        if stations.contains_key(href) {
            anyhow::bail!("Duplicate station href found: {}", href);
        }
        stations.insert(href.to_owned(), entry);
    }
    Ok(())
}

fn collect_measurements(
    document: Html,
    stations: &mut HashMap<String, Station>,
) -> Result<Vec<Measurement>, anyhow::Error> {
    let tooltip_selector = parse_selector("span.tooltip")?;
    let mut measurements: Vec<Measurement> = Vec::new();

    document.select(&tooltip_selector).for_each(|span| {
        if let Err(e) = consume_span(span, stations, &mut measurements) {
            println!("{}", e);
        }
    });
    Ok(measurements)
}

fn consume_span(
    span: ElementRef<'_>,
    stations: &mut HashMap<String, Station>,
    measurements: &mut Vec<Measurement>,
) -> anyhow::Result<()> {
    let row_selector = parse_selector("tr")?;

    let vendor_id = if let Some(vendor_id) = span.attr("id") {
        vendor_id
    } else {
        anyhow::bail!("No ID found for span element");
    };

    let href = format!("/perfil/{}", vendor_id);
    if let Some(station) = stations.get_mut(&href) {
        match collect_station_info(span) {
            Ok((name, altitude)) => {
                station.name = name;
                station.elevation = altitude;
            }
            Err(e) => {
                anyhow::bail!("[{}]: {}", vendor_id, e);
            }
        }

        let mut measurement = Measurement {
            station_id: station.id.to_owned(),
            ..Default::default()
        };

        let rows = span
            .select(&row_selector)
            .map(|row| row.text().collect::<Vec<&str>>().join("").trim().to_owned())
            .collect::<Vec<String>>();

        if let Some(timestamp) = rows.get(1) {
            match collect_last_update_utc(timestamp.to_owned()) {
                Ok(last_update_utc) => {
                    let last_update = parse_time_utc(&last_update_utc)?;
                    measurement.last_update_utc = last_update_utc; // TO BE REMOVED
                    measurement.last_update = last_update.format("%Y-%m-%dT%H:%MZ").to_string();
                }
                Err(e) => {
                    anyhow::bail!("[{}]: {}", vendor_id, e);
                }
            }
        } else {
            anyhow::bail!("[{}]: Wind information not available", vendor_id);
        }

        if let Some(temp) = rows.get(2) {
            if let Ok(temperature) = collect_temp_info(temp.to_owned()) {
                measurement.temperature = Some(temperature);
            }
        }

        if let Some(wind) = rows.get(4) {
            match collect_wind_info(wind.to_owned(), true) {
                Ok((speed, direction)) => {
                    measurement.wind_speed = speed;
                    measurement.wind_direction = direction;
                }
                Err(e) => {
                    anyhow::bail!("[{}]: {}", vendor_id, e);
                }
            }
        } else {
            anyhow::bail!("[{}]: Wind information not available", vendor_id);
        }

        if let Some(gusts) = rows.get(5) {
            if let Ok((speed, _)) = collect_wind_info(gusts.to_owned(), false) {
                measurement.gusts_speed = Some(speed);
            }
        }

        station.available = true;
        measurements.push(measurement);
    } else {
        anyhow::bail!("[{}] Station not found", vendor_id);
    }
    Ok(())
}

fn parse_time_utc(time_utc_str: &String) -> Result<chrono::DateTime<Utc>, anyhow::Error> {
    let utc_now = Utc::now();
    let time_utc = NaiveTime::parse_from_str(time_utc_str, "%H:%M")
        .map_err(|e| anyhow::anyhow!("Invalid time format: {}", e))?;
    let date_time_utc = utc_now.date_naive().and_time(time_utc);
    let mut date_time = Utc::from_utc_datetime(&Utc, &date_time_utc);
    if date_time > utc_now {
        date_time = date_time - Duration::days(1);
    }
    Ok(date_time)
}

fn collect_station_info(span: ElementRef<'_>) -> anyhow::Result<(String, i64)> {
    let header_selector = parse_selector("th")?;
    let altitude_selector = parse_selector("span.petitet")?;

    let name = if let Some(header) = span.select(&header_selector).next() {
        let name = header
            .children()
            .filter_map(|child| {
                if let Node::Text(t) = child.value() {
                    Some(decode_html_entities(&t.text).to_string())
                } else {
                    None
                }
            })
            .collect::<String>()
            .trim()
            .to_string();

        // Replace non-breaking space with regular space
        name.replace('\u{00A0}', " ")
    } else {
        anyhow::bail!("No station name found in span");
    };

    let elevation = if let Some(elevation_tag) = span.select(&altitude_selector).next() {
        let elevation_str = elevation_tag.text().collect::<Vec<&str>>().join("");
        elevation_str
            .strip_prefix("(")
            .and_then(|s| s.strip_suffix(" m)"))
            .and_then(|s| s.trim().parse::<i64>().ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid elevation format: {}", elevation_str))?
    } else {
        anyhow::bail!("No elevation found in span");
    };

    Ok((name, elevation))
}

fn collect_last_update_utc(line: String) -> anyhow::Result<String> {
    println!("Collecting update time from: {}", line);
    let line = if let Some(line_stripped) = line.strip_prefix("Actualizado:") {
        line_stripped
    } else {
        anyhow::bail!("Invalid last update format: {}", line);
    };

    println!("Collecting update time from: {}", line);
    let line = if let Some(line_stripped) = line.strip_suffix("UTC") {
        line_stripped
    } else {
        anyhow::bail!("Invalid last update format: {}", line);
    };

    Ok(line.trim().to_owned())
}

fn collect_wind_info(line: String, with_direction: bool) -> anyhow::Result<(u64, Option<f64>)> {
    let mut speed: u64 = 0;
    let mut direction: Option<f64> = None;

    if line != "Calma" {
        let wind_parts = line.split_whitespace().collect::<Vec<&str>>();

        if let Some(unit_parsed) = wind_parts.get(1) {
            if *unit_parsed != "km/h" {
                anyhow::bail!("Unsupported wind speed unit '{}'", unit_parsed);
            }
        } else {
            anyhow::bail!("Unsupported wind speed unit format");
        }

        speed = wind_parts
            .first()
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid wind speed format: {}", line))?;

        if with_direction && speed != 0 {
            direction = Some(
                wind_parts
                    .get(2)
                    .and_then(|s| wind_direction_to_degrees(s))
                    .ok_or_else(|| anyhow::anyhow!("Invalid wind direction format: {}", line))?,
            );
        }
    }
    Ok((speed, direction))
}

fn collect_temp_info(line: String) -> anyhow::Result<f64> {
    let mut parts = line.split("°C");
    if parts.clone().count() == 3 {
        if let Some(val_str) = parts.next() {
            let val_str = val_str.replace(",", ".");
            if let Ok(temp) = val_str.parse::<f64>() {
                return Ok(temp);
            } else {
                anyhow::bail!("Invalid temperature value: {}", val_str);
            }
        }
    }
    anyhow::bail!("Invalid temperature format");
}
