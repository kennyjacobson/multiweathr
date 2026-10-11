use crate::error::WeatherError;
use crate::weather::types::{CelestialEvents, WeatherLocation, WeatherUnits};
use async_trait::async_trait;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

pub mod met_office;
pub mod open_meteo;
pub mod supplementary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherProviderResponse {
    pub weather_code: i32,
    pub temperature: f64,
    pub precipitation: f64,
    pub wind_speed: f64,
    pub wind_direction: f64,
    pub sun: CelestialEvents,
    pub moon_phase: Option<f64>,
    pub timestamp: String,
    pub attribution: String,
    /// Daily forecast, starting with today. Empty when the provider has none.
    #[serde(default)]
    pub forecast: Vec<DailyForecastResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyForecastResponse {
    pub date: NaiveDate,
    pub weather_code: i32,
    pub temperature_max: f64,
    pub temperature_min: f64,
    pub precipitation_probability: Option<u8>,
}

#[async_trait]
pub trait WeatherProvider: Send + Sync {
    async fn get_current_weather(
        &self,
        location: &WeatherLocation,
        units: &WeatherUnits,
    ) -> Result<WeatherProviderResponse, WeatherError>;

    fn get_attribution(&self) -> &'static str;
}
