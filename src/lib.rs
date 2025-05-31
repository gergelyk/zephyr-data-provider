mod meteoclimatic;

use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;
use std::collections::HashMap;

fn log_req_info(req: &Request) {
    let client_addr: &str = req
        .header("spin-client-addr")
        .map(|v| v.as_str().unwrap_or("?!"))
        .unwrap_or("?");

    let full_url: &str = req
        .header("spin-full-url")
        .map(|v| v.as_str().unwrap_or("?!"))
        .unwrap_or("?");

    log::info!("{} {} {}", client_addr, req.method(), full_url);
}

fn plain_text_resp(status: u16, message: &str) -> Response {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(message)
        .build()
}

fn json_ok_resp(payload: &str) -> Response {
    Response::builder()
        .status(200)
        .header("content-type", "application/json; charset=utf-8")
        .body(payload)
        .build()
}

fn check_token(req: &Request) -> anyhow::Result<Option<Response>> {
    let query_string = req.query();
    let query_vector = querystring::querify(query_string);
    let query: HashMap<_, _> = query_vector.into_iter().collect();

    let expected_token = spin_sdk::variables::get("api_token")?;

    if let Some(token) = query.get("token") {
        if token != &expected_token {
            log::error!("Invalid token: {}", token);
            return Ok(Some(plain_text_resp(400, "Invalid token")));
        }
    } else {
        log::error!("Missing token");
        return Ok(Some(plain_text_resp(400, "Missing token")));
    }

    Ok(None)
}

fn handle_get_health_check() -> anyhow::Result<Response> {
    Ok(plain_text_resp(200, "OK"))
}

fn handle_get_version_info() -> anyhow::Result<Response> {
    let app_name = env!("CARGO_PKG_NAME");
    let app_version = env!("CARGO_PKG_VERSION");
    Ok(plain_text_resp(200, &format!("{app_name} v{app_version}")))
}

fn handle_get_units(req: &Request) -> anyhow::Result<Response> {
    if let Some(resp) = check_token(req)? {
        return Ok(resp);
    };
    let units = meteoclimatic::get_units();
    let json = serde_json::to_string(&units)?;
    Ok(json_ok_resp(json.as_str()))
}

async fn handle_get_stations(req: &Request) -> anyhow::Result<Response> {
    if let Some(resp) = check_token(req)? {
        return Ok(resp);
    };
    let (stations, _) = meteoclimatic::fetch_data().await?;
    let json = serde_json::to_string(&stations)?;
    Ok(json_ok_resp(json.as_str()))
}

async fn handle_get_measurements(req: &Request) -> anyhow::Result<Response> {
    if let Some(resp) = check_token(req)? {
        return Ok(resp);
    };
    let (_, measurements) = meteoclimatic::fetch_data().await?;
    let json = serde_json::to_string(&measurements)?;
    Ok(json_ok_resp(json.as_str()))
}

async fn handle_get(req: &Request) -> anyhow::Result<Response> {
    match req.path() {
        "/api/v1/health" => handle_get_health_check(),
        "/api/v1/version" => handle_get_version_info(),
        "/api/v1/units" => handle_get_units(req),
        "/api/v1/stations" => handle_get_stations(req).await,
        "/api/v1/measurements" => handle_get_measurements(req).await,
        _ => Ok(plain_text_resp(404, "Not Found")),
    }
}

#[http_component]
async fn handle_zephyr_data_provider(req: Request) -> anyhow::Result<impl IntoResponse> {
    simple_logger::init_with_level(log::Level::Info)?;
    log_req_info(&req);

    match req.method() {
        spin_sdk::http::Method::Get => handle_get(&req).await,
        _ => Ok(plain_text_resp(405, "Method not allowed")),
    }
}
