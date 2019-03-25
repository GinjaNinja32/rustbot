extern crate chrono;
extern crate reqwest;
extern crate rusqlite;
extern crate serde_derive;
extern crate shared;

use chrono::NaiveDateTime;
use rusqlite::NO_PARAMS;
use serde_derive::Deserialize;
use shared::prelude::*;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.cmd("weather", Command::new(weather));
    meta
}

fn weather(ctx: &mut Context, args: &str) -> Result<()> {
    let appid: String = ctx
        .bot
        .sql()
        .lock()
        .query_row("SELECT appid FROM mod_weather_config", NO_PARAMS, |row| row.get(0))?;
    let params = [("q", args), ("APPID", appid.as_str())];
    let client = reqwest::Client::new();
    let mut result = client
        .get("https://api.openweathermap.org/data/2.5/weather")
        .query(&params)
        .send()?;

    match result.status().as_u16() {
        200 => (),
        404 => return ctx.reply("could not find location"),
        code => return ctx.reply(&format!("error {}", code)),
    }

    let data: Response = result.json()?;

    let location = format!("{}, {}", data.name, data.sys.country);
    let timestamp = NaiveDateTime::from_timestamp(data.dt, 0).format("%a %e %b %H:%M");
    let weathers: Vec<String> = data.weather.iter().map(|s| s.description.clone()).collect();
    let temp = format!(
        "{:.0} C ({:.0} F)",
        data.main.temp - 273.15,
        ((data.main.temp - 273.15) * 9.0 / 5.0) + 32.0
    );
    let wind = format!(
        "{:.0} mph ({:.0} kph) from the {}",
        data.wind.speed * 2.23694,
        data.wind.speed * 3.6,
        text_for_angle(data.wind.deg)
    );
    let pressure = format!("{:.0} mb", data.main.pressure);
    ctx.reply(&format!(
        "Weather for {}; Last updated {}; Conditions: {}; Temperature: {}; Humidity: {}%; Wind: {}; Pressure: {}",
        location,
        timestamp,
        weathers.join(", "),
        temp,
        data.main.humidity,
        wind,
        pressure
    ))
}

fn text_for_angle(angle: f64) -> String {
    if angle < 0.0 || angle > 360.0 {
        "unknown".to_string()
    } else if angle < 23.0 || angle >= 23.0 + 315.0 {
        "north".to_string()
    } else if angle < 23.0 + 45.0 {
        "northeast".to_string()
    } else if angle < 23.0 + 90.0 {
        "east".to_string()
    } else if angle < 23.0 + 135.0 {
        "southeast".to_string()
    } else if angle < 23.0 + 180.0 {
        "south".to_string()
    } else if angle < 23.0 + 225.0 {
        "southwest".to_string()
    } else if angle < 23.0 + 270.0 {
        "west".to_string()
    } else if angle < 23.0 + 315.0 {
        "northwest".to_string()
    } else {
        "what".to_string()
    }
}

#[derive(Debug, Deserialize)]
struct Coord {
    lat: f64,
    lon: f64,
}

#[derive(Debug, Deserialize)]
struct Clouds {
    all: i64,
}

#[derive(Debug, Deserialize)]
struct Main {
    humidity: i64,
    pressure: f64,
    temp: f64,
    temp_max: f64,
    temp_min: f64,
}

#[derive(Debug, Deserialize)]
struct Sys {
    country: String,
    message: f64,
    sunrise: i64,
    sunset: i64,
}

#[derive(Debug, Deserialize)]
struct Weather {
    description: String,
    icon: String,
    id: i64,
    main: String,
}

#[derive(Debug, Deserialize)]
struct Wind {
    deg: f64,
    gust: Option<f64>,
    speed: f64,
}

#[derive(Debug, Deserialize)]
struct Response {
    base: String,
    clouds: Clouds,
    cod: i64,
    coord: Coord,
    dt: i64,
    id: i64,
    main: Main,
    name: String,
    sys: Sys,
    visibility: Option<i64>,
    weather: Vec<Weather>,
    wind: Wind,
}
