mod airport;

use rustbot::spans;

use chrono::NaiveDateTime;
use rustbot::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Module {
    appid: String,
}

#[no_mangle]
pub fn get_meta_conf(meta: &mut dyn Meta, config: toml::Value) -> Result<()> {
    let m: Module = config.try_into()?;
    meta.cmd("weather", Command::new(move |ctx, args| m.weather(ctx, args)));
    Ok(())
}

impl Module {
    fn weather(&self, ctx: &dyn Context, args: &str) -> Result<()> {
        let params = if let Some(coords) = airport::locate(args) {
            vec![("lat", coords.lat), ("lon", coords.lon), ("APPID", self.appid.clone())]
        } else {
            vec![("q", args.to_string()), ("APPID", self.appid.clone())]
        };
        let client = reqwest::blocking::Client::new();
        let result = client
            .get("https://api.openweathermap.org/data/2.5/weather")
            .query(&params)
            .send()?;

        match result.status().as_u16() {
            200 => (),
            404 => return ctx.say("could not find location"),
            code => return ctx.say(&format!("error {}", code)),
        }

        let text = result.text()?;
        let data: Response =
            serde_json::from_str(&text).with_context(|| format!("failed to unmarshal weather: {}", text))?;

        let location = if let Some(country) = data.sys.country {
            format!("{}, {}", data.name, country)
        } else if !data.name.is_empty() {
            data.name
        } else {
            "unknown location".to_string()
        };

        let timestamp =
            NaiveDateTime::from_timestamp_opt(data.dt + data.timezone, 0).map(|t| t.format("%a %e %b %H:%M"));
        let sunrise = NaiveDateTime::from_timestamp_opt(data.sys.sunrise + data.timezone, 0).map(|t| t.format("%H:%M"));
        let sunset = NaiveDateTime::from_timestamp_opt(data.sys.sunset + data.timezone, 0).map(|t| t.format("%H:%M"));

        let weathers: Vec<String> = data.weather.iter().map(|s| s.description.clone()).collect();
        let temp = format!(
            "{:.0} C ({:.0} F)",
            data.main.temp - 273.15,
            ((data.main.temp - 273.15) * 9.0 / 5.0) + 32.0
        );
        let direction = {
            match data.wind.deg {
                None => "".to_string(),
                Some(d) => format!(" from the {}", text_for_angle(d)),
            }
        };

        let wind = format!(
            "{:.0} mph ({:.0} kph){}",
            data.wind.speed * 2.23694,
            data.wind.speed * 3.6,
            direction,
        );
        let pressure = format!("{:.0} mb", data.main.pressure);
        let mut spans = spans! {
            "Weather for ", location,
        };
        if let Some(ts) = timestamp {
            spans.append(&mut spans! {"; Last updated ", ts.to_string()});
        }
        spans.append(&mut spans! {
            "; Conditions: ", weathers.join(", "),
            "; Temperature: ", temp,
            "; Humidity: ", format!("{}%", data.main.humidity),
            "; Wind: ", wind,
            "; Pressure: ", pressure,
        });
        if let Some(sr) = sunrise {
            spans.append(&mut spans! { "; Sunrise: ", sr.to_string() });
        }
        if let Some(ss) = sunset {
            spans.append(&mut spans! { "; Sunset: ", ss.to_string() });
        }
        ctx.reply(Message::Spans(spans))
    }
}

#[allow(clippy::manual_range_contains)]
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
#[allow(unused)]
struct Coord {
    lat: f64,
    lon: f64,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
struct Clouds {
    all: i64,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
struct Main {
    humidity: i64,
    pressure: f64,
    temp: f64,
    temp_max: f64,
    temp_min: f64,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
struct Sys {
    country: Option<String>,
    sunrise: i64,
    sunset: i64,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
struct Weather {
    description: String,
    icon: String,
    id: i64,
    main: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
struct Wind {
    deg: Option<f64>,
    gust: Option<f64>,
    speed: f64,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
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
    timezone: i64,
    visibility: Option<i64>,
    weather: Vec<Weather>,
    wind: Wind,
}
