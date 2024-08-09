use std::{fs, io::Write, path::PathBuf, str::FromStr};

use anyhow::{bail, Result};
use clap::Parser;
use fantoccini::{Client, ClientBuilder};
use geojson::{Feature, FeatureCollection, Geometry, JsonObject, Value};
use regex::Regex;
use serde::Deserialize;
use tokio::time::{sleep, Duration};

// A latitude,longitude regex pattern. E.g. "-25.0,160.0".
// (?:) denotes a non-capturing group. ()? denotes an optional group.
const LATLNGPAT: &str = r"(-?\d+(?:\.\d+)?),(-?\d+(?:\.\d+)?)";

/// Read GeoJSON and CSV files exported from Google Maps and converts them to
/// GeoJSON files with coordinates for each place.
///
/// First run a WebDriver server in another terminal, such as geckodriver:
///
/// `cargo install geckodriver && geckodriver`
#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// Input filename
    ///
    /// If the extension is "csv", it is interpreted as CSV, otherwise it is
    /// interpreted as GeoJSON. Lines with comments should be removed from CSV
    /// files beforehand.
    #[arg(short, long, value_name = "FILE")]
    input: PathBuf,

    /// Output filename, GeoJSON formatted
    #[arg(short, long, value_name = "FILE")]
    output: PathBuf,

    /// (GeoJSON only) Only output features that got updated coordinates
    #[arg(long)]
    only_changed_places: bool,

    /// The port to connect to the WebDriver server. Defaults to 4444.
    #[arg(short, long, value_name = "PORT")]
    port: Option<u16>,

    /// Show the browser as coordinates are looked up
    #[arg(long)]
    noheadless: bool,
}

/// Run the command-line interface
pub async fn run() {
    let cli = Cli::parse();

    let opts = match cli.noheadless {
        false => serde_json::json!({
            "moz:firefoxOptions": {
                "args": ["--headless"]
            }
        })
        .as_object()
        .unwrap()
        .clone(),
        true => serde_json::Map::new(),
    };
    let c = ClientBuilder::native()
        .capabilities(opts)
        .connect(&format!("http://localhost:{}", cli.port.unwrap_or(4444)))
        .await
        .expect("Failed to connect to WebDriver");

    // check that we can write to the output file, without overwriting, before
    // spending lots of time fetching coordinates
    fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&cli.output)
        .expect("Cannot write to output file");

    let features = match cli.input.extension().and_then(|e| e.to_str()) {
        Some("csv") => run_csv(&c, &cli.input).await,
        _ => run_geojson(&c, &cli.input, cli.only_changed_places).await,
    };

    let mut file =
        fs::File::create(&cli.output).expect("Failed to create output file");
    file.write_all(features.to_string().as_bytes())
        .expect("Failed to write to output file");

    c.close().await.expect("Closing WebDriver client");
}

/// Update a GeoJSON with missing coordiante data.
async fn run_geojson(
    c: &Client,
    input_path: &PathBuf,
    only_change_places: bool,
) -> FeatureCollection {
    let mut feature_collection = FeatureCollection::from_str(
        &fs::read_to_string(input_path).expect("Failed to read file"),
    )
    .expect("Failed to parse input as GeoJSON");

    let mut new_features = vec![];
    for mut feature in feature_collection.features.into_iter() {
        if let Some(Geometry {
            value: Value::Point(ref mut coords),
            ..
        }) = feature.geometry
        {
            if let (Some(lng), Some(lat)) = (coords.first(), coords.get(1)) {
                if *lng == 0.0 && *lat == 0.0 {
                    // at null island, missing coordinate data
                    if let Some(url) = feature
                        .properties
                        .as_ref()
                        .and_then(|p| p.get("google_maps_url"))
                        .and_then(|v| v.as_str())
                    {
                        match get_coords_for_url(c, url).await {
                            Ok(new_coords) => {
                                // update coords and move feature to output vec
                                *coords = new_coords;
                                new_features.push(feature);
                                continue;
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to retrieve coordinates for record \
                                    {url} with error {e}. Continuing."
                                );
                            }
                        };
                    }
                }
            }
        }
        if !only_change_places {
            new_features.push(feature);
        }
    }
    feature_collection.features = new_features;
    feature_collection
}

