use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Read, Write};
use std::time::Duration;
use wasi_http_client::Client;

#[derive(Deserialize, Debug)]
struct Input {
    city: String,
    #[serde(default = "default_units")]
    units: String, // "metric" or "imperial"
}

#[derive(Serialize)]
struct Output {
    city: String,
    country: Option<String>,
    temperature: f64,
    temperature_unit: String,
    description: String,
    humidity: u32,
    wind_speed: f64,
}

#[derive(Deserialize, Debug)]
struct OpenWeatherResponse {
    list: Vec<CityWeather>,
}

#[derive(Deserialize, Debug)]
struct CityWeather {
    name: String,
    sys: SystemInfo,
    main: MainWeather,
    weather: Vec<WeatherDescription>,
    wind: Wind,
}

#[derive(Deserialize, Debug)]
struct SystemInfo {
    country: String,
}

#[derive(Deserialize, Debug)]
struct MainWeather {
    temp: f64,
    humidity: u32,
}

#[derive(Deserialize, Debug)]
struct WeatherDescription {
    description: String,
}

#[derive(Deserialize, Debug)]
struct Wind {
    speed: f64,
}

fn default_units() -> String {
    "metric".to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read input from stdin
    let mut input_string = String::new();
    io::stdin().read_to_string(&mut input_string)?;

    // Parse input JSON
    let input: Input = serde_json::from_str(&input_string)?;

    // Get API key from environment variable (provided via encrypted secrets)
    let api_key = env::var("OPENWEATHER_API_KEY")
        .map_err(|_| "OPENWEATHER_API_KEY environment variable not set")?;

    // Encode city name for URL
    let encoded_city = urlencoding::encode(&input.city);

    // Build API URL
    let url = format!(
        "https://api.openweathermap.org/data/2.5/find?q={}&appid={}&units={}",
        encoded_city, api_key, input.units
    );

    // Make HTTP request
    let response = Client::new()
        .get(&url)
        .connect_timeout(Duration::from_secs(10))
        .send()?;

    // Check response status
    let status = response.status();
    if status < 200 || status >= 300 {
        let error_body = response.body().unwrap_or_default();
        let error_text = String::from_utf8_lossy(&error_body);
        return Err(format!("Weather API error ({}): {}", status, error_text).into());
    }

    // Parse response
    let response_body = response.body()?;
    let weather_data: OpenWeatherResponse = serde_json::from_slice(&response_body)?;

    // Extract first city from results
    let city_weather = weather_data
        .list
        .first()
        .ok_or("No weather data found for this city")?;

    let weather_desc = city_weather
        .weather
        .first()
        .map(|w| w.description.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // Determine temperature unit label
    let temp_unit = match input.units.as_str() {
        "imperial" => "F",
        _ => "C",
    };

    // Build output
    let output = Output {
        city: city_weather.name.clone(),
        country: Some(city_weather.sys.country.clone()),
        temperature: city_weather.main.temp,
        temperature_unit: temp_unit.to_string(),
        description: weather_desc,
        humidity: city_weather.main.humidity,
        wind_speed: city_weather.wind.speed,
    };

    // Write JSON output to stdout
    let json_output = serde_json::to_string(&output)?;
    print!("{}", json_output);
    io::stdout().flush()?;

    Ok(())
}
