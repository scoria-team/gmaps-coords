# gmaps-coords

Finds coordinates for saved places in CSV and GeoJSON files exported from Google Maps.

Only starred places can be exported with coordinate data from Google Maps as a GeoJSON, but even then the places sometimes lack coordinates. Other lists of saved places are only exported as CSV, without coordinate data. This tool finds coordinates for places in a GeoJSON or CSV file, and outputs the result in a GeoJSON file.

## Usage

Prerequisites: install Rust, clone this repo, and `cd` into it.

Install and run a WebDriver server in another terminal, such as `geckodriver`. The WebDriver server lets the tool visit the Google Maps webpage and retrieve each place's coordinates.

```shell
cargo install geckodriver
geckodriver
```

Then run the CLI tool on your files.

```shell
cargo run -- -i saved_places.json -o saved_places_complete_coordinates.json
cargo run -- -i my_travel_list.csv -o my_travel_list_with_coordinates.json
```

### Parallelism

Multiple instances of the tool can be run at the same time using multiple WebDriver instances. Specify the `-p` argument for `geckodriver` and `gmaps-coords` to a value other than the default `4444`.

```shell
geckodriver -p 4445
cargo run -- -p 4445 -i saved_places.json -o out.json
```

### Help

Pass `--help` to see more options.
