use crate::error::{NetworkError, WeatherError};
use crate::weather::provider::{DailyForecastResponse, WeatherProvider, WeatherProviderResponse};
use crate::weather::types::{
    CelestialEvents, PrecipitationUnit, TemperatureUnit, WeatherLocation, WeatherUnits,
    WindSpeedUnit,
};
use crate::weather::units::{normalize_precipitation, normalize_temperature, normalize_wind_speed};
use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;
use serde::de::{self, Deserializer};
use std::time::Duration;

const OPEN_METEO_BASE_URL: &str = "https://api.open-meteo.com/v1/forecast";
/// The most days Open-Meteo will forecast.
pub const MAX_FORECAST_DAYS: u8 = 16;

pub struct OpenMeteoProvider {
    client: reqwest::Client,
    base_url: String,
    forecast_days: u8,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    current: CurrentWeather,
    #[serde(default)]
    daily: Option<DailyWeather>,
}

/// Open-Meteo returns each daily field as its own list, one value per day.
/// Values can be null when the model has no data for that day.
#[derive(Debug, Deserialize)]
struct DailyWeather {
    time: Vec<NaiveDate>,
    weather_code: Vec<Option<f64>>,
    temperature_2m_max: Vec<Option<f64>>,
    temperature_2m_min: Vec<Option<f64>>,
    precipitation_probability_max: Vec<Option<f64>>,
}

#[derive(Debug, Deserialize)]
struct CurrentWeather {
    time: String,
    temperature_2m: f64,
    #[serde(deserialize_with = "deserialize_i32_from_number")]
    is_day: i32,
    precipitation: f64,
    #[serde(deserialize_with = "deserialize_i32_from_number")]
    weather_code: i32,
    wind_speed_10m: f64,
    wind_direction_10m: f64,
}

fn deserialize_i32_from_number<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Number {
        Integer(i32),
        Float(f64),
    }

    match Number::deserialize(deserializer)? {
        Number::Integer(value) => Ok(value),
        Number::Float(value) => {
            if !value.is_finite() {
                return Err(de::Error::custom("expected a finite numeric value"));
            }
            Ok(value.round() as i32)
        }
    }
}

impl OpenMeteoProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to create custom HTTP client: {}", e);
                eprintln!("Using default client with standard timeout settings.");
                reqwest::Client::new()
            });

        Self {
            client,
            base_url: OPEN_METEO_BASE_URL.to_string(),
            forecast_days: 0,
        }
    }

    /// Also requests a daily forecast for this many days, starting with today.
    pub fn with_forecast_days(mut self, days: u8) -> Self {
        self.forecast_days = days.min(MAX_FORECAST_DAYS);
        self
    }

    fn temperature_unit_param(unit: &TemperatureUnit) -> &'static str {
        match unit {
            TemperatureUnit::Celsius => "celsius",
            TemperatureUnit::Fahrenheit => "fahrenheit",
        }
    }

    fn wind_speed_unit_param(unit: &WindSpeedUnit) -> &'static str {
        match unit {
            WindSpeedUnit::Kmh => "kmh",
            WindSpeedUnit::Ms => "ms",
            WindSpeedUnit::Mph => "mph",
            WindSpeedUnit::Kn => "kn",
        }
    }

    fn precipitation_unit_param(unit: &PrecipitationUnit) -> &'static str {
        match unit {
            PrecipitationUnit::Mm => "mm",
            PrecipitationUnit::Inch => "inch",
        }
    }

    fn build_url(&self, location: &WeatherLocation, units: &WeatherUnits) -> String {
        let mut url = format!(
            "{}?latitude={}&longitude={}&current=temperature_2m,is_day,precipitation,weather_code,wind_speed_10m,wind_direction_10m&temperature_unit={}&wind_speed_unit={}&precipitation_unit={}&timezone=auto",
            self.base_url,
            location.latitude,
            location.longitude,
            Self::temperature_unit_param(&units.temperature),
            Self::wind_speed_unit_param(&units.wind_speed),
            Self::precipitation_unit_param(&units.precipitation)
        );
        if self.forecast_days > 0 {
            url.push_str(&format!(
                "&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max&forecast_days={}",
                self.forecast_days
            ));
        }
        url
    }

    /// Turns Open-Meteo's per-field lists into one entry per day, skipping
    /// days without a weather code or temperatures.
    fn parse_daily(daily: DailyWeather, units: &WeatherUnits) -> Vec<DailyForecastResponse> {
        daily
            .time
            .iter()
            .enumerate()
            .filter_map(|(i, date)| {
                let weather_code = (*daily.weather_code.get(i)?)?;
                let max = (*daily.temperature_2m_max.get(i)?)?;
                let min = (*daily.temperature_2m_min.get(i)?)?;
                let probability = daily
                    .precipitation_probability_max
                    .get(i)
                    .copied()
                    .flatten();
                Some(DailyForecastResponse {
                    date: *date,
                    weather_code: weather_code.round() as i32,
                    temperature_max: normalize_temperature(max, units.temperature),
                    temperature_min: normalize_temperature(min, units.temperature),
                    precipitation_probability: probability
                        .map(|p| p.round().clamp(0.0, 100.0) as u8),
                })
            })
            .collect()
    }
}

