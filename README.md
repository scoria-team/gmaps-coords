# gmaps-coords

Find coordinates for saved places in CSV and GeoJSON files exported from Google Maps, and generate a GeoJSON with with the result.

If you export your saved places from Google Maps, your starred places will be in a GeoJSON format with coordinates for each place. However, all other lists are exported in CSV format without coordinates. And sometimes places still lack coordinates even though they're in the GeoJSON.

This tool looks up the coordinates for each place using the Google Maps URL, converting a CSV into a GeoJSON with the location data, or fixing up missing coordinates in a GeoJSON file.

## Usage

First, [install Rust](https://www.rust-lang.org/tools/install).

Next, install the `gmaps-coords` CLI tool, and a WebDriver server like `geckodriver`. The WebDriver server lets `gmaps-coords` visit the Google Maps webpage and retrieve each place's coordinates.

```shell
cargo install --git https://github.com/scoria-team/gmaps-coords.git
cargo install geckodriver
```

In one terminal, start `geckodriver`.

```shell
geckodriver
```

In a second terminal, run `gmaps-coords` on your files. The tool takes about two seconds to look up each place's coordinates.

```shell
gmaps-coords -i saved_places.json -o saved_places_complete.json
gmaps-coords -i travel_list.csv -o travel_list_coords.json
```

### Parallelism

Multiple instances of the tool can be run at the same time using multiple WebDriver instances. Specify the `-p` argument for `geckodriver` and `gmaps-coords` to a value other than the default `4444`.

```shell
geckodriver -p 4445
gmaps-coords -p 4445 -i saved_places.json -o out.json
```

### More Options

```shell
gmaps-coords --help
```
