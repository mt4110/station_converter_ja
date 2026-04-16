use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: String,
}

#[derive(Debug, Serialize)]
pub struct ReadyResponse {
    pub status: &'static str,
    pub database_type: String,
    pub cache: &'static str,
}

#[derive(Debug, Serialize)]
pub struct StationSummary {
    pub station_uid: String,
    pub station_name: String,
    pub line_name: String,
    pub operator_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub status: String,
}