/// Go to the url and get the coordinates of the place, returned as lng, lat.
async fn get_coords_for_url(c: &Client, url: &str) -> Result<Vec<f64>> {
    // if url contains a coordinate query, the map will not be centered, so
    // just get the coordinates from the url
    let pattern = Regex::new(&format!("{}{}", "q=", LATLNGPAT)).unwrap();
    if let Ok(coords) = coords_from_regex(&pattern, url) {
        return Ok(coords);
    }

    // pattern to match in url when it updates with the view center
    let pattern = Regex::new(&format!("{}{}", "@", LATLNGPAT)).unwrap();
    c.goto(url).await?;
    for i in 0..100 {
        // wait up to 10 seconds total
        sleep(Duration::from_millis(100)).await;
        let redirected_url = c.current_url().await?;
        if redirected_url.as_str() != url {
            if let Ok(coords) =
                coords_from_regex(&pattern, redirected_url.as_str())
            {
                println!("Fetched coordinates in {} seconds", i as f64 / 10.0,);
                return Ok(coords);
            }
        }
    }
    bail!("Failed to get coordinates before timeout");
}

/// Parse the coordinates contained in text, according to the given regex.
fn coords_from_regex(pattern: &Regex, text: &str) -> Result<Vec<f64>> {
    if let Some((_, [lat, lng])) =
        pattern.captures_iter(text).map(|c| c.extract()).next()
    {
        let lat = lat.parse::<f64>()?;
        let lng = lng.parse::<f64>()?;
        Ok(vec![lng, lat])
    } else {
        bail!("No coordinates found in text")
    }
}

/// The expected CSV structure.
#[derive(Debug, Deserialize)]
struct Record {
    #[serde(rename = "Title")]
    title: String,
    #[serde(rename = "Note")]
    note: Option<String>,
    #[serde(rename = "URL")]
    url: String,
    #[serde(rename = "Comment")]
    comment: Option<String>,
}

/// Convert a CSV of locations without coordinates to GeoJSON by looking up the
/// locations.
async fn run_csv(c: &Client, input_path: &PathBuf) -> FeatureCollection {
    let mut rdr = csv::ReaderBuilder::new()
        .from_path(input_path)
        .expect("Failed to read CSV file");

    let mut records_and_coords = vec![];
    for result in rdr.deserialize::<Record>() {
        match result {
            Ok(record) => {
                match get_coords_for_url(c, &record.url).await {
                    Ok(coords) => {
                        records_and_coords.push((record, coords));
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to retrieve coordinates for record \
                            {record:?} with error {e}. Continuing."
                        );
                    }
                };
            }
            Err(e) => {
                eprintln!(
                    "Failed to parse CSV record with error {e}. Continuing."
                );
            }
        };
    }

    FeatureCollection {
        features: records_and_coords
            .into_iter()
            .map(record_and_coords_to_feature)
            .collect(),
        bbox: None,
        foreign_members: None,
    }
}

/// Convert tuples of (CSV record, coordinates) to GeoJSON features.
fn record_and_coords_to_feature(
    (record, coords): (Record, Vec<f64>),
) -> Feature {
    let mut properties = JsonObject::new();
    properties.insert("name".into(), record.title.into());
    properties.insert("google_maps_url".into(), record.url.into());
    if let Some(note) = record.note {
        properties.insert("note".into(), note.into());
    }
    if let Some(comment) = record.comment {
        properties.insert("comment".into(), comment.into());
    }
    Feature {
        geometry: Some(Value::Point(coords).into()),
        properties: Some(properties),
        ..Default::default()
    }
}