impl Default for OpenMeteoProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WeatherProvider for OpenMeteoProvider {
    fn get_attribution(&self) -> &'static str {
        ""
    }

    async fn get_current_weather(
        &self,
        location: &WeatherLocation,
        units: &WeatherUnits,
    ) -> Result<WeatherProviderResponse, WeatherError> {
        let url = self.build_url(location, units);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .and_then(|resp| resp.error_for_status())
            .map_err(|e| WeatherError::Network(NetworkError::from_reqwest(e, &url, 30)))?;

        let data: OpenMeteoResponse = response
            .json()
            .await
            .map_err(|e| WeatherError::Network(NetworkError::from_reqwest(e, &url, 30)))?;

        let moon_phase = Some(0.5);

        Ok(WeatherProviderResponse {
            weather_code: data.current.weather_code,
            temperature: normalize_temperature(data.current.temperature_2m, units.temperature),
            precipitation: normalize_precipitation(data.current.precipitation, units.precipitation),
            wind_speed: normalize_wind_speed(data.current.wind_speed_10m, units.wind_speed),
            wind_direction: data.current.wind_direction_10m,
            sun: CelestialEvents::only_day(data.current.is_day),
            moon_phase,
            timestamp: data.current.time,
            attribution: self.get_attribution().to_string(),
            forecast: data
                .daily
                .map(|daily| Self::parse_daily(daily, units))
                .unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_conversion_params() {
        assert_eq!(
            OpenMeteoProvider::temperature_unit_param(&TemperatureUnit::Celsius),
            "celsius"
        );
        assert_eq!(
            OpenMeteoProvider::temperature_unit_param(&TemperatureUnit::Fahrenheit),
            "fahrenheit"
        );
        assert_eq!(
            OpenMeteoProvider::wind_speed_unit_param(&WindSpeedUnit::Kmh),
            "kmh"
        );
        assert_eq!(
            OpenMeteoProvider::wind_speed_unit_param(&WindSpeedUnit::Ms),
            "ms"
        );
        assert_eq!(
            OpenMeteoProvider::precipitation_unit_param(&PrecipitationUnit::Mm),
            "mm"
        );
    }

    #[test]
    fn test_url_without_forecast() {
        let provider = OpenMeteoProvider::new();
        let location = WeatherLocation {
            latitude: 33.6,
            longitude: -117.6,
            elevation: None,
        };
        let url = provider.build_url(&location, &WeatherUnits::imperial());
        assert!(!url.contains("daily="));
    }

    #[test]
    fn test_url_with_forecast_is_capped() {
        let provider = OpenMeteoProvider::new().with_forecast_days(30);
        let location = WeatherLocation {
            latitude: 33.6,
            longitude: -117.6,
            elevation: None,
        };
        let url = provider.build_url(&location, &WeatherUnits::imperial());
        assert!(url.contains("&daily=weather_code,temperature_2m_max"));
        assert!(url.ends_with("&forecast_days=16"));
    }

    #[test]
    fn test_parse_daily() {
        let json = r#"{
            "time": ["2026-10-11", "2026-10-12", "2026-10-13"],
            "weather_code": [63, 51, null],
            "temperature_2m_max": [68.4, 61.9, 72.1],
            "temperature_2m_min": [60.2, 54.7, 53.6],
            "precipitation_probability_max": [78, null, 19]
        }"#;
        let daily: DailyWeather = serde_json::from_str(json).unwrap();
        let days = OpenMeteoProvider::parse_daily(daily, &WeatherUnits::imperial());

        // The third day has no weather code, so it is skipped.
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].date, NaiveDate::from_ymd_opt(2026, 10, 11).unwrap());
        assert_eq!(days[0].weather_code, 63);
        assert_eq!(days[0].precipitation_probability, Some(78));
        assert!((days[0].temperature_max - 20.22).abs() < 0.01); // 68.4°F in °C
        assert_eq!(days[1].precipitation_probability, None);
    }
}
